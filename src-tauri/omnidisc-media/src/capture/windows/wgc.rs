use super::d3d::{Gpu, TexturePool};
use super::CapturedFrame;
use crate::capture::{unix_micros, CaptureGeometry, VideoSink, VideoTick};
use crate::stream::{resolve_geometry, StreamError};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use windows::core::{Interface, Ref};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_BOX};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

const POOL_BUFFERS: i32 = 2;
const POOL_BUDGET: usize = 4;

pub enum Target {
    Monitor(HMONITOR),
    Window(HWND),
}

// SAFETY: HMONITOR and HWND are opaque handle values, not thread-affine
// pointers; they are only passed to the capture-item interop factory.
unsafe impl Send for Target {}

/// `IDirect3DDevice` is agile (WinRT free-threaded marshalling), so it may be
/// used from the frame-pool callback thread that WGC picks.
struct SendDevice(IDirect3DDevice);
unsafe impl Send for SendDevice {}

impl SendDevice {
    // A method, not a field read: edition-2021 closures capture disjoint
    // fields, and capturing the bare `IDirect3DDevice` would make the handler
    // closure `!Send`.
    fn get(&self) -> &IDirect3DDevice {
        &self.0
    }
}

fn cap_err(what: &str, e: windows::core::Error) -> StreamError {
    StreamError::Capture(format!("{what}: {e}"))
}

pub fn direct3d_device(gpu: &Gpu) -> Result<IDirect3DDevice, StreamError> {
    let dxgi: IDXGIDevice = gpu.device().cast().map_err(|e| cap_err("IDXGIDevice", e))?;
    let inspectable = unsafe {
        CreateDirect3D11DeviceFromDXGIDevice(&dxgi)
            .map_err(|e| cap_err("CreateDirect3D11DeviceFromDXGIDevice", e))?
    };
    inspectable
        .cast()
        .map_err(|e| cap_err("IDirect3DDevice", e))
}

pub fn capture_item(target: &Target) -> Result<GraphicsCaptureItem, StreamError> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .map_err(|e| cap_err("IGraphicsCaptureItemInterop", e))?;
    let item = unsafe {
        match target {
            Target::Monitor(m) => interop.CreateForMonitor::<GraphicsCaptureItem>(*m),
            Target::Window(w) => interop.CreateForWindow::<GraphicsCaptureItem>(*w),
        }
    };
    item.map_err(|e| {
        // WGC refuses items the user is not allowed to capture, and refuses
        // everything when the machine is below Windows 10 1903.
        if e.code().0 == windows::Win32::Foundation::E_INVALIDARG.0 {
            StreamError::SourceGone
        } else {
            cap_err("GraphicsCaptureItem", e)
        }
    })
}

struct Session {
    session: GraphicsCaptureSession,
    pool: Direct3D11CaptureFramePool,
}

// SAFETY: WGC objects are agile (free-threaded marshalled); the session is
// created and closed on the capture thread and never touched concurrently.
unsafe impl Send for Session {}

pub struct Capture {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Capture {
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    gpu: &Arc<Gpu>,
    item: &GraphicsCaptureItem,
    pool: TexturePool,
    sink: VideoSink,
    cursor: bool,
    fps: u16,
    last_frame_us: Arc<AtomicU64>,
) -> Result<Session, StreamError> {
    let device = direct3d_device(gpu)?;
    let size = item.Size().map_err(|e| cap_err("item size", e))?;
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        POOL_BUFFERS,
        size,
    )
    .map_err(|e| cap_err("Direct3D11CaptureFramePool", e))?;

    let gpu_cb = gpu.clone();
    let min_interval = Duration::from_nanos(1_000_000_000u64 / fps.max(1) as u64);
    let mut last_emit: Option<Instant> = None;
    let mut pool_size = size;
    let device_cb = SendDevice(device.clone());
    let handler = TypedEventHandler::<Direct3D11CaptureFramePool, windows::core::IInspectable>::new(
        move |sender: Ref<'_, Direct3D11CaptureFramePool>, _| {
            let Ok(sender) = sender.ok() else {
                return Ok(());
            };
            let Ok(frame) = sender.TryGetNextFrame() else {
                return Ok(());
            };
            if let Ok(content) = frame.ContentSize() {
                if content.Width != pool_size.Width || content.Height != pool_size.Height {
                    // WHY: WGC stops delivering frames after the item resizes
                    // unless the pool is recreated at the new size. Our own
                    // textures stay at the size the encoder was configured for;
                    // the copy below clamps to the smaller of the two.
                    let _ = sender.Recreate(
                        device_cb.get(),
                        DirectXPixelFormat::B8G8R8A8UIntNormalized,
                        POOL_BUFFERS,
                        content,
                    );
                    pool_size = content;
                }
            }
            let now = Instant::now();
            if let Some(prev) = last_emit {
                if now.duration_since(prev) + Duration::from_micros(500) < min_interval {
                    return Ok(());
                }
            }
            let Ok(surface) = frame.Surface() else {
                return Ok(());
            };
            let Ok(access) = surface.cast::<IDirect3DDxgiInterfaceAccess>() else {
                return Ok(());
            };
            let source: ID3D11Texture2D = match unsafe { access.GetInterface() } {
                Ok(t) => t,
                Err(_) => return Ok(()),
            };
            let slot = match pool.acquire() {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!("[omnidisc-media] wgc: {e}");
                    return Ok(());
                }
            };
            let Some(dst) = slot.texture() else {
                return Ok(());
            };
            let (pw, ph) = pool.size();
            let src_w = pool_size.Width.max(0) as u32;
            let src_h = pool_size.Height.max(0) as u32;
            let box_ = D3D11_BOX {
                left: 0,
                top: 0,
                front: 0,
                right: pw.min(src_w),
                bottom: ph.min(src_h),
                back: 1,
            };
            if box_.right == 0 || box_.bottom == 0 {
                return Ok(());
            }
            unsafe {
                gpu_cb
                    .context()
                    .CopySubresourceRegion(dst, 0, 0, 0, 0, &source, 0, Some(&box_));
            }
            last_emit = Some(now);
            let capture_us = unix_micros();
            last_frame_us.store(capture_us as u64, Ordering::Relaxed);
            sink(VideoTick::Frame(CapturedFrame {
                texture: slot,
                width: pw,
                height: ph,
                capture_us,
            }));
            Ok(())
        },
    );
    frame_pool
        .FrameArrived(&handler)
        .map_err(|e| cap_err("FrameArrived", e))?;

    let session = frame_pool
        .CreateCaptureSession(item)
        .map_err(|e| cap_err("CreateCaptureSession", e))?;
    if let Err(e) = session.SetIsCursorCaptureEnabled(cursor) {
        tracing::debug!("[omnidisc-media] SetIsCursorCaptureEnabled unavailable: {e}");
    }
    // Windows 11 only; on Windows 10 the yellow border cannot be removed.
    if let Err(e) = session.SetIsBorderRequired(false) {
        tracing::debug!("[omnidisc-media] SetIsBorderRequired unavailable: {e}");
    }
    session
        .StartCapture()
        .map_err(|e| cap_err("StartCapture", e))?;
    Ok(Session {
        session,
        pool: frame_pool,
    })
}

/// Starts a WGC capture on a dedicated MTA thread. The thread also emits
/// `VideoTick::Idle` whenever a frame interval passes without a new frame — WGC
/// is dirty-driven like ScreenCaptureKit, so a still screen delivers nothing.
pub fn start(
    gpu: Arc<Gpu>,
    target: Target,
    sink: VideoSink,
    cursor: bool,
    fps: u16,
    requested_height: Option<u16>,
) -> Result<(Capture, CaptureGeometry), StreamError> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let (tx, rx) = mpsc::channel::<Result<CaptureGeometry, StreamError>>();
    let thread = std::thread::Builder::new()
        .name("omnidisc-wgc".into())
        .spawn(move || {
            super::audio::ensure_mta();
            let last_frame_us = Arc::new(AtomicU64::new(0));
            let started = capture_item(&target).and_then(|item| {
                let size: SizeInt32 = item.Size().map_err(|e| cap_err("item size", e))?;
                let (native_w, native_h) = (size.Width.max(2) as u32, size.Height.max(2) as u32);
                let (width, height) = resolve_geometry(native_w, native_h, requested_height);
                let pool = TexturePool::new(gpu.clone(), native_w, native_h, POOL_BUDGET);
                let session = build(
                    &gpu,
                    &item,
                    pool,
                    sink.clone(),
                    cursor,
                    fps,
                    last_frame_us.clone(),
                )?;
                Ok((session, CaptureGeometry { width, height, fps }))
            });
            let session = match started {
                Ok((session, geometry)) => {
                    let _ = tx.send(Ok(geometry));
                    session
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let interval = Duration::from_nanos(1_000_000_000u64 / fps.max(1) as u64);
            while !stop_thread.load(Ordering::Acquire) {
                std::thread::sleep(interval);
                let last = last_frame_us.load(Ordering::Relaxed) as i64;
                if unix_micros() - last > interval.as_micros() as i64 {
                    sink(VideoTick::Idle);
                }
            }
            let _ = session.session.Close();
            let _ = session.pool.Close();
        })
        .map_err(|e| StreamError::Capture(format!("wgc thread: {e}")))?;
    match rx.recv() {
        Ok(Ok(geometry)) => Ok((
            Capture {
                stop,
                thread: Some(thread),
            },
            geometry,
        )),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            let _ = thread.join();
            Err(StreamError::Capture(
                "the screen capture thread stopped before it started".into(),
            ))
        }
    }
}
