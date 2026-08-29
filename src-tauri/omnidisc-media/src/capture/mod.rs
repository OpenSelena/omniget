use crate::stream::{AudioMode, SourceId, StreamError, StreamSources};
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(all(target_os = "linux", feature = "linux-capture"))]
mod linux;
#[cfg(all(target_os = "linux", feature = "linux-capture"))]
pub use self::linux::*;

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
pub use self::stub::*;

/// Whether this build can capture a screen at all. The interface asks the
/// backend rather than the operating system, so a client compiled without the
/// Linux capture backend says so instead of offering a button that fails.
pub const SCREEN_CAPTURE_SUPPORTED: bool = cfg!(any(
    target_os = "macos",
    windows,
    all(target_os = "linux", feature = "linux-capture")
));

#[derive(Debug, Clone)]
pub struct CaptureOptions {
    pub source: SourceId,
    pub fps: u16,
    pub height: Option<u16>,
    pub cursor: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureGeometry {
    pub width: u32,
    pub height: u32,
    pub fps: u16,
}

pub enum VideoTick {
    Frame(CapturedFrame),
    Idle,
}

pub type VideoSink = Arc<dyn Fn(VideoTick) + Send + Sync>;
pub type AudioSink = Arc<dyn Fn(&[f32]) + Send + Sync>;

pub const AUDIO_SAMPLE_RATE: u32 = 48_000;
pub const AUDIO_CHANNELS: u32 = 2;

pub fn unix_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

pub trait CaptureApi {
    fn list_sources(thumbnails: bool) -> Result<StreamSources, StreamError>;
    fn thumbnail_for(source: &SourceId) -> Option<String>;
    fn start_video(
        opts: &CaptureOptions,
        sink: VideoSink,
    ) -> Result<(VideoCapture, CaptureGeometry), StreamError>;
    fn start_audio(
        mode: AudioMode,
        sink: AudioSink,
    ) -> Result<(AudioCapture, AudioMode), StreamError>;
}

pub fn list_sources(thumbnails: bool) -> Result<StreamSources, StreamError> {
    Platform::list_sources(thumbnails)
}

pub fn thumbnail_for(source: &SourceId) -> Option<String> {
    Platform::thumbnail_for(source)
}

pub fn start_video(
    opts: &CaptureOptions,
    sink: VideoSink,
) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
    Platform::start_video(opts, sink)
}

pub fn start_audio(
    mode: AudioMode,
    sink: AudioSink,
) -> Result<(AudioCapture, AudioMode), StreamError> {
    Platform::start_audio(mode, sink)
}
