use crate::stream::StreamError;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL,
    D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Multithread, ID3D11Texture2D,
    ID3D11VideoContext, ID3D11VideoDevice, ID3D11VideoProcessor, ID3D11VideoProcessorEnumerator,
    ID3D11VideoProcessorInputView, ID3D11VideoProcessorOutputView, D3D11_BIND_RENDER_TARGET,
    D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
    D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE, D3D11_VIDEO_PROCESSOR_CONTENT_DESC,
    D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0,
    D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0,
    D3D11_VIDEO_PROCESSOR_STREAM, D3D11_VPIV_DIMENSION, D3D11_VPIV_DIMENSION_TEXTURE2D,
    D3D11_VPOV_DIMENSION_TEXTURE2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};

const NV12_RING: usize = 3;

/// D3D11 device shared by the capture session and the encoder.
///
/// COM interfaces from `windows-rs` are `!Send`, but this device is created
/// with multithread protection on (`ID3D11Multithread::SetMultithreadProtected`),
/// so the immediate context may legally be driven from the WGC callback thread
/// and the encode thread at the same time. That is the invariant behind the
/// `Send`/`Sync` impls below; do not hand these objects to a context that has
/// multithread protection disabled.
pub struct Gpu {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}

unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}

fn hr(what: &str, e: windows::core::Error) -> StreamError {
    StreamError::Capture(format!("{what}: {e}"))
}

fn create_device(
    driver: D3D_DRIVER_TYPE,
) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    let levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    let mut level = D3D_FEATURE_LEVEL::default();
    unsafe {
        D3D11CreateDevice(
            None,
            driver,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
            Some(&levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut level),
            Some(&mut context),
        )?;
    }
    match (device, context) {
        (Some(d), Some(c)) => Ok((d, c)),
        _ => Err(windows::core::Error::from_win32()),
    }
}

impl Gpu {
    fn create() -> Result<Self, StreamError> {
        let (device, context) = match create_device(D3D_DRIVER_TYPE_HARDWARE) {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(
                    "[omnidisc-media] no hardware D3D11 device ({e}); falling back to the WARP software rasteriser, screen share will be slow"
                );
                create_device(D3D_DRIVER_TYPE_WARP)
                    .map_err(|e| hr("D3D11CreateDevice (WARP)", e))?
            }
        };
        let multithread: ID3D11Multithread =
            context.cast().map_err(|e| hr("ID3D11Multithread", e))?;
        unsafe {
            let _ = multithread.SetMultithreadProtected(true);
        }
        Ok(Self { device, context })
    }

    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    pub fn context(&self) -> &ID3D11DeviceContext {
        &self.context
    }

    fn texture(
        &self,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
        staging: bool,
        seed: Option<&[u8]>,
        seed_pitch: u32,
    ) -> Result<ID3D11Texture2D, StreamError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: if staging {
                D3D11_USAGE_STAGING
            } else {
                D3D11_USAGE_DEFAULT
            },
            BindFlags: if staging {
                0
            } else {
                (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32
            },
            CPUAccessFlags: if staging {
                D3D11_CPU_ACCESS_READ.0 as u32
            } else {
                0
            },
            MiscFlags: 0,
        };
        let initial = seed.map(|data| D3D11_SUBRESOURCE_DATA {
            pSysMem: data.as_ptr().cast(),
            SysMemPitch: seed_pitch,
            SysMemSlicePitch: 0,
        });
        let mut out: Option<ID3D11Texture2D> = None;
        unsafe {
            self.device
                .CreateTexture2D(
                    &desc,
                    initial.as_ref().map(|d| d as *const _),
                    Some(&mut out),
                )
                .map_err(|e| hr("CreateTexture2D", e))?;
        }
        out.ok_or_else(|| StreamError::Capture("CreateTexture2D returned nothing".into()))
    }

    pub fn bgra_texture(&self, width: u32, height: u32) -> Result<ID3D11Texture2D, StreamError> {
        self.texture(width, height, DXGI_FORMAT_B8G8R8A8_UNORM, false, None, 0)
    }
}

/// Process-wide device. Capture and encode must share one so a captured texture
/// can be converted without crossing devices.
pub fn gpu() -> Result<Arc<Gpu>, StreamError> {
    static GPU: OnceLock<Result<Arc<Gpu>, String>> = OnceLock::new();
    GPU.get_or_init(|| Gpu::create().map(Arc::new).map_err(|e| e.to_string()))
        .clone()
        .map_err(StreamError::Capture)
}

struct PoolInner {
    free: Vec<ID3D11Texture2D>,
}

unsafe impl Send for PoolInner {}

/// Fixed-size ring of BGRA textures so the capture callback never writes into a
/// texture the encoder still holds.
pub struct TexturePool {
    inner: Arc<Mutex<PoolInner>>,
    gpu: Arc<Gpu>,
    width: u32,
    height: u32,
    budget: usize,
    handed_out: Arc<Mutex<usize>>,
}

impl TexturePool {
    pub fn new(gpu: Arc<Gpu>, width: u32, height: u32, budget: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolInner { free: Vec::new() })),
            gpu,
            width,
            height,
            budget,
            handed_out: Arc::new(Mutex::new(0)),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn acquire(&self) -> Result<PooledTexture, StreamError> {
        let reused = self.inner.lock().ok().and_then(|mut g| g.free.pop());
        let texture = match reused {
            Some(t) => t,
            None => {
                let live = self.handed_out.lock().map(|g| *g).unwrap_or(0);
                if live >= self.budget {
                    return Err(StreamError::Capture(
                        "capture texture pool exhausted; the encoder is not releasing frames"
                            .into(),
                    ));
                }
                self.gpu.bgra_texture(self.width, self.height)?
            }
        };
        if let Ok(mut g) = self.handed_out.lock() {
            *g += 1;
        }
        Ok(PooledTexture {
            texture: Some(texture),
            pool: Arc::downgrade(&self.inner),
            handed_out: Arc::downgrade(&self.handed_out),
        })
    }
}

pub struct PooledTexture {
    texture: Option<ID3D11Texture2D>,
    pool: Weak<Mutex<PoolInner>>,
    handed_out: Weak<Mutex<usize>>,
}

// SAFETY: the texture is only ever touched through the shared `Gpu`, whose
// immediate context is multithread-protected (see `Gpu`).
unsafe impl Send for PooledTexture {}
unsafe impl Sync for PooledTexture {}

impl PooledTexture {
    pub fn texture(&self) -> Option<&ID3D11Texture2D> {
        self.texture.as_ref()
    }
}

impl Drop for PooledTexture {
    fn drop(&mut self) {
        if let (Some(t), Some(pool)) = (self.texture.take(), self.pool.upgrade()) {
            if let Ok(mut g) = pool.lock() {
                g.free.push(t);
            }
        }
        if let Some(count) = self.handed_out.upgrade() {
            if let Ok(mut g) = count.lock() {
                *g = g.saturating_sub(1);
            }
        }
    }
}

/// BGRA -> NV12 with scaling, on the GPU, through the D3D11 video processor.
/// Both publish paths need NV12: Media Foundation encoders take it directly and
/// libwebrtc's `NV12Buffer` is the cheapest raw hand-off.
pub struct Nv12Converter {
    gpu: Arc<Gpu>,
    video_device: ID3D11VideoDevice,
    video_context: ID3D11VideoContext,
    enumerator: ID3D11VideoProcessorEnumerator,
    processor: ID3D11VideoProcessor,
    ring: Vec<(ID3D11Texture2D, ID3D11VideoProcessorOutputView)>,
    next: usize,
    staging: Option<ID3D11Texture2D>,
    dst_width: u32,
    dst_height: u32,
}

// SAFETY: same invariant as `Gpu` — the underlying immediate context is
// multithread-protected, and a converter is only driven from one thread at a
// time (the encode loop owns it).
unsafe impl Send for Nv12Converter {}

pub struct Nv12Cpu {
    pub data: Vec<u8>,
    pub stride_y: u32,
    pub stride_uv: u32,
    pub width: u32,
    pub height: u32,
}

impl Nv12Cpu {
    pub fn planes(&self) -> (&[u8], &[u8]) {
        let split = (self.stride_y * self.height) as usize;
        let split = split.min(self.data.len());
        self.data.split_at(split)
    }
}

impl Nv12Converter {
    pub fn new(
        gpu: Arc<Gpu>,
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
    ) -> Result<Self, StreamError> {
        let video_device: ID3D11VideoDevice = gpu
            .device()
            .cast()
            .map_err(|e| hr("ID3D11VideoDevice (no GPU video support)", e))?;
        let video_context: ID3D11VideoContext = gpu
            .context()
            .cast()
            .map_err(|e| hr("ID3D11VideoContext", e))?;
        let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            InputWidth: src_width,
            InputHeight: src_height,
            OutputFrameRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            OutputWidth: dst_width,
            OutputHeight: dst_height,
            Usage: windows::Win32::Graphics::Direct3D11::D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };
        let enumerator = unsafe {
            video_device
                .CreateVideoProcessorEnumerator(&desc)
                .map_err(|e| hr("CreateVideoProcessorEnumerator", e))?
        };
        let processor = unsafe {
            video_device
                .CreateVideoProcessor(&enumerator, 0)
                .map_err(|e| hr("CreateVideoProcessor", e))?
        };
        unsafe {
            video_context.VideoProcessorSetStreamFrameFormat(
                &processor,
                0,
                D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            );
        }

        let mut ring = Vec::with_capacity(NV12_RING);
        for _ in 0..NV12_RING {
            let tex = gpu.texture(dst_width, dst_height, DXGI_FORMAT_NV12, false, None, 0)?;
            let view_desc = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
                ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                    Texture2D: windows::Win32::Graphics::Direct3D11::D3D11_TEX2D_VPOV {
                        MipSlice: 0,
                    },
                },
            };
            let mut view: Option<ID3D11VideoProcessorOutputView> = None;
            unsafe {
                video_device
                    .CreateVideoProcessorOutputView(&tex, &enumerator, &view_desc, Some(&mut view))
                    .map_err(|e| hr("CreateVideoProcessorOutputView", e))?;
            }
            let view = view.ok_or_else(|| {
                StreamError::Capture("CreateVideoProcessorOutputView returned nothing".into())
            })?;
            ring.push((tex, view));
        }

        Ok(Self {
            gpu,
            video_device,
            video_context,
            enumerator,
            processor,
            ring,
            next: 0,
            staging: None,
            dst_width,
            dst_height,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.dst_width, self.dst_height)
    }

    pub fn convert(&mut self, src: &ID3D11Texture2D) -> Result<ID3D11Texture2D, StreamError> {
        let idx = self.next % self.ring.len();
        self.next = self.next.wrapping_add(1);
        let input_desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION(D3D11_VPIV_DIMENSION_TEXTURE2D.0),
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                Texture2D: windows::Win32::Graphics::Direct3D11::D3D11_TEX2D_VPIV {
                    MipSlice: 0,
                    ArraySlice: 0,
                },
            },
        };
        let mut input_view: Option<ID3D11VideoProcessorInputView> = None;
        unsafe {
            self.video_device
                .CreateVideoProcessorInputView(
                    src,
                    &self.enumerator,
                    &input_desc,
                    Some(&mut input_view),
                )
                .map_err(|e| hr("CreateVideoProcessorInputView", e))?;
        }
        let input_view = input_view.ok_or_else(|| {
            StreamError::Capture("CreateVideoProcessorInputView returned nothing".into())
        })?;
        let (out_tex, out_view) = &self.ring[idx];
        // WHY: `pInputSurface` is a `ManuallyDrop`, so the reference it holds is
        // never released on its own — the view is dropped by hand after the blt
        // or it leaks one COM reference per captured frame.
        let mut streams = [D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            OutputIndex: 0,
            InputFrameOrField: 0,
            PastFrames: 0,
            FutureFrames: 0,
            ppPastSurfaces: std::ptr::null_mut(),
            pInputSurface: std::mem::ManuallyDrop::new(Some(input_view)),
            ppFutureSurfaces: std::ptr::null_mut(),
            ppPastSurfacesRight: std::ptr::null_mut(),
            pInputSurfaceRight: std::mem::ManuallyDrop::new(None),
            ppFutureSurfacesRight: std::ptr::null_mut(),
        }];
        let result = unsafe {
            self.video_context
                .VideoProcessorBlt(&self.processor, out_view, 0, &streams)
        };
        unsafe {
            std::mem::ManuallyDrop::drop(&mut streams[0].pInputSurface);
            std::mem::ManuallyDrop::drop(&mut streams[0].pInputSurfaceRight);
        }
        result.map_err(|e| hr("VideoProcessorBlt", e))?;
        Ok(out_tex.clone())
    }

    /// CPU copy of the last converted frame. Only the raw publish path (and the
    /// system-memory Media Foundation fallback) needs this; the D3D-aware
    /// encoder consumes the texture directly.
    pub fn read_back(&mut self, nv12: &ID3D11Texture2D) -> Result<Nv12Cpu, StreamError> {
        if self.staging.is_none() {
            self.staging = Some(self.gpu.texture(
                self.dst_width,
                self.dst_height,
                DXGI_FORMAT_NV12,
                true,
                None,
                0,
            )?);
        }
        let staging = self
            .staging
            .as_ref()
            .ok_or_else(|| StreamError::Capture("no staging texture".into()))?;
        let ctx = self.gpu.context();
        unsafe { ctx.CopyResource(staging, nv12) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            ctx.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|e| hr("Map(NV12 staging)", e))?;
        }
        let stride = mapped.RowPitch;
        let rows = self.dst_height as usize + self.dst_height as usize / 2;
        let len = stride as usize * rows;
        let mut data = vec![0u8; len];
        // WHY: D3D11 maps NV12 as one allocation — the UV plane starts exactly
        // `RowPitch * Height` bytes in and shares the Y row pitch. There is no
        // per-plane pitch to query for a mapped NV12 staging resource.
        unsafe {
            std::ptr::copy_nonoverlapping(mapped.pData as *const u8, data.as_mut_ptr(), len);
            ctx.Unmap(staging, 0);
        }
        Ok(Nv12Cpu {
            data,
            stride_y: stride,
            stride_uv: stride,
            width: self.dst_width,
            height: self.dst_height,
        })
    }
}
