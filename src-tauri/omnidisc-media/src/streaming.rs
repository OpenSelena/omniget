use crate::capture::{self, AudioSink, CaptureOptions, CapturedFrame, VideoSink, VideoTick};
use crate::encode::{self, EncoderConfig, EncoderCounters, PublishPath, VideoEncoder};
use crate::livekit_backend::LiveKitBackend;
use crate::stream::{
    resolve_bitrate, AudioMode, PublishStats, ResolvedStream, StreamCodec, StreamError, StreamMode,
    StreamRequest,
};
use livekit::options::{TrackPublishOptions, VideoCodec, VideoEncoderBackend, VideoEncoding};
use livekit::prelude::*;
use livekit::webrtc::audio_frame::AudioFrame;
use livekit::webrtc::audio_source::{
    native::NativeAudioSource, AudioSourceOptions, RtcAudioSource,
};
use livekit::webrtc::stats::RtcStats;
use livekit::webrtc::video_frame::{I420Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::{native::NativeVideoSource, RtcVideoSource, VideoResolution};
use omnidisc_proto::bitrate::{Codec, StreamingPolicy};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

const PRIMER_FRAMES: u32 = 6;
const PRIMER_INTERVAL_MS: u64 = 100;
const RATE_FLOOR_PCT: u64 = 25;

fn codec_from_policy(policy: &StreamingPolicy, height: u32, fps: u16) -> StreamCodec {
    match policy.codec_for(height.min(u16::MAX as u32) as u16, fps) {
        Codec::H265 => StreamCodec::H265,
        _ => StreamCodec::H264,
    }
}

fn to_video_codec(c: StreamCodec) -> VideoCodec {
    match c {
        StreamCodec::H264 => VideoCodec::H264,
        StreamCodec::H265 => VideoCodec::H265,
    }
}

struct EncodeLoop {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    latest: Arc<StdMutex<Option<CapturedFrame>>>,
    fps_captured: Arc<AtomicU64>,
}

impl EncodeLoop {
    fn video_sink(&self) -> VideoSink {
        let latest = self.latest.clone();
        let counter = self.fps_captured.clone();
        Arc::new(move |tick: VideoTick| {
            if let VideoTick::Frame(frame) = tick {
                counter.fetch_add(1, Ordering::Relaxed);
                if let Ok(mut slot) = latest.lock() {
                    *slot = Some(frame);
                }
            }
        })
    }

    fn spawn(
        encoder: Arc<VideoEncoder>,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
        fps: u16,
        configured_bps: u64,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let latest: Arc<StdMutex<Option<CapturedFrame>>> = Arc::new(StdMutex::new(None));
        let fps_captured = Arc::new(AtomicU64::new(0));
        let stop2 = stop.clone();
        let latest2 = latest.clone();
        let floor = configured_bps * RATE_FLOOR_PCT / 100;
        let thread = std::thread::Builder::new()
            .name("omnidisc-encode".into())
            .spawn(move || {
                let interval = Duration::from_nanos(1_000_000_000u64 / fps.max(1) as u64);
                let mut next = Instant::now();
                let mut have_first = false;
                let mut last_applied = configured_bps;
                while !stop2.load(Ordering::Relaxed) {
                    let frame = latest2.lock().ok().and_then(|mut s| s.take());
                    let mut force_key = !have_first;
                    if source.take_keyframe_request() {
                        force_key = true;
                        counters.keyframe_requests.fetch_add(1, Ordering::Relaxed);
                    }
                    if let Some(rc) = source.take_rate_control_request() {
                        counters.rate_requests.fetch_add(1, Ordering::Relaxed);
                        if rc.target_bitrate_bps > 0 {
                            let target = rc.target_bitrate_bps.max(floor).min(configured_bps);
                            let changed = (last_applied as f64 - target as f64).abs()
                                / last_applied.max(1) as f64
                                > 0.05;
                            if changed {
                                encoder.set_bitrate(target);
                                last_applied = target;
                            }
                        }
                    }
                    if let Some(frame) = frame {
                        if let Err(e) = encoder.encode(&frame, force_key) {
                            tracing::debug!("[omnidisc-media] encode: {e}");
                        } else {
                            have_first = true;
                        }
                        if let Ok(mut slot) = latest2.lock() {
                            if slot.is_none() {
                                *slot = Some(frame);
                            }
                        }
                    }
                    next += interval;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else if now - next > Duration::from_millis(200) {
                        next = now;
                    }
                }
            })
            .expect("spawn encode loop");
        Self {
            stop,
            thread: Some(thread),
            latest,
            fps_captured,
        }
    }
}

impl Drop for EncodeLoop {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct AudioPublish {
    capture: capture::AudioCapture,
    _track: LocalAudioTrack,
    publication: LocalTrackPublication,
    mode: AudioMode,
}

pub struct ActiveStream {
    pub resolved: ResolvedStream,
    video_track: LocalVideoTrack,
    video_pub: LocalTrackPublication,
    _source: NativeVideoSource,
    encoder: Arc<VideoEncoder>,
    capture: Option<capture::VideoCapture>,
    encode_loop: Option<EncodeLoop>,
    counters: Arc<EncoderCounters>,
    audio: Option<AudioPublish>,
    configured_kbps: u32,
    started: Instant,
    last_sample: StdMutex<Option<(Instant, u64, u64)>>,
}

impl ActiveStream {
    pub async fn stop(mut self, room: &Arc<Room>) {
        self.encoder.finish();
        self.encode_loop.take();
        if let Some(c) = self.capture.take() {
            c.stop();
        }
        let _ = room
            .local_participant()
            .unpublish_track(&self.video_pub.sid())
            .await;
        if let Some(a) = self.audio.take() {
            let _ = room
                .local_participant()
                .unpublish_track(&a.publication.sid())
                .await;
            a.capture.stop();
        }
    }
}

pub async fn start_stream(
    backend: Arc<LiveKitBackend>,
    req: StreamRequest,
) -> Result<ActiveStream, StreamError> {
    let room = backend
        .current_room()
        .await
        .ok_or(StreamError::NotConnected)?;

    let cap_sink_slot: Arc<StdMutex<Option<VideoSink>>> = Arc::new(StdMutex::new(None));
    let slot = cap_sink_slot.clone();
    let vsink: VideoSink = Arc::new(move |tick| {
        if let Some(s) = slot.lock().ok().and_then(|g| g.clone()) {
            s(tick);
        }
    });
    let opts = CaptureOptions {
        source: req.source.clone(),
        fps: req.fps,
        height: req.height,
        cursor: req.cursor,
    };
    let (video_capture, geometry) = capture::start_video(&opts, vsink)?;
    let (width, height) = (geometry.width, geometry.height);

    let codec = encode::clamp_codec(codec_from_policy(&req.policy, height, req.fps));
    let bitrate = resolve_bitrate(
        &req.policy,
        width,
        height,
        req.fps,
        req.height,
        req.bitrate_kbps,
    );
    let configured_bps = bitrate as u64 * 1000;
    let cfg = EncoderConfig {
        width,
        height,
        fps: req.fps,
        codec,
        bitrate_kbps: bitrate,
        mode: req.mode,
    };
    let path = encode::publish_path(&cfg);
    let resolved = ResolvedStream {
        width,
        height,
        fps: req.fps,
        bitrate_kbps: bitrate,
        codec,
        mode: req.mode,
        audio: req.audio,
    };

    let resolution = VideoResolution { width, height };
    let source = match path {
        PublishPath::PreEncoded => NativeVideoSource::new_encoded(resolution),
        PublishPath::Raw => NativeVideoSource::new(resolution, true),
    };
    let track = LocalVideoTrack::create_video_track(
        "omnidisc-screen",
        RtcVideoSource::Native(source.clone()),
    );
    let degradation = Some(match req.mode {
        StreamMode::Text => livekit::options::DegradationPreference::MaintainResolution,
        StreamMode::Game => livekit::options::DegradationPreference::Balanced,
    });
    let publication = room
        .local_participant()
        .publish_track(
            LocalTrack::Video(track.clone()),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                video_codec: to_video_codec(codec),
                simulcast: false,
                video_encoder: match path {
                    PublishPath::PreEncoded => VideoEncoderBackend::PreEncoded,
                    // On Linux the SDK's own VAAPI/NVENC backends replace the
                    // encoder we write by hand elsewhere, so ask for one by
                    // name instead of leaving the choice to `Auto`.
                    #[cfg(all(target_os = "linux", feature = "linux-capture"))]
                    PublishPath::Raw => encode::preferred_backend(),
                    #[cfg(not(all(target_os = "linux", feature = "linux-capture")))]
                    PublishPath::Raw => VideoEncoderBackend::Auto,
                },
                video_encoding: Some(VideoEncoding {
                    max_bitrate: configured_bps,
                    max_framerate: req.fps as f64,
                }),
                degradation_preference: degradation,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| StreamError::Encoder(format!("publish screenshare: {e}")))?;
    let video_pub = publication;

    // S-07 §5.1: prime the sender with raw frames so the pass-through selector
    // switches before the first encoded frame (otherwise it segfaults on macOS).
    if path == PublishPath::PreEncoded {
        for _ in 0..PRIMER_FRAMES {
            let black = VideoFrame {
                rotation: VideoRotation::VideoRotation0,
                timestamp_us: 0,
                frame_metadata: None,
                buffer: I420Buffer::new_black(width, height),
            };
            source.capture_frame(&black);
            tokio::time::sleep(Duration::from_millis(PRIMER_INTERVAL_MS)).await;
        }
        tokio::time::sleep(Duration::from_millis(PRIMER_INTERVAL_MS * 2)).await;
    }

    let counters = Arc::new(EncoderCounters::default());
    let encoder = Arc::new(VideoEncoder::new(cfg, source.clone(), counters.clone())?);
    let encode_loop = EncodeLoop::spawn(
        encoder.clone(),
        source.clone(),
        counters.clone(),
        req.fps,
        configured_bps,
    );
    if let Ok(mut g) = cap_sink_slot.lock() {
        *g = Some(encode_loop.video_sink());
    }

    let audio = start_screenshare_audio(&room, req.audio).await;
    // Report the mode actually obtained, not the one asked for: the capture
    // side degrades per-app -> system-except-us -> none on its own.
    let mut resolved = resolved;
    resolved.audio = audio.as_ref().map(|a| a.mode).unwrap_or(AudioMode::None);

    Ok(ActiveStream {
        resolved,
        video_track: track,
        video_pub,
        _source: source,
        encoder,
        capture: Some(video_capture),
        encode_loop: Some(encode_loop),
        counters,
        audio,
        configured_kbps: bitrate,
        started: Instant::now(),
        last_sample: StdMutex::new(None),
    })
}

async fn start_screenshare_audio(room: &Arc<Room>, mode: AudioMode) -> Option<AudioPublish> {
    if mode == AudioMode::None {
        return None;
    }
    let source = NativeAudioSource::new(
        AudioSourceOptions {
            echo_cancellation: false,
            noise_suppression: false,
            auto_gain_control: false,
        },
        capture::AUDIO_SAMPLE_RATE,
        capture::AUDIO_CHANNELS,
        0,
    );
    let src2 = source.clone();
    // SCK delivers 20 ms interleaved-stereo buffers; split into 10 ms frames.
    let asink: AudioSink = Arc::new(move |samples: &[f32]| {
        let per_frame = (capture::AUDIO_SAMPLE_RATE / 100 * capture::AUDIO_CHANNELS) as usize;
        for chunk in samples.chunks(per_frame) {
            if chunk.len() < per_frame {
                break;
            }
            let mut frame = AudioFrame::new(
                capture::AUDIO_SAMPLE_RATE,
                capture::AUDIO_CHANNELS,
                (chunk.len() / capture::AUDIO_CHANNELS as usize) as u32,
            );
            {
                let data = frame.data.to_mut();
                for (i, s) in chunk.iter().enumerate() {
                    data[i] = (*s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
            let _ = futures::executor::block_on(src2.capture_frame(&frame));
        }
    });
    let (capture_handle, obtained) = match capture::start_audio(mode, asink) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[omnidisc-media] screenshare audio unavailable: {e}");
            return None;
        }
    };
    if obtained == AudioMode::None {
        return None;
    }
    let track = LocalAudioTrack::create_audio_track(
        "omnidisc-screen-audio",
        RtcAudioSource::Native(source),
    );
    match room
        .local_participant()
        .publish_track(
            LocalTrack::Audio(track.clone()),
            TrackPublishOptions {
                source: TrackSource::ScreenshareAudio,
                ..Default::default()
            },
        )
        .await
    {
        Ok(pub_) => Some(AudioPublish {
            capture: capture_handle,
            _track: track,
            publication: pub_,
            mode: obtained,
        }),
        Err(e) => {
            tracing::warn!("[omnidisc-media] publish screenshare audio: {e}");
            capture_handle.stop();
            None
        }
    }
}

impl ActiveStream {
    pub fn audio_mode(&self) -> AudioMode {
        self.resolved.audio
    }

    pub fn overdrive(&self) {
        self.encoder.set_bitrate(self.configured_kbps as u64 * 1000);
        self.encoder.set_framerate(self.resolved.fps);
    }

    pub async fn stats(&self) -> PublishStats {
        let mut stats = PublishStats {
            width: self.resolved.width,
            height: self.resolved.height,
            configured_kbps: self.configured_kbps,
            codec: Some(self.resolved.codec),
            encoder: self.encoder.name().to_string(),
            hardware: self.encoder.hardware(),
            audio: self.resolved.audio,
            ..Default::default()
        };
        let encoded = self.counters.encoded.load(Ordering::Relaxed);
        let bytes = self.counters.bytes.load(Ordering::Relaxed);
        let lat_sum = self.counters.latency_ns_sum.load(Ordering::Relaxed);
        let now = Instant::now();
        if let Ok(mut last) = self.last_sample.lock() {
            if let Some((t, e, b)) = *last {
                let dt = now.duration_since(t).as_secs_f64();
                if dt > 0.2 {
                    stats.fps_encoded = (encoded.saturating_sub(e)) as f64 / dt;
                    stats.bitrate_kbps = (bytes.saturating_sub(b)) as f64 * 8.0 / 1000.0 / dt;
                }
            }
            *last = Some((now, encoded, bytes));
        }
        if encoded > 0 {
            stats.encode_ms = lat_sum as f64 / encoded as f64 / 1e6;
        }
        stats.keyframes = self.counters.keyframes.load(Ordering::Relaxed);
        stats.keyframe_requests = self.counters.keyframe_requests.load(Ordering::Relaxed);
        stats.frames_dropped = self.counters.dropped.load(Ordering::Relaxed);
        if let Some(el) = self.encode_loop.as_ref() {
            stats.fps_captured = el.fps_captured.load(Ordering::Relaxed) as f64
                / self.started.elapsed().as_secs_f64().max(1.0);
        }
        if let Ok(rtc) = self.video_track.get_stats().await {
            apply_outbound(&mut stats, &rtc);
        }
        stats
    }
}

fn apply_outbound(stats: &mut PublishStats, rtc: &[RtcStats]) {
    let mut mimes = std::collections::HashMap::new();
    for s in rtc {
        if let RtcStats::Codec(c) = s {
            mimes.insert(c.rtc.id.clone(), c.codec.mime_type.clone());
        }
    }
    for s in rtc {
        match s {
            RtcStats::OutboundRtp(o) if o.stream.kind == "video" => {
                stats.fps_sent = o.outbound.frames_per_second;
                stats.target_kbps = o.outbound.target_bitrate / 1000.0;
                // On the pass-through the SDK only knows "passthrough"; the
                // encoder that actually ran is ours, so keep our name.
                let reported = &o.outbound.encoder_implementation;
                if !reported.is_empty() && !reported.to_lowercase().contains("passthrough") {
                    stats.encoder = reported.clone();
                }
                stats.quality_limitation = format!("{:?}", o.outbound.quality_limitation_reason);
                if let Some(m) = mimes.get(&o.stream.codec_id) {
                    if m.contains("265") || m.to_lowercase().contains("hevc") {
                        stats.codec = Some(StreamCodec::H265);
                    } else if m.contains("264") {
                        stats.codec = Some(StreamCodec::H264);
                    }
                }
            }
            RtcStats::CandidatePair(p) if p.candidate_pair.nominated => {
                if p.candidate_pair.current_round_trip_time > 0.0 {
                    stats.rtt_ms = Some(p.candidate_pair.current_round_trip_time * 1000.0);
                }
            }
            RtcStats::RemoteInboundRtp(r) if r.stream.kind == "video" => {
                stats.packet_loss = Some(r.remote_inbound.fraction_lost.clamp(0.0, 1.0));
                if r.remote_inbound.round_trip_time > 0.0 {
                    stats.rtt_ms = Some(r.remote_inbound.round_trip_time * 1000.0);
                }
            }
            _ => {}
        }
    }
}
