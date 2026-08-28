mod audio;
pub mod d3d;
mod sources;
mod wgc;

pub use audio::{process_loopback_supported, windows_build, MIN_LOOPBACK_BUILD};
pub use d3d::{gpu, Gpu, Nv12Converter, Nv12Cpu, PooledTexture};

use super::{
    unix_micros, AudioSink, CaptureApi, CaptureGeometry, CaptureOptions, VideoSink, VideoTick,
};
use crate::stream::{AudioMode, SourceId, StreamError, StreamSources};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Graphics::Capture::GraphicsCaptureSession;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

/// One captured screen frame. The pixels stay on the GPU: `texture` is a BGRA
/// `ID3D11Texture2D` owned by the capture pool and recycled when the frame is
/// dropped, so nothing is read back to system memory unless the encoder asks.
pub struct CapturedFrame {
    texture: PooledTexture,
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

    pub fn texture(&self) -> Option<&ID3D11Texture2D> {
        self.texture.texture()
    }
}

struct SyntheticSource {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for SyntheticSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub struct VideoCapture {
    _wgc: Option<wgc::Capture>,
    _synthetic: Option<SyntheticSource>,
}

impl VideoCapture {
    pub fn stop(self) {}
}

pub struct AudioCapture {
    inner: Option<audio::AudioCaptureHandle>,
}

impl AudioCapture {
    pub fn stop(self) {
        if let Some(h) = self.inner {
            h.stop();
        }
    }
}

fn start_synthetic(
    width: u32,
    height: u32,
    fps: u16,
    sink: VideoSink,
) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
    let (width, height) = ((width.max(2)) & !1, (height.max(2)) & !1);
    let gpu = gpu()?;
    let pool = d3d::TexturePool::new(gpu.clone(), width, height, 4);
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("omnidisc-synthetic".into())
        .spawn(move || {
            let interval = Duration::from_nanos(1_000_000_000u64 / fps.max(1) as u64);
            let row = width as usize * 4;
            let mut pixels = vec![0u8; row * height as usize];
            let mut next = Instant::now();
            let mut n: u64 = 0;
            while !stop2.load(Ordering::Acquire) {
                let shift = (n % 256) as usize;
                for y in 0..height as usize {
                    let base = y * row;
                    let v = ((y / 4 + shift) % 256) as u8;
                    for x in 0..width as usize {
                        let p = base + x * 4;
                        pixels[p] = v;
                        pixels[p + 1] = ((x + shift) % 256) as u8;
                        pixels[p + 2] = v.wrapping_add(64);
                        pixels[p + 3] = 255;
                    }
                }
                match pool.acquire() {
                    Ok(slot) => {
                        if let Some(tex) = slot.texture() {
                            unsafe {
                                gpu.context().UpdateSubresource(
                                    tex,
                                    0,
                                    None,
                                    pixels.as_ptr().cast(),
                                    row as u32,
                                    0,
                                );
                            }
                            sink(VideoTick::Frame(CapturedFrame {
                                texture: slot,
                                width,
                                height,
                                capture_us: unix_micros(),
                            }));
                        }
                    }
                    Err(e) => tracing::debug!("[omnidisc-media] synthetic frame: {e}"),
                }
                n += 1;
                next += interval;
                let now = Instant::now();
                if next > now {
                    std::thread::sleep(next - now);
                } else if now - next > Duration::from_millis(200) {
                    next = now;
                }
            }
        })
        .map_err(|e| StreamError::Capture(format!("synthetic thread: {e}")))?;
    Ok((
        VideoCapture {
            _wgc: None,
            _synthetic: Some(SyntheticSource {
                stop,
                thread: Some(thread),
            }),
        },
        CaptureGeometry { width, height, fps },
    ))
}

fn wgc_available() -> Result<(), StreamError> {
    match GraphicsCaptureSession::IsSupported() {
        Ok(true) => Ok(()),
        _ => Err(StreamError::Capture(
            "Windows.Graphics.Capture is unavailable; screen sharing needs Windows 10 version 1903 or newer".into(),
        )),
    }
}

pub struct Platform;

impl CaptureApi for Platform {
    fn list_sources(thumbnails: bool) -> Result<StreamSources, StreamError> {
        let _com = audio::ComGuard::mta();
        wgc_available()?;
        let displays = sources::display_sources(thumbnails);
        if displays.is_empty() {
            return Err(StreamError::Capture(
                "no displays were reported by Windows".into(),
            ));
        }
        let windows = sources::window_sources(thumbnails);
        let loopback = audio::process_loopback_supported();
        let apps = if loopback {
            audio::audio_apps()
        } else {
            Vec::new()
        };
        Ok(StreamSources {
            displays,
            windows,
            apps,
            app_audio_supported: loopback,
            system_audio_supported: loopback,
        })
    }

    fn start_video(
        opts: &CaptureOptions,
        sink: VideoSink,
    ) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
        if let SourceId::Synthetic { width, height } = opts.source {
            return start_synthetic(width, height, opts.fps, sink);
        }
        // `GraphicsCaptureSession::IsSupported` is a WinRT static call, so the
        // caller's thread needs an apartment even though the session itself
        // runs on our own.
        let _com = audio::ComGuard::mta();
        wgc_available()?;
        let gpu = gpu()?;
        let target = match opts.source {
            SourceId::Display { id } => wgc::Target::Monitor(sources::monitor_from_id(id)?.handle),
            SourceId::Window { id } => {
                wgc::Target::Window(sources::window_from_id(id).ok_or(StreamError::SourceGone)?)
            }
            SourceId::Synthetic { .. } => return Err(StreamError::Unsupported),
        };
        let (capture, geometry) =
            wgc::start(gpu, target, sink, opts.cursor, opts.fps, opts.height)?;
        Ok((
            VideoCapture {
                _wgc: Some(capture),
                _synthetic: None,
            },
            geometry,
        ))
    }

    fn start_audio(
        mode: AudioMode,
        sink: AudioSink,
    ) -> Result<(AudioCapture, AudioMode), StreamError> {
        if mode == AudioMode::None {
            return Ok((AudioCapture { inner: None }, AudioMode::None));
        }
        if !audio::process_loopback_supported() {
            return Err(audio::unsupported_build_error());
        }
        let own_pid = std::process::id();
        let mut attempt = mode;
        loop {
            let started = match attempt {
                AudioMode::App { pid } if pid > 0 => {
                    audio::start_process_loopback(pid as u32, true, sink.clone())
                }
                AudioMode::App { pid } => Err(StreamError::Capture(format!(
                    "invalid audio source pid {pid}"
                ))),
                // "System except us": exclude our own process tree so the share
                // never echoes OmniGet's own output back into the room.
                AudioMode::System => audio::start_process_loopback(own_pid, false, sink.clone()),
                AudioMode::None => return Ok((AudioCapture { inner: None }, AudioMode::None)),
            };
            match started {
                Ok(handle) => {
                    return Ok((
                        AudioCapture {
                            inner: Some(handle),
                        },
                        attempt,
                    ))
                }
                Err(e) => {
                    tracing::warn!("[omnidisc-media] audio capture {attempt:?} failed: {e}");
                    attempt = match attempt {
                        AudioMode::App { .. } => AudioMode::System,
                        _ => return Ok((AudioCapture { inner: None }, AudioMode::None)),
                    };
                }
            }
        }
    }
}
