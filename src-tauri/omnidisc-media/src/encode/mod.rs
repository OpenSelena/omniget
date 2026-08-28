use crate::stream::{StreamCodec, StreamMode};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "macos")]
mod videotoolbox;
#[cfg(target_os = "macos")]
pub use videotoolbox::VideoEncoder;

#[cfg(not(target_os = "macos"))]
mod stub;
#[cfg(not(target_os = "macos"))]
pub use stub::VideoEncoder;

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
