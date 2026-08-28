mod wgpu_surface;

pub use wgpu_surface::{FrameSlot, Planes, WgpuViewer};

use crate::stream::{StreamError, Viewport, WatchStats};
use futures::StreamExt;
use livekit::track::RemoteVideoTrack;
use livekit::webrtc::stats::RtcStats;
use livekit::webrtc::video_frame::native::VideoFrameBufferExt;
use livekit::webrtc::video_frame::{I420Buffer, VideoBuffer};
use livekit::webrtc::video_stream::native::NativeVideoStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
pub unsafe fn create_appkit_surface(ns_view: *mut std::ffi::c_void, width: u32, height: u32) -> Result<Arc<WgpuViewer>, StreamError> {
    use objc2::runtime::NSObjectProtocol;
    use objc2::ClassType;
    use objc2_app_kit::NSView;
    use objc2_quartz_core::CAMetalLayer;
    use raw_window_handle::{AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle};

    if ns_view.is_null() {
        return Err(StreamError::Viewer("null ns_view".into()));
    }
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(desc);
    let handle = AppKitWindowHandle::new(std::ptr::NonNull::new(ns_view).ok_or_else(|| StreamError::Viewer("null ns_view".into()))?);
    let surface = instance
        .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::AppKit(AppKitDisplayHandle::new())),
            raw_window_handle: RawWindowHandle::AppKit(handle),
        })
        .map_err(|e| StreamError::Viewer(format!("create_surface: {e}")))?;

    let view: &NSView = &*(ns_view as *const NSView);
    if let Some(root) = view.layer() {
        for l in root.sublayers().into_iter().flatten() {
            if l.isKindOfClass(CAMetalLayer::class()) {
                l.setZPosition(-1.0);
            }
        }
    }
    WgpuViewer::from_surface(instance, surface, width, height)
}

fn pack_i420(buf: &I420Buffer, full_range: bool) -> Planes {
    let (w, h) = (buf.width(), buf.height());
    let (cw, ch) = (buf.chroma_width(), buf.chroma_height());
    let (sy, su, sv) = buf.strides();
    let (dy, du, dv) = buf.data();
    let pack = |data: &[u8], stride: u32, width: u32, height: u32| {
        let stride = stride as usize;
        let width = width as usize;
        let mut out = Vec::with_capacity(width * height as usize);
        for row in 0..height as usize {
            let start = row * stride;
            if start + width <= data.len() {
                out.extend_from_slice(&data[start..start + width]);
            } else {
                out.resize(width * (row + 1), 16);
            }
        }
        out
    };
    Planes {
        y: pack(dy, sy, w, h),
        u: pack(du, su, cw, ch),
        v: pack(dv, sv, cw, ch),
        width: w,
        height: h,
        full_range,
    }
}

struct DecodeStats {
    received: AtomicU64,
    width: AtomicU64,
    height: AtomicU64,
}

pub struct Viewer {
    renderer: Arc<WgpuViewer>,
    track: RemoteVideoTrack,
    user_id: String,
    stop: Arc<AtomicBool>,
    decode_stats: Arc<DecodeStats>,
    started: Instant,
    render_thread: Option<std::thread::JoinHandle<()>>,
    decode_task: Option<tokio::task::JoinHandle<()>>,
}

impl Viewer {
    pub fn new(renderer: Arc<WgpuViewer>, track: RemoteVideoTrack, user_id: String, rt: tokio::runtime::Handle) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let decode_stats = Arc::new(DecodeStats { received: AtomicU64::new(0), width: AtomicU64::new(0), height: AtomicU64::new(0) });
        let slot = renderer.frame_slot();
        let stats2 = decode_stats.clone();
        let stop_dec = stop.clone();
        let rtc = track.rtc_track();
        let decode_task = rt.spawn(async move {
            let mut stream = NativeVideoStream::new(rtc);
            while let Some(frame) = stream.next().await {
                if stop_dec.load(Ordering::Relaxed) {
                    break;
                }
                let planes = if let Some(native) = frame.buffer.as_native() {
                    pack_i420(&native.to_i420(), false)
                } else if let Some(i) = frame.buffer.as_i420() {
                    pack_i420(i, false)
                } else {
                    continue;
                };
                stats2.received.fetch_add(1, Ordering::Relaxed);
                stats2.width.store(planes.width as u64, Ordering::Relaxed);
                stats2.height.store(planes.height as u64, Ordering::Relaxed);
                if let Ok(mut s) = slot.planes.lock() {
                    *s = Some(planes);
                }
            }
        });

        let renderer_r = renderer.clone();
        let stop_render = stop.clone();
        let render_thread = std::thread::Builder::new()
            .name("omnidisc-viewer-render".into())
            .spawn(move || {
                while !stop_render.load(Ordering::Relaxed) {
                    let painted = renderer_r.render_once();
                    if !painted {
                        std::thread::sleep(Duration::from_millis(16));
                    }
                }
            })
            .expect("spawn viewer render thread");

        Self { renderer, track, user_id, stop, decode_stats, started: Instant::now(), render_thread: Some(render_thread), decode_task: Some(decode_task) }
    }

    pub fn set_viewport(&self, viewport: Option<Viewport>) {
        self.renderer.set_viewport(viewport);
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub async fn stats(&self) -> WatchStats {
        let mut stats = WatchStats {
            user_id: self.user_id.clone(),
            width: self.decode_stats.width.load(Ordering::Relaxed) as u32,
            height: self.decode_stats.height.load(Ordering::Relaxed) as u32,
            fps_received: self.decode_stats.received.load(Ordering::Relaxed) as f64 / self.started.elapsed().as_secs_f64().max(1.0),
            fps_rendered: self.renderer.rendered_frames() as f64 / self.started.elapsed().as_secs_f64().max(1.0),
            ..Default::default()
        };
        if let Ok(rtc) = self.track.get_stats().await {
            for s in &rtc {
                if let RtcStats::InboundRtp(i) = s {
                    if i.stream.kind == "video" {
                        stats.fps_received = i.inbound.frames_per_second;
                        stats.decoder = i.inbound.decoder_implementation.clone();
                        stats.width = i.inbound.frame_width;
                        stats.height = i.inbound.frame_height;
                        stats.frames_dropped = i.inbound.frames_dropped as u64;
                        stats.freeze_count = i.inbound.freeze_count as u64;
                        stats.jitter_ms = Some(i.received.jitter * 1000.0);
                        stats.packet_loss = Some(i.received.packets_lost.max(0) as f64);
                    }
                }
                if let RtcStats::Codec(c) = s {
                    if c.codec.mime_type.to_lowercase().contains("video") {
                        stats.codec = c.codec.mime_type.clone();
                    }
                }
            }
        }
        stats
    }
}

impl Drop for Viewer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.renderer.stop();
        if let Some(t) = self.decode_task.take() {
            t.abort();
        }
        if let Some(t) = self.render_thread.take() {
            let _ = t.join();
        }
    }
}
