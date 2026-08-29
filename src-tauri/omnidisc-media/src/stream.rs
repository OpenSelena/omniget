use omnidisc_proto::bitrate::{Codec, StreamingPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceId {
    Display { id: u32 },
    Window { id: u32 },
    Synthetic { width: u32, height: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSource {
    pub id: SourceId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioApp {
    pub pid: i32,
    pub name: String,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamSources {
    pub displays: Vec<StreamSource>,
    pub windows: Vec<StreamSource>,
    pub apps: Vec<AudioApp>,
    pub app_audio_supported: bool,
    pub system_audio_supported: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AudioMode {
    #[default]
    None,
    App {
        pid: i32,
    },
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamMode {
    #[default]
    Text,
    Game,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StreamCodec {
    H264,
    H265,
}

impl From<Codec> for StreamCodec {
    fn from(c: Codec) -> Self {
        match c {
            Codec::H265 => StreamCodec::H265,
            _ => StreamCodec::H264,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    pub source: SourceId,
    pub fps: u16,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub audio: AudioMode,
    #[serde(default)]
    pub bitrate_kbps: Option<u32>,
    #[serde(default)]
    pub mode: StreamMode,
    #[serde(default = "default_cursor")]
    pub cursor: bool,
    #[serde(default)]
    pub policy: StreamingPolicy,
}

fn default_cursor() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedStream {
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub bitrate_kbps: u32,
    pub codec: StreamCodec,
    pub mode: StreamMode,
    pub audio: AudioMode,
}

pub fn resolve_geometry(native_w: u32, native_h: u32, requested_height: Option<u16>) -> (u32, u32) {
    let native_h = native_h.max(2);
    let native_w = native_w.max(2);
    let target_h = match requested_height {
        Some(h) if (h as u32) < native_h => h as u32,
        _ => native_h,
    };
    let target_w = (native_w as u64 * target_h as u64 / native_h as u64) as u32;
    ((target_w.max(2)) & !1, (target_h.max(2)) & !1)
}

pub fn resolve_bitrate(
    policy: &StreamingPolicy,
    width: u32,
    height: u32,
    fps: u16,
    requested_height: Option<u16>,
    custom_kbps: Option<u32>,
) -> u32 {
    if let Some(custom) = custom_kbps {
        return policy.clamp_custom(custom);
    }
    match requested_height {
        Some(h) if h as u32 >= height => policy.kbps_for(h, fps),
        _ => policy.native_kbps(width, height, fps),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("screen recording permission is missing")]
    Permission,
    #[error("screen share is not supported on this platform yet")]
    Unsupported,
    #[error("capture source is gone")]
    SourceGone,
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("encoder failed: {0}")]
    Encoder(String),
    #[error("not streaming")]
    NotStreaming,
    #[error("no such stream")]
    NoSuchStream,
    #[error("viewer failed: {0}")]
    Viewer(String),
    #[error("not connected to a voice channel")]
    NotConnected,
}

impl StreamError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Permission => "ERR_SCREEN_PERMISSION",
            Self::Unsupported => "ERR_STREAM_UNSUPPORTED",
            Self::SourceGone => "ERR_STREAM_SOURCE_GONE",
            Self::Capture(_) => "ERR_STREAM_CAPTURE_FAILED",
            Self::Encoder(_) => "ERR_STREAM_ENCODER_FAILED",
            Self::NotStreaming => "ERR_STREAM_NOT_STREAMING",
            Self::NoSuchStream => "ERR_STREAM_NOT_FOUND",
            Self::Viewer(_) => "ERR_STREAM_VIEWER_FAILED",
            Self::NotConnected => "ERR_VOICE_NOT_CONNECTED",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchState {
    Connecting,
    Playing,
    Ended,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    pub surface_width: u32,
    pub surface_height: u32,
    pub background: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PublishStats {
    pub width: u32,
    pub height: u32,
    pub fps_captured: f64,
    pub fps_encoded: f64,
    pub fps_sent: f64,
    pub bitrate_kbps: f64,
    pub target_kbps: f64,
    pub configured_kbps: u32,
    pub codec: Option<StreamCodec>,
    pub encoder: String,
    pub hardware: Option<bool>,
    pub encode_ms: f64,
    pub keyframes: u64,
    pub keyframe_requests: u64,
    pub frames_dropped: u64,
    pub rtt_ms: Option<f64>,
    pub packet_loss: Option<f64>,
    pub quality_limitation: String,
    pub audio: AudioMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WatchStats {
    pub user_id: String,
    pub width: u32,
    pub height: u32,
    pub fps_received: f64,
    pub fps_rendered: f64,
    pub bitrate_kbps: f64,
    pub codec: String,
    pub decoder: String,
    pub packet_loss: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub frames_dropped: u64,
    pub freeze_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StreamStats {
    pub publishing: Option<PublishStats>,
    pub watching: Vec<WatchStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamBadge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_keeps_aspect_and_never_upscales() {
        assert_eq!(resolve_geometry(3420, 2214, Some(1080)), (1668, 1080));
        assert_eq!(resolve_geometry(1920, 1080, Some(2160)), (1920, 1080));
        assert_eq!(resolve_geometry(3440, 1440, None), (3440, 1440));
        let (w, h) = resolve_geometry(1365, 767, Some(720));
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
    }

    #[test]
    fn bitrate_uses_matrix_for_presets_and_native_otherwise() {
        let p = StreamingPolicy::default();
        assert_eq!(
            resolve_bitrate(&p, 1920, 1080, 60, Some(1080), None),
            p.kbps_for(1080, 60)
        );
        assert_eq!(
            resolve_bitrate(&p, 3440, 1440, 60, None, None),
            p.native_kbps(3440, 1440, 60)
        );
        assert_eq!(
            resolve_bitrate(&p, 1920, 1080, 60, Some(1080), Some(7_777)),
            p.clamp_custom(7_777)
        );
    }
}
