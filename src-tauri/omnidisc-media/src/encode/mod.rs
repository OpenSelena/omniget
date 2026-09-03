use crate::stream::{StreamCodec, StreamMode};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "macos")]
mod videotoolbox;
#[cfg(target_os = "macos")]
pub use self::videotoolbox::{clamp_codec, publish_path, VideoEncoder};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{clamp_codec, publish_path, VideoEncoder};

#[cfg(all(target_os = "linux", feature = "linux-capture"))]
mod linux;
#[cfg(all(target_os = "linux", feature = "linux-capture"))]
pub use self::linux::{clamp_codec, preferred_backend, publish_path, VideoEncoder};

#[cfg(not(any(
    target_os = "macos",
    windows,
    all(target_os = "linux", feature = "linux-capture")
)))]
mod stub;
#[cfg(not(any(
    target_os = "macos",
    windows,
    all(target_os = "linux", feature = "linux-capture")
)))]
pub use self::stub::{clamp_codec, publish_path, VideoEncoder};

/// How a platform hands frames to LiveKit: through our own encoder and the
/// `PreEncoded` pass-through, or as raw frames for libwebrtc to encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishPath {
    PreEncoded,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub codec: StreamCodec,
    pub bitrate_kbps: u32,
    pub mode: StreamMode,
}

#[derive(Default, Debug)]
pub struct EncoderCounters {
    pub submitted: AtomicU64,
    pub encoded: AtomicU64,
    pub keyframes: AtomicU64,
    pub dropped: AtomicU64,
    pub errors: AtomicU64,
    pub bytes: AtomicU64,
    pub latency_ns_sum: AtomicU64,
    pub latency_ns_max: AtomicU64,
    pub applied_bps: AtomicU64,
    pub rate_requests: AtomicU64,
    pub keyframe_requests: AtomicU64,
    pub captured_ok: AtomicU64,
    pub captured_rejected: AtomicU64,
}

impl EncoderCounters {
    pub fn applied_kbps(&self) -> f64 {
        self.applied_bps.load(Ordering::Relaxed) as f64 / 1000.0
    }
}
