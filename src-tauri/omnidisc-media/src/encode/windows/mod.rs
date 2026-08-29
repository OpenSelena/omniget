mod mf;
mod raw;

use super::{EncoderConfig, EncoderCounters, PublishPath};
use crate::capture::CapturedFrame;
use crate::stream::{StreamCodec, StreamError};
use livekit::webrtc::video_source::native::NativeVideoSource;
use std::sync::{Arc, OnceLock};

/// Windows publishes H.264 only: the Media Foundation encoder wired here is
/// H.264, and libwebrtc's Windows build has no HEVC encoder for the raw path.
pub fn clamp_codec(codec: StreamCodec) -> StreamCodec {
    if codec != StreamCodec::H264 {
        tracing::info!("[omnidisc-media] {codec:?} is not encodable on Windows; using H.264");
    }
    StreamCodec::H264
}

fn forced_path() -> Option<PublishPath> {
    match std::env::var("OMNIDISC_WIN_ENCODER")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "raw" | "libwebrtc" | "software" => Some(PublishPath::Raw),
        "mf" | "mediafoundation" | "hardware" => Some(PublishPath::PreEncoded),
        _ => None,
    }
}

fn hardware_present() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(mf::hardware_h264_available)
}

pub fn publish_path(_cfg: &EncoderConfig) -> PublishPath {
    if let Some(forced) = forced_path() {
        return forced;
    }
    if hardware_present() {
        PublishPath::PreEncoded
    } else {
        PublishPath::Raw
    }
}

pub enum VideoEncoder {
    Mf(mf::MfEncoder),
    Raw(raw::RawEncoder),
}

impl VideoEncoder {
    pub fn new(
        cfg: EncoderConfig,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        match publish_path(&cfg) {
            PublishPath::PreEncoded => mf::MfEncoder::new(cfg, source, counters)
                .map(VideoEncoder::Mf)
                .map_err(|e| {
                    StreamError::Encoder(format!(
                        "{e}; set OMNIDISC_WIN_ENCODER=raw to publish through the software encoder instead"
                    ))
                }),
            PublishPath::Raw => raw::RawEncoder::new(cfg, source, counters).map(VideoEncoder::Raw),
        }
    }

    pub fn encode(&self, frame: &CapturedFrame, force_key: bool) -> Result<(), StreamError> {
        match self {
            Self::Mf(e) => e.encode(frame, force_key),
            Self::Raw(e) => e.encode(frame, force_key),
        }
    }

    pub fn set_bitrate(&self, bps: u64) {
        match self {
            Self::Mf(e) => e.set_bitrate(bps),
            Self::Raw(e) => e.set_bitrate(bps),
        }
    }

    pub fn set_framerate(&self, fps: u16) {
        match self {
            Self::Mf(e) => e.set_framerate(fps),
            Self::Raw(e) => e.set_framerate(fps),
        }
    }

    pub fn hardware(&self) -> Option<bool> {
        match self {
            Self::Mf(e) => e.hardware(),
            Self::Raw(e) => e.hardware(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Mf(e) => e.name(),
            Self::Raw(e) => e.name(),
        }
    }

    pub fn finish(&self) {
        match self {
            Self::Mf(e) => e.finish(),
            Self::Raw(e) => e.finish(),
        }
    }
}
