use super::{AudioSink, CaptureApi, CaptureGeometry, CaptureOptions, VideoSink};
use crate::stream::{AudioMode, StreamError, StreamSources};

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub capture_us: i64,
}

impl CapturedFrame {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn capture_us(&self) -> i64 {
        self.capture_us
    }
}

pub struct VideoCapture;

impl VideoCapture {
    pub fn stop(self) {}
}
pub struct AudioCapture;

impl AudioCapture {
    pub fn stop(self) {}
}

pub struct Platform;

impl CaptureApi for Platform {
    fn list_sources(_thumbnails: bool) -> Result<StreamSources, StreamError> {
        Err(StreamError::Unsupported)
    }

    fn start_video(
        _opts: &CaptureOptions,
        _sink: VideoSink,
    ) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
        Err(StreamError::Unsupported)
    }

    fn start_audio(
        mode: AudioMode,
        _sink: AudioSink,
    ) -> Result<(AudioCapture, AudioMode), StreamError> {
        if mode == AudioMode::None {
            Ok((AudioCapture, AudioMode::None))
        } else {
            Err(StreamError::Unsupported)
        }
    }
}
