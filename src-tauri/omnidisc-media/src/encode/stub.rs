use super::{EncoderConfig, EncoderCounters, PublishPath};
use crate::capture::CapturedFrame;
use crate::stream::{StreamCodec, StreamError};
use livekit::webrtc::video_source::native::NativeVideoSource;
use std::sync::Arc;

pub fn clamp_codec(codec: StreamCodec) -> StreamCodec {
    codec
}

pub fn publish_path(_cfg: &EncoderConfig) -> PublishPath {
    PublishPath::PreEncoded
}

pub struct VideoEncoder;

impl VideoEncoder {
    pub fn new(
        _cfg: EncoderConfig,
        _source: NativeVideoSource,
        _counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        Err(StreamError::Unsupported)
    }

    pub fn encode(&self, _frame: &CapturedFrame, _force_key: bool) -> Result<(), StreamError> {
        Err(StreamError::Unsupported)
    }

    pub fn set_bitrate(&self, _bps: u64) {}

    pub fn set_framerate(&self, _fps: u16) {}

    pub fn hardware(&self) -> Option<bool> {
        None
    }

    pub fn name(&self) -> &str {
        "none"
    }

    pub fn finish(&self) {}
}
