//! Linux publish path: hand raw I420 to libwebrtc and let it encode.
//!
//! Unlike macOS and Windows there is no encoder of our own here. The vendored
//! libwebrtc already ships VAAPI and NVENC backends, and asking for one by name
//! at publish time is both less code and less to go wrong than a hand-written
//! encoder. What this module owns is the colour conversion the GPU path on the
//! other platforms gets for free.

use super::{EncoderConfig, EncoderCounters, PublishPath};
use crate::capture::CapturedFrame;
use crate::stream::{StreamCodec, StreamError};
use livekit::webrtc::prelude::VideoBuffer;
use livekit::webrtc::video_frame::{I420Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::native::NativeVideoSource;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use yuv::{
    bgra_to_yuv420, BufferStoreMut, YuvConversionMode, YuvPlanarImageMut, YuvRange,
    YuvStandardMatrix,
};

/// H.265 has no reliable software fallback across Linux desktops, and the
/// hardware path is not guaranteed either. Screen sharing stays on H.264.
pub fn clamp_codec(_codec: StreamCodec) -> StreamCodec {
    StreamCodec::H264
}

pub fn publish_path(_cfg: &EncoderConfig) -> PublishPath {
    PublishPath::Raw
}

pub struct VideoEncoder {
    cfg: EncoderConfig,
    source: NativeVideoSource,
    counters: Arc<EncoderCounters>,
    buffer: Mutex<Option<I420Buffer>>,
}

impl VideoEncoder {
    pub fn new(
        cfg: EncoderConfig,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        Ok(Self {
            cfg,
            source,
            counters,
            buffer: Mutex::new(None),
        })
    }

    pub fn encode(&self, frame: &CapturedFrame, _force_key: bool) -> Result<(), StreamError> {
        let started = Instant::now();
        self.counters.submitted.fetch_add(1, Ordering::Relaxed);
        let (w, h) = (frame.width, frame.height);
        if w == 0 || h == 0 {
            return Ok(());
        }
        let mut slot = self
            .buffer
            .lock()
            .map_err(|_| StreamError::Encoder("frame buffer poisoned".into()))?;
        let reusable = slot
            .as_ref()
            .map(|b| b.width() == w && b.height() == h)
            .unwrap_or(false);
        // The buffer is moved into the frame and moved back after the capture,
        // so a 1080p allocation happens once instead of thirty times a second.
        let mut buffer = if reusable {
            slot.take()
                .ok_or_else(|| StreamError::Encoder("no frame buffer".into()))?
        } else {
            I420Buffer::new(w, h)
        };
        let (stride_y, stride_u, stride_v) = buffer.strides();
        let (y, u, v) = buffer.data_mut();
        let mut planar = YuvPlanarImageMut {
            y_plane: BufferStoreMut::Borrowed(y),
            y_stride: stride_y,
            u_plane: BufferStoreMut::Borrowed(u),
            u_stride: stride_u,
            v_plane: BufferStoreMut::Borrowed(v),
            v_stride: stride_v,
            width: w,
            height: h,
        };
        bgra_to_yuv420(
            &mut planar,
            &frame.data,
            frame.stride as u32,
            // Screen content is studio-range BT.709: matching what every
            // compositor and every decoder assumes keeps greys grey.
            YuvRange::Limited,
            YuvStandardMatrix::Bt709,
            YuvConversionMode::Balanced,
        )
        .map_err(|e| StreamError::Encoder(format!("bgrx to i420: {e}")))?;

        let out = VideoFrame {
            rotation: VideoRotation::VideoRotation0,
            timestamp_us: frame.capture_us,
            frame_metadata: None,
            buffer,
        };
        self.source.capture_frame(&out);
        *slot = Some(out.buffer);
        self.counters.encoded.fetch_add(1, Ordering::Relaxed);
        self.counters.captured_ok.fetch_add(1, Ordering::Relaxed);
        let elapsed = started.elapsed().as_nanos() as u64;
        self.counters
            .latency_ns_sum
            .fetch_add(elapsed, Ordering::Relaxed);
        self.counters
            .latency_ns_max
            .fetch_max(elapsed, Ordering::Relaxed);
        Ok(())
    }

    /// Rate control belongs to libwebrtc on this path; the publication carries
    /// the ceiling and the encoder inside the SDK obeys it.
    pub fn set_bitrate(&self, bps: u64) {
        self.counters.applied_bps.store(bps, Ordering::Relaxed);
    }

    pub fn set_framerate(&self, _fps: u16) {}

    pub fn hardware(&self) -> Option<bool> {
        Some(matches!(
            preferred_backend(),
            livekit::options::VideoEncoderBackend::Vaapi
                | livekit::options::VideoEncoderBackend::Nvenc
        ))
    }

    pub fn name(&self) -> &str {
        match preferred_backend() {
            livekit::options::VideoEncoderBackend::Vaapi => "vaapi",
            livekit::options::VideoEncoderBackend::Nvenc => "nvenc",
            _ => "libwebrtc-software",
        }
    }

    pub fn finish(&self) {
        let _ = &self.cfg;
    }
}

/// Ask libwebrtc what it actually has rather than assuming a build flag. The
/// answer is also what the inspector reports, so the badge can never claim
/// hardware we did not get.
pub fn preferred_backend() -> livekit::options::VideoEncoderBackend {
    use livekit::options::VideoEncoderBackend as B;
    let available: Vec<B> = B::list_available().into_iter().collect();
    if available.contains(&B::Vaapi) {
        B::Vaapi
    } else if available.contains(&B::Nvenc) {
        B::Nvenc
    } else {
        B::Software
    }
}
