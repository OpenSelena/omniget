//! Live screen-share e2e against a real omnidisc-server + LiveKit. A publishes a
//! synthetic NV12 1080p60 stream through the real encoder path (generator -> our
//! VideoToolbox session -> PreEncoded pass-through); B subscribes to the remote
//! screenshare video and must receive >= 30 fps within 60 s with a decoder
//! reported. Needs `OMNIDISC_TEST_URL`, e.g.
//! `OMNIDISC_TEST_URL=http://72.62.136.3:8080 cargo test -p omnidisc-media --test stream_e2e -- --ignored --nocapture`.

use futures_util::{SinkExt, StreamExt};
use omnidisc_media::{start_stream, AudioMode, AudioPrefs, ConnectOptions, LiveKitBackend, MediaEngine, SourceId, StreamMode, StreamRequest, VoiceState};
use omnidisc_proto::gateway::VoiceServerUpdate;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{}{}{}", prefix, std::process::id() % 10_000, nanos % 100_000)
}

async fn register(http: &reqwest::Client, base: &str, name: &str) -> (String, String) {
    let res: Value = http
        .post(format!("{base}/api/auth/register"))
        .json(&json!({ "username": name, "password": "correct horse battery" }))
        .send()
        .await
        .expect("register request")
        .json()
        .await
        .expect("register json");
    (res["token"].as_str().expect("token").to_string(), res["user"]["id"].as_str().expect("user id").to_string())
}

struct Gateway {
    ws: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
}

impl Gateway {
    async fn connect(base: &str, token: &str) -> Self {
        let ws_url = format!("{}/gateway", base.replacen("http", "ws", 1));
        let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url).await.expect("gateway ws");
        let _hello = ws.next().await.expect("hello").expect("hello frame");
        let identify = json!({ "op": 2, "d": { "token": token, "protocol_version": omnidisc_proto::PROTOCOL_VERSION, "compress": "none", "properties": { "os": "test", "client": "omnidisc-media-test", "version": "0" } } });
        ws.send(Message::Text(identify.to_string().into())).await.expect("identify");
        let mut g = Self { ws };
        g.wait_dispatch("READY").await;
        g
    }

    async fn send(&mut self, frame: Value) {
        self.ws.send(Message::Text(frame.to_string().into())).await.expect("send frame");
    }

    async fn try_dispatch(&mut self, name: &str, budget: Duration) -> Option<Value> {
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Ok(Some(Ok(msg))) = tokio::time::timeout(remaining, self.ws.next()).await else { return None };
            let Message::Text(text) = msg else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
            if v["op"] == 0 && v["t"] == name {
                return Some(v["d"].clone());
            }
        }
        None
    }

    async fn wait_dispatch(&mut self, name: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let msg = tokio::time::timeout(remaining, self.ws.next())
                .await
                .unwrap_or_else(|_| panic!("timed out waiting for {name}"))
                .expect("gateway closed")
                .expect("gateway frame");
            let Message::Text(text) = msg else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
            if v["op"] == 0 && v["t"] == name {
                return v["d"].clone();
            }
            if v["op"] == 12 {
                panic!("gateway error waiting for {name}: {v}");
            }
        }
        panic!("timed out waiting for {name}");
    }
}

fn cpu_percent() -> String {
    let pid = std::process::id().to_string();
    std::process::Command::new("ps").args(["-o", "%cpu=", "-p", &pid]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|_| "?".into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn stream_publish_and_watch() {
    let Ok(url) = std::env::var("OMNIDISC_TEST_URL") else {
        eprintln!("OMNIDISC_TEST_URL not set; skipping");
        return;
    };
    let base = url.trim().trim_end_matches('/').to_string();
    let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().expect("http");

    let (token_a, id_a) = register(&http, &base, &unique("streama")).await;
    let (token_b, _id_b) = register(&http, &base, &unique("streamb")).await;

    let guild: Value = http.post(format!("{base}/api/guilds")).bearer_auth(&token_a).json(&json!({ "name": "Stream E2E" })).send().await.expect("guild").json().await.expect("guild json");
    let guild_id = guild["id"].as_str().expect("guild id").to_string();
    let channel: Value = http.post(format!("{base}/api/guilds/{guild_id}/channels")).bearer_auth(&token_a).json(&json!({ "name": "stage", "type": 2 })).send().await.expect("channel").json().await.expect("channel json");
    let channel_id = channel["id"].as_str().expect("channel id").to_string();
    let invite: Value = http.post(format!("{base}/api/invites")).bearer_auth(&token_a).json(&json!({ "guild_id": guild_id })).send().await.expect("invite").json().await.expect("invite json");
    let code = invite["code"].as_str().expect("code");
    let joined = http.post(format!("{base}/api/invites/{code}")).bearer_auth(&token_b).json(&json!({})).send().await.expect("join");
    assert!(joined.status().is_success(), "invite join: {}", joined.status());

    let mut gw_a = Gateway::connect(&base, &token_a).await;
    let mut gw_b = Gateway::connect(&base, &token_b).await;
    gw_a.send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": channel_id, "self_mute": false, "self_deaf": false, "self_stream": true } })).await;
    let vsu_a: VoiceServerUpdate = serde_json::from_value(gw_a.wait_dispatch("VOICE_SERVER_UPDATE").await).expect("vsu a");
    gw_b.send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": channel_id, "self_mute": false, "self_deaf": false } })).await;
    let vsu_b: VoiceServerUpdate = serde_json::from_value(gw_b.wait_dispatch("VOICE_SERVER_UPDATE").await).expect("vsu b");
    assert_eq!(vsu_a.room, vsu_b.room);

    // A streaming=true propagation is best-effort: only servers built with the
    // self_stream change fan it out; the deployed VPS may predate it.
    if let Some(vs) = gw_b.try_dispatch("VOICE_STATE_UPDATE", Duration::from_secs(3)).await {
        eprintln!("[e2e] B saw voice state: streaming={:?} user={:?}", vs["streaming"], vs["user_id"]);
    } else {
        eprintln!("[e2e] no VOICE_STATE_UPDATE seen (older server or timing) — continuing");
    }

    let backend_a = Arc::new(LiveKitBackend::new().expect("backend a"));
    let backend_b = Arc::new(LiveKitBackend::new().expect("backend b"));
    let engine_a = Arc::new(MediaEngine::new(backend_a.clone()));
    let engine_b = Arc::new(MediaEngine::new(backend_b.clone()));
    let pa = { let e = engine_a.clone(); tokio::spawn(async move { e.pump().await }) };
    let pb = { let e = engine_b.clone(); tokio::spawn(async move { e.pump().await }) };

    let prefs = AudioPrefs { noise_suppression: false, ..Default::default() };
    engine_a.join(&vsu_a, &prefs, &ConnectOptions::default()).await.expect("A joins");
    assert_eq!(engine_a.state().await, VoiceState::Connected);
    engine_b.join(&vsu_b, &prefs, &ConnectOptions::default()).await.expect("B joins");

    let req = StreamRequest {
        source: SourceId::Synthetic { width: 1920, height: 1080 },
        fps: 60,
        height: Some(1080),
        audio: AudioMode::None,
        bitrate_kbps: None,
        mode: StreamMode::Text,
        cursor: false,
        policy: Default::default(),
    };
    let stream = start_stream(backend_a.clone(), req).await.expect("start stream");
    eprintln!("[e2e] A publishing {}x{}@{} {:?} {} kbps", stream.resolved.width, stream.resolved.height, stream.resolved.fps, stream.resolved.codec, stream.resolved.bitrate_kbps);

    // B subscribes to A's screenshare video and counts decoded frames.
    let start = Instant::now();
    let track = {
        let mut found = None;
        while start.elapsed() < Duration::from_secs(20) {
            if let Some(p) = backend_b.video_publication_for(&id_a) {
                p.set_subscribed(true);
            }
            if let Some(rv) = backend_b.remote_video_for(&id_a) {
                found = Some(rv.track);
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        found.expect("B never received A's video track")
    };

    let frames = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let frames2 = frames.clone();
    let rtc = track.rtc_track();
    let pump = tokio::spawn(async move {
        let mut stream = livekit::webrtc::video_stream::native::NativeVideoStream::new(rtc);
        while let Some(_f) = stream.next().await {
            frames2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });

    let mut fps = 0.0;
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = (Instant::now(), 0u64);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let n = frames.load(std::sync::atomic::Ordering::Relaxed);
        let dt = last.0.elapsed().as_secs_f64();
        fps = (n.saturating_sub(last.1)) as f64 / dt;
        last = (Instant::now(), n);
        let mut decoder = String::new();
        if let Ok(st) = track.get_stats().await {
            for s in &st {
                if let livekit::webrtc::stats::RtcStats::InboundRtp(i) = s {
                    if i.stream.kind == "video" {
                        decoder = i.inbound.decoder_implementation.clone();
                        eprintln!("[e2e] B recv fps={:.1} {}x{} decoder='{}' key={} dropped={}", i.inbound.frames_per_second, i.inbound.frame_width, i.inbound.frame_height, decoder, i.inbound.key_frames_decoded, i.inbound.frames_dropped);
                    }
                }
            }
        }
        if fps >= 30.0 && !decoder.is_empty() {
            eprintln!("[e2e] reached {:.1} fps with decoder '{}' at t+{:.0}s", fps, decoder, start.elapsed().as_secs_f64());
            break;
        }
    }

    let mut decoder = String::new();
    if let Ok(st) = track.get_stats().await {
        for s in &st {
            if let livekit::webrtc::stats::RtcStats::InboundRtp(i) = s {
                if i.stream.kind == "video" {
                    decoder = i.inbound.decoder_implementation.clone();
                }
            }
        }
    }
    eprintln!("[e2e] final fps={:.1} decoder='{}' cpu(ps lifetime)={}%", fps, decoder, cpu_percent());

    let room_a = backend_a.current_room().await.expect("room a");
    stream.stop(&room_a).await;
    engine_a.leave().await.ok();
    engine_b.leave().await.ok();
    pump.abort();
    pa.abort();
    pb.abort();

    assert!(fps >= 30.0, "B received only {fps:.1} fps within 60 s");
    assert!(!decoder.is_empty(), "no decoder implementation reported for B");
}
