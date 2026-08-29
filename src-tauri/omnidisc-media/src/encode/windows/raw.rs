use crate::capture::{gpu, CapturedFrame, Gpu, Nv12Converter};
use crate::encode::{EncoderConfig, EncoderCounters};
use crate::stream::StreamError;
use livekit::webrtc::video_frame::{NV12Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::native::NativeVideoSource;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Publishes raw NV12 frames and lets libwebrtc pick the encoder. This is the
/// path Windows uses when no hardware Media Foundation H.264 MFT is available:
/// slower and software-bound, but it always produces a working share.
pub struct RawEncoder {
    source: NativeVideoSource,
    counters: Arc<EncoderCounters>,
    converter: Mutex<Option<Nv12Converter>>,
    gpu: Arc<Gpu>,
    cfg: EncoderConfig,
}

impl RawEncoder {
    pub fn new(
        cfg: EncoderConfig,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        let gpu = gpu().map_err(|e| StreamError::Encoder(e.to_string()))?;
        counters
            .applied_bps
            .store(cfg.bitrate_kbps as u64 * 1000, Ordering::Relaxed);
        Ok(Self {
            source,
            counters,
            converter: Mutex::new(None),
            gpu,
            cfg,
        })
    }

    pub fn encode(&self, frame: &CapturedFrame, _force_key: bool) -> Result<(), StreamError> {
        let Some(texture) = frame.texture() else {
            return Err(StreamError::Encoder("captured frame has no texture".into()));
        };
        let started = Instant::now();
        let mut slot = self
            .converter
            .lock()
            .map_err(|_| StreamError::Encoder("converter poisoned".into()))?;
        let needs_new = match slot.as_ref() {
            Some(c) => c.size() != (self.cfg.width, self.cfg.height),
            None => true,
        };
        if needs_new {
            *slot = Some(Nv12Converter::new(
                self.gpu.clone(),
                frame.width(),
                frame.height(),
                self.cfg.width,
                self.cfg.height,
            )?);
        }
        let converter = slot
            .as_mut()
            .ok_or_else(|| StreamError::Encoder("no converter".into()))?;
        let nv12 = converter.convert(texture)?;
        let cpu = converter.read_back(&nv12)?;
        drop(slot);

        let mut buffer = NV12Buffer::with_strides(
            self.cfg.width,
            self.cfg.height,
            self.cfg.width,
            self.cfg.width,
        );
        {
            let (dst_y, dst_uv) = buffer.data_mut();
            let (src_y, src_uv) = cpu.planes();
            let w = self.cfg.width as usize;
            for row in 0..self.cfg.height as usize {
                let src = row * cpu.stride_y as usize;
                let dst = row * w;
                if src + w <= src_y.len() && dst + w <= dst_y.len() {
                    dst_y[dst..dst + w].copy_from_slice(&src_y[src..src + w]);
                }
            }
            for row in 0..(self.cfg.height / 2) as usize {
                let src = row * cpu.stride_uv as usize;
                let dst = row * w;
                if src + w <= src_uv.len() && dst + w <= dst_uv.len() {
                    dst_uv[dst..dst + w].copy_from_slice(&src_uv[src..src + w]);
                }
            }
        }
        let video = VideoFrame {
            rotation: VideoRotation::VideoRotation0,
            timestamp_us: frame.capture_us(),
            frame_metadata: None,
            buffer,
        };
        self.source.capture_frame(&video);
        self.counters.submitted.fetch_add(1, Ordering::Relaxed);
        self.counters.encoded.fetch_add(1, Ordering::Relaxed);
        self.counters.captured_ok.fetch_add(1, Ordering::Relaxed);
        let latency = started.elapsed().as_nanos() as u64;
        self.counters
            .latency_ns_sum
            .fetch_add(latency, Ordering::Relaxed);
        self.counters
            .latency_ns_max
            .fetch_max(latency, Ordering::Relaxed);
        Ok(())
    }

    pub fn set_bitrate(&self, bps: u64) {
        // libwebrtc owns rate control on this path; record what was asked so the
        // debug panel does not claim a number we never applied.
        self.counters.applied_bps.store(bps, Ordering::Relaxed);
    }

    pub fn set_framerate(&self, _fps: u16) {}

    /// Unknown until the SDK reports `encoder_implementation`; never claim
    /// hardware we did not select ourselves.
    pub fn hardware(&self) -> Option<bool> {
        None
    }

    pub fn name(&self) -> &str {
        "libwebrtc"
    }

    pub fn finish(&self) {}
}
