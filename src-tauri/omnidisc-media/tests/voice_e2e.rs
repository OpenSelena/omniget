//! Two participants on a live omnidisc-server + LiveKit: A publishes a 440 Hz
//! tone through the real capture pipeline (test-tone hook), B must receive
//! non-silent audio. Runs three times: in the clear, with the SFrame cryptor
//! keyed by a shared room key (what an MLS epoch hands us), and once with
//! mismatched keys, where nothing may be decoded.
//! Needs `OMNIDISC_TEST_URL`, e.g.
//! `OMNIDISC_TEST_URL=https://72-62-136-3.sslip.io cargo test -p omnidisc-media --test voice_e2e -- --ignored --nocapture`.

use futures_util::{SinkExt, StreamExt};
use omnidisc_media::{
    AudioPrefs, ConnectOptions, LiveKitBackend, MediaEngine, RoomKey, VoiceState,
};
use omnidisc_proto::gateway::VoiceServerUpdate;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

/// Refuses to run against anything but a throwaway instance unless told
/// otherwise. These tests create real accounts, and pointing them at a live
/// server litters it — which is exactly what happened to the production
/// instance before this guard existed.
fn allow_target(base: &str) -> bool {
    if std::env::var_os("OMNIDISC_TEST_ALLOW_REMOTE").is_some() {
        return true;
    }
    let local = base.contains("localhost") || base.contains("127.0.0.1") || base.contains("[::1]");
    if !local {
        eprintln!(
            "refusing to run against {base}: it is not a local instance. Set \
             OMNIDISC_TEST_ALLOW_REMOTE=1 if that is really what you want."
        );
    }
    local
}

/// A password nobody can guess from the repository.
///
/// These tests register real accounts on whatever instance `OMNIDISC_TEST_URL`
/// points at, and a constant in a public repo is a working credential for every
/// account they ever left behind. One random value per process keeps the run
/// self-consistent without publishing a key.
fn test_password() -> &'static str {
    static PASSWORD: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PASSWORD.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(
            "od-{}-{:x}-{:x}",
            std::process::id(),
            nanos,
            nanos.rotate_left(29)
        )
    })
}

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!(
        "{}{}{}",
        prefix,
        std::process::id() % 10_000,
        nanos % 100_000
    )
}

async fn register(http: &reqwest::Client, base: &str, name: &str) -> (String, String) {
    let res: Value = http
        .post(format!("{base}/api/auth/register"))
        .json(&json!({ "username": name, "password": test_password() }))
        .send()
        .await
        .expect("register request")
        .json()
        .await
        .expect("register json");
    let token = res["token"].as_str().expect("token").to_string();
    let id = res["user"]["id"].as_str().expect("user id").to_string();
    (token, id)
}

struct Gateway {
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl Gateway {
    async fn connect(base: &str, token: &str) -> Self {
        let ws_url = format!("{}/gateway", base.replacen("http", "ws", 1));
        let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .expect("gateway ws");
        let hello = ws.next().await.expect("hello").expect("hello frame");
        assert!(matches!(hello, Message::Text(_)));
        let identify = json!({ "op": 2, "d": { "token": token, "protocol_version": omnidisc_proto::PROTOCOL_VERSION, "compress": "none", "properties": { "os": "test", "client": "omnidisc-media-test", "version": "0" } } });
        ws.send(Message::Text(identify.to_string().into()))
            .await
            .expect("identify");
        let mut g = Self { ws };
        g.wait_dispatch("READY").await;
        g
    }

    async fn send(&mut self, frame: Value) {
        self.ws
            .send(Message::Text(frame.to_string().into()))
            .await
            .expect("send frame");
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
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if v["op"] == 0 && v["t"] == name {
                return v["d"].clone();
            }
            if v["op"] == 12 {
                panic!("gateway error while waiting for {name}: {v}");
            }
        }
        panic!("timed out waiting for {name}");
    }
}

fn cpu_percent() -> String {
    let pid = std::process::id().to_string();
    std::process::Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "?".into())
}

struct Room {
    guild_id: String,
    channel_id: String,
    gw_a: Gateway,
    gw_b: Gateway,
    vsu_a: VoiceServerUpdate,
    vsu_b: VoiceServerUpdate,
}

async fn two_in_a_voice_channel(base: &str, label: &str) -> Room {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("http");
    let (token_a, _id_a) = register(&http, base, &unique(&format!("{label}a"))).await;
    let (token_b, _id_b) = register(&http, base, &unique(&format!("{label}b"))).await;

    let guild: Value = http
        .post(format!("{base}/api/guilds"))
        .bearer_auth(&token_a)
        .json(&json!({ "name": "Voice E2E" }))
        .send()
        .await
        .expect("create guild")
        .json()
        .await
        .expect("guild json");
    let guild_id = guild["id"].as_str().expect("guild id").to_string();
    let channel: Value = http
        .post(format!("{base}/api/guilds/{guild_id}/channels"))
        .bearer_auth(&token_a)
        .json(&json!({ "name": "voice", "type": 2 }))
        .send()
        .await
        .expect("create channel")
        .json()
        .await
        .expect("channel json");
    let channel_id = channel["id"].as_str().expect("channel id").to_string();
    let invite: Value = http
        .post(format!("{base}/api/invites"))
        .bearer_auth(&token_a)
        .json(&json!({ "guild_id": guild_id }))
        .send()
        .await
        .expect("invite")
        .json()
        .await
        .expect("invite json");
    let code = invite["code"].as_str().expect("code");
    let joined = http
        .post(format!("{base}/api/invites/{code}"))
        .bearer_auth(&token_b)
        .json(&json!({}))
        .send()
        .await
        .expect("join invite");
    assert!(
        joined.status().is_success(),
        "invite join failed: {}",
        joined.status()
    );

    let mut gw_a = Gateway::connect(base, &token_a).await;
    let mut gw_b = Gateway::connect(base, &token_b).await;
    gw_a.send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": channel_id, "self_mute": false, "self_deaf": false } })).await;
    let vsu_a: VoiceServerUpdate =
        serde_json::from_value(gw_a.wait_dispatch("VOICE_SERVER_UPDATE").await).expect("vsu a");
    gw_b.send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": channel_id, "self_mute": false, "self_deaf": false } })).await;
    let vsu_b: VoiceServerUpdate =
        serde_json::from_value(gw_b.wait_dispatch("VOICE_SERVER_UPDATE").await).expect("vsu b");
    assert_eq!(vsu_a.room, vsu_b.room);
    eprintln!(
        "[e2e/{label}] room {} endpoint {} ice_servers={}",
        vsu_a.room,
        vsu_a.endpoint,
        vsu_a.ice_servers.len()
    );
    Room {
        guild_id,
        channel_id,
        gw_a,
        gw_b,
        vsu_a,
        vsu_b,
    }
}

/// A publishes a tone, B must hear it. `room_key` present means both sides run
/// the SFrame cryptor in shared-key mode, exactly as an MLS epoch would set it.
async fn tone_reaches_the_other_side(base: &str, label: &str, room_key: Option<RoomKey>) {
    let mut room = two_in_a_voice_channel(base, label).await;

    let backend_a = Arc::new(LiveKitBackend::new().expect("backend a"));
    let backend_b = Arc::new(LiveKitBackend::new().expect("backend b"));
    let engine_a = Arc::new(MediaEngine::new(backend_a.clone()));
    let engine_b = Arc::new(MediaEngine::new(backend_b.clone()));
    let pump_a = {
        let e = engine_a.clone();
        tokio::spawn(async move { e.pump().await })
    };
    let pump_b = {
        let e = engine_b.clone();
        tokio::spawn(async move { e.pump().await })
    };
    let mut notes_b = engine_b.subscribe();

    let prefs = AudioPrefs {
        noise_suppression: false,
        ..Default::default()
    };
    let options = ConnectOptions {
        room_key,
        relay_only: false,
    };
    let outcome_a = engine_a
        .join(&room.vsu_a, &prefs, &options)
        .await
        .expect("A joins");
    eprintln!(
        "[e2e/{label}] A connected (mic_error={:?}, output_error={:?})",
        outcome_a.mic_error, outcome_a.output_error
    );
    assert_eq!(engine_a.state().await, VoiceState::Connected);
    backend_a.set_test_tone(Some(440.0));
    let outcome_b = engine_b
        .join(&room.vsu_b, &prefs, &options)
        .await
        .expect("B joins");
    eprintln!(
        "[e2e/{label}] B connected (mic_error={:?}, output_error={:?})",
        outcome_b.mic_error, outcome_b.output_error
    );
    backend_b.set_test_tone(Some(440.0));

    assert_eq!(
        engine_a.e2ee_epoch(),
        room_key.map(|k| k.epoch),
        "A's key ring state"
    );
    assert_eq!(
        engine_b.e2ee_epoch(),
        room_key.map(|k| k.epoch),
        "B's key ring state"
    );

    let start = Instant::now();
    let deadline = start + Duration::from_secs(20);
    let mut frames = 0;
    while Instant::now() < deadline {
        frames = backend_b.remote_nonsilent_frames();
        if frames >= 50 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let tone = backend_b.remote_tone_ratio();
    eprintln!(
        "[e2e/{label}] B received {frames} non-silent frames after {:?}, {:.0}% of the energy at 440 Hz",
        start.elapsed(),
        tone * 100.0
    );
    assert!(
        frames >= 50,
        "B received only {frames} non-silent audio frames within 20 s"
    );
    // Frame count alone would also pass on the comfort noise Opus invents for
    // frames it could not decrypt; the tone is what proves A's audio arrived.
    assert!(
        tone > 0.5,
        "only {:.0}% of B's received energy was A's 440 Hz tone",
        tone * 100.0
    );

    let mut saw_speaking = false;
    let mut saw_participant = false;
    while let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(50), notes_b.recv()).await {
        match n {
            omnidisc_media::EngineNotification::Speaking { speaking: true, .. } => {
                saw_speaking = true
            }
            omnidisc_media::EngineNotification::ParticipantJoined { .. } => saw_participant = true,
            _ => {}
        }
    }
    eprintln!(
        "[e2e/{label}] B events: speaking={saw_speaking} participant_joined={saw_participant}"
    );

    if let Some(key) = room_key {
        // A commit on the group bumps the epoch; the ring index has to follow it
        // without dropping the call.
        let next = RoomKey::new(key.epoch + 1, [0x5A; 32]);
        engine_a.set_room_key(next).await.expect("A rotates");
        engine_b.set_room_key(next).await.expect("B rotates");
        assert_eq!(engine_a.e2ee_epoch(), Some(next.epoch));
        let before = backend_b.remote_nonsilent_frames();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let after = backend_b.remote_nonsilent_frames();
        eprintln!("[e2e/{label}] frames across the epoch bump: {before} -> {after}");
        assert!(
            after > before,
            "audio stopped after the key rotated ({before} -> {after})"
        );
    }

    let _ = engine_b.stats().await.expect("stats b (first sample)");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let stats_b = engine_b.stats().await.expect("stats b");
    eprintln!("[e2e/{label}] B stats: {:?}", stats_b);
    eprintln!(
        "[e2e/{label}] process cpu (ps lifetime avg) = {}%",
        cpu_percent()
    );
    assert!(
        stats_b.bitrate_in_kbps > 0.0,
        "B reports no inbound bitrate"
    );

    engine_a.leave().await.expect("A leaves");
    engine_b.leave().await.expect("B leaves");
    assert_eq!(engine_b.state().await, VoiceState::Idle);
    let guild_id = room.guild_id.clone();
    room.gw_a
        .send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": null } }))
        .await;
    room.gw_b
        .send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": null } }))
        .await;
    let _ = room.channel_id;
    pump_a.abort();
    pump_b.abort();
}

fn test_url() -> Option<String> {
    let picked = real_test_url();
    return picked.filter(|b| allow_target(b));
}

#[allow(dead_code)]
fn real_test_url() -> Option<String> {
    match std::env::var("OMNIDISC_TEST_URL") {
        Ok(url) => Some(url.trim().trim_end_matches('/').to_string()),
        Err(_) => {
            eprintln!("OMNIDISC_TEST_URL not set; skipping");
            None
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn voice_two_participants_exchange_audio() {
    let Some(base) = test_url() else { return };
    tone_reaches_the_other_side(&base, "clear", None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn voice_two_participants_exchange_encrypted_audio() {
    let Some(base) = test_url() else { return };
    tone_reaches_the_other_side(&base, "e2ee", Some(RoomKey::new(3, [0xA7; 32]))).await;
}

/// The proof that the cryptor is really in the path: same room, different keys,
/// no audio. Without this the encrypted test above would still pass if E2EE
/// were silently ignored.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn voice_with_the_wrong_key_hears_nothing() {
    let Some(base) = test_url() else { return };
    let mut room = two_in_a_voice_channel(&base, "badkey").await;

    let backend_a = Arc::new(LiveKitBackend::new().expect("backend a"));
    let backend_b = Arc::new(LiveKitBackend::new().expect("backend b"));
    let engine_a = Arc::new(MediaEngine::new(backend_a.clone()));
    let engine_b = Arc::new(MediaEngine::new(backend_b.clone()));
    let pump_a = {
        let e = engine_a.clone();
        tokio::spawn(async move { e.pump().await })
    };
    let pump_b = {
        let e = engine_b.clone();
        tokio::spawn(async move { e.pump().await })
    };

    let prefs = AudioPrefs {
        noise_suppression: false,
        ..Default::default()
    };
    engine_a
        .join(
            &room.vsu_a,
            &prefs,
            &ConnectOptions {
                room_key: Some(RoomKey::new(1, [0x11; 32])),
                relay_only: false,
            },
        )
        .await
        .expect("A joins");
    backend_a.set_test_tone(Some(440.0));
    engine_b
        .join(
            &room.vsu_b,
            &prefs,
            &ConnectOptions {
                room_key: Some(RoomKey::new(1, [0x22; 32])),
                relay_only: false,
            },
        )
        .await
        .expect("B joins");

    tokio::time::sleep(Duration::from_secs(12)).await;
    let frames = backend_b.remote_nonsilent_frames();
    let tone = backend_b.remote_tone_ratio();
    eprintln!(
        "[e2e/badkey] B received {frames} non-silent frames with the wrong key, {:.1}% of the energy at 440 Hz",
        tone * 100.0
    );
    // A handful of non-silent frames is Opus concealment noise for packets the
    // cryptor rejected, not audio: what must be absent is the tone itself.
    assert!(
        tone < 0.1,
        "B recovered A's tone with the wrong key ({:.0}% of the energy at 440 Hz)",
        tone * 100.0
    );
    assert!(
        frames < 200,
        "B decoded {frames} frames it should not have been able to decrypt"
    );

    engine_a.leave().await.expect("A leaves");
    engine_b.leave().await.expect("B leaves");
    let guild_id = room.guild_id.clone();
    room.gw_a
        .send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": null } }))
        .await;
    room.gw_b
        .send(json!({ "op": 4, "d": { "guild_id": guild_id, "channel_id": null } }))
        .await;
    pump_a.abort();
    pump_b.abort();
}
