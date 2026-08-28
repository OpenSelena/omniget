use crate::stream::{StreamError, Viewport};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub struct Planes {
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub full_range: bool,
}

#[derive(Default)]
pub struct FrameSlot {
    pub planes: Mutex<Option<Planes>>,
    pub width: AtomicU32,
    pub height: AtomicU32,
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    textures: Option<[wgpu::Texture; 3]>,
    tex_w: u32,
    tex_h: u32,
    format: wgpu::TextureFormat,
}

pub struct WgpuViewer {
    gpu: Mutex<Gpu>,
    frame: Arc<FrameSlot>,
    viewport: Mutex<Option<Viewport>>,
    stop: Arc<AtomicBool>,
    rendered: Arc<std::sync::atomic::AtomicU64>,
}

fn build_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("omnidisc-viewer"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let tex_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("omnidisc-viewer"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            tex_entry(2),
            tex_entry(3),
            tex_entry(4),
        ],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("omnidisc-viewer"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("omnidisc-viewer"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, bgl)
}

impl WgpuViewer {
    pub fn from_surface(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Arc<Self>, StreamError> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| StreamError::Viewer(format!("no gpu adapter: {e}")))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("omnidisc-viewer"),
            ..Default::default()
        }))
        .map_err(|e| StreamError::Viewer(format!("no gpu device: {e}")))?;
        let caps = surface.get_capabilities(&adapter);
        // An adapter that cannot present to this window answers with empty
        // lists instead of an error; indexing them would abort the app.
        let first_format = *caps.formats.first().ok_or_else(|| {
            StreamError::Viewer("this GPU cannot draw into the stream window".into())
        })?;
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(first_format);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: width.max(1),
            height: height.max(1),
            present_mode: caps
                .present_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::PresentMode::Fifo)
                .unwrap_or(wgpu::PresentMode::AutoVsync),
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let (pipeline, bgl) = build_pipeline(&device, format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("omnidisc-viewer-uniform"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let gpu = Gpu {
            device,
            queue,
            surface,
            config,
            pipeline,
            bgl,
            sampler,
            uniform,
            textures: None,
            tex_w: 0,
            tex_h: 0,
            format,
        };
        Ok(Arc::new(Self {
            gpu: Mutex::new(gpu),
            frame: Arc::new(FrameSlot::default()),
            viewport: Mutex::new(None),
            stop: Arc::new(AtomicBool::new(false)),
            rendered: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }))
    }

    pub fn frame_slot(&self) -> Arc<FrameSlot> {
        self.frame.clone()
    }

    pub fn rendered_frames(&self) -> u64 {
        self.rendered.load(Ordering::Relaxed)
    }

    pub fn set_viewport(&self, viewport: Option<Viewport>) {
        if let Ok(mut v) = self.viewport.lock() {
            *v = viewport;
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let Ok(mut gpu) = self.gpu.lock() {
            gpu.config.width = width.max(1);
            gpu.config.height = height.max(1);
            let (device, config) = (&gpu.device, gpu.config.clone());
            gpu.surface.configure(device, &config);
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn ensure_textures(gpu: &mut Gpu, w: u32, h: u32) {
        if gpu.textures.is_some() && gpu.tex_w == w && gpu.tex_h == h {
            return;
        }
        let make = |tw: u32, th: u32| {
            gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("omnidisc-plane"),
                size: wgpu::Extent3d {
                    width: tw.max(1),
                    height: th.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        gpu.textures = Some([make(w, h), make(w / 2, h / 2), make(w / 2, h / 2)]);
        gpu.tex_w = w;
        gpu.tex_h = h;
    }

    // Renders one frame; returns false when nothing was presented (no viewport).
    pub fn render_once(&self) -> bool {
        let viewport = match self.viewport.lock().ok().and_then(|v| *v) {
            Some(v) if v.width > 0.0 && v.height > 0.0 => v,
            _ => return false,
        };
        let mut gpu = match self.gpu.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let surf_w = viewport.surface_width.max(1);
        let surf_h = viewport.surface_height.max(1);
        if gpu.config.width != surf_w || gpu.config.height != surf_h {
            gpu.config.width = surf_w;
            gpu.config.height = surf_h;
            let (device, config) = (&gpu.device, gpu.config.clone());
            gpu.surface.configure(device, &config);
        }

        let mut full = 0.0f32;
        if let Ok(mut slot) = self.frame.planes.lock() {
            if let Some(p) = slot.take() {
                Self::ensure_textures(&mut gpu, p.width, p.height);
                if let Some(tex) = &gpu.textures {
                    let write =
                        |q: &wgpu::Queue, t: &wgpu::Texture, data: &[u8], w: u32, h: u32| {
                            q.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: t,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                data,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(w),
                                    rows_per_image: Some(h),
                                },
                                wgpu::Extent3d {
                                    width: w,
                                    height: h,
                                    depth_or_array_layers: 1,
                                },
                            );
                        };
                    write(&gpu.queue, &tex[0], &p.y, p.width, p.height);
                    write(&gpu.queue, &tex[1], &p.u, p.width / 2, p.height / 2);
                    write(&gpu.queue, &tex[2], &p.v, p.width / 2, p.height / 2);
                    self.frame.width.store(p.width, Ordering::Relaxed);
                    self.frame.height.store(p.height, Ordering::Relaxed);
                }
                full = if p.full_range { 1.0 } else { 0.0 };
            }
        }
        let vw = self.frame.width.load(Ordering::Relaxed);
        let vh = self.frame.height.load(Ordering::Relaxed);
        if gpu.textures.is_none() || vw == 0 {
            return false;
        }
        let mode = 3u32; // I420

        // fit video into viewport rect preserving aspect ratio (letterbox)
        let rx = viewport.x as f32 * viewport.scale as f32;
        let ry = viewport.y as f32 * viewport.scale as f32;
        let rw = viewport.width as f32 * viewport.scale as f32;
        let rh = viewport.height as f32 * viewport.scale as f32;
        let src_aspect = vw as f32 / vh.max(1) as f32;
        let dst_aspect = rw / rh.max(1.0);
        let (fw, fh) = if src_aspect > dst_aspect {
            (rw, rw / src_aspect)
        } else {
            (rh * src_aspect, rh)
        };
        let fx = rx + (rw - fw) / 2.0;
        let fy = ry + (rh - fh) / 2.0;
        let vid = [fx, fy, fx + fw, fy + fh];

        let bg = viewport.background;
        let mut ub = [0u8; 80];
        let write_f =
            |ub: &mut [u8], off: usize, v: f32| ub[off..off + 4].copy_from_slice(&v.to_ne_bytes());
        write_f(&mut ub, 0, gpu.config.width as f32);
        write_f(&mut ub, 4, gpu.config.height as f32);
        write_f(&mut ub, 16, rx);
        write_f(&mut ub, 20, ry);
        write_f(&mut ub, 24, rx + rw);
        write_f(&mut ub, 28, ry + rh);
        write_f(&mut ub, 32, vid[0]);
        write_f(&mut ub, 36, vid[1]);
        write_f(&mut ub, 40, vid[2]);
        write_f(&mut ub, 44, vid[3]);
        write_f(&mut ub, 48, bg[0]);
        write_f(&mut ub, 52, bg[1]);
        write_f(&mut ub, 56, bg[2]);
        write_f(&mut ub, 64, mode as f32);
        write_f(&mut ub, 68, full);
        gpu.queue.write_buffer(&gpu.uniform, 0, &ub);

        let tex = gpu.textures.as_ref().unwrap();
        let views: Vec<wgpu::TextureView> = tex
            .iter()
            .map(|t| t.create_view(&Default::default()))
            .collect();
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("omnidisc-viewer"),
            layout: &gpu.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
            ],
        });
        let surface_tex = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let (device, config) = (&gpu.device, gpu.config.clone());
                gpu.surface.configure(device, &config);
                return false;
            }
            _ => return false,
        };
        let view = surface_tex.texture.create_view(&Default::default());
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("omnidisc-viewer"),
            });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("omnidisc-viewer"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&gpu.pipeline);
            rp.set_bind_group(0, &bind, &[]);
            rp.draw(0..3, 0..1);
        }
        gpu.queue.submit([enc.finish()]);
        gpu.queue.present(surface_tex);
        self.rendered.fetch_add(1, Ordering::Relaxed);
        let _ = gpu.format;
        true
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}
