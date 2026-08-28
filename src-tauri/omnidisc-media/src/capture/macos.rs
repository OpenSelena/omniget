use super::{unix_micros, AudioSink, CaptureApi, CaptureGeometry, CaptureOptions, VideoSink, VideoTick};
use crate::stream::{resolve_geometry, AudioApp, AudioMode, SourceId, StreamError, StreamSource, StreamSources};
use base64::Engine;
use screencapturekit::prelude::*;
use screencapturekit::screenshot_manager::{CGImageExt, SCScreenshotManager};
use screencapturekit::shareable_content::SCShareableContentInfo;
use screencapturekit::CVPixelBuffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const FOURCC_420V: u32 = 0x3432_3076;

pub struct CapturedFrame {
    pub pixel_buffer: CVPixelBuffer,
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

    pub fn as_raw(&self) -> *mut std::ffi::c_void {
        self.pixel_buffer.as_ptr()
    }
}

type CFRef = *const std::ffi::c_void;

extern "C" {
    fn CMSampleBufferGetSampleAttachmentsArray(sbuf: CFRef, create: u8) -> CFRef;
    fn CFArrayGetCount(arr: CFRef) -> isize;
    fn CFArrayGetValueAtIndex(arr: CFRef, idx: isize) -> CFRef;
    fn CFStringCreateWithCString(alloc: CFRef, s: *const u8, encoding: u32) -> CFRef;
    fn CFDictionaryGetValue(dict: CFRef, key: CFRef) -> CFRef;
    fn CFNumberGetValue(num: CFRef, ty: isize, out: *mut std::ffi::c_void) -> u8;
    fn CFRelease(cf: CFRef);
}

// The crate's `frame_status()` always returns None (it casts an NSNumber to an
// enum), so the attachment is read by hand: 0 = Complete, 1 = Idle.
fn frame_status(sample: &CMSampleBuffer) -> Option<i64> {
    unsafe {
        let arr = CMSampleBufferGetSampleAttachmentsArray(sample.as_ptr(), 0);
        if arr.is_null() || CFArrayGetCount(arr) == 0 {
            return None;
        }
        let dict = CFArrayGetValueAtIndex(arr, 0);
        let key = CFStringCreateWithCString(std::ptr::null(), b"SCStreamUpdateFrameStatus\0".as_ptr(), 0x0800_0100);
        let val = CFDictionaryGetValue(dict, key);
        CFRelease(key);
        if val.is_null() {
            return None;
        }
        let mut out: i64 = 0;
        if CFNumberGetValue(val, 4, (&mut out as *mut i64).cast()) != 0 {
            Some(out)
        } else {
            None
        }
    }
}

fn map_sc_error(e: SCError) -> StreamError {
    let text = e.to_string();
    let lower = text.to_lowercase();
    if matches!(e, SCError::PermissionDenied(_))
        || e.stream_error_code() == Some(screencapturekit::utils::error::SCStreamErrorCode::UserDeclined)
        || lower.contains("declined")
        || lower.contains("permission")
        || lower.contains("-3801")
    {
        StreamError::Permission
    } else {
        StreamError::Capture(text)
    }
}

fn shareable() -> Result<SCShareableContent, StreamError> {
    SCShareableContent::create()
        .with_on_screen_windows_only(true)
        .with_exclude_desktop_windows(true)
        .get()
        .map_err(map_sc_error)
}

fn own_app(content: &SCShareableContent) -> Option<SCRunningApplication> {
    let pid = std::process::id() as i32;
    content.applications().into_iter().find(|a| a.process_id() == pid)
}

fn thumbnail(filter: &SCContentFilter, src_w: u32, src_h: u32) -> Option<String> {
    let w = 320u32;
    let h = ((src_h as u64 * w as u64) / src_w.max(1) as u64).clamp(2, 320) as u32;
    let config = SCStreamConfiguration::new().with_width(w).with_height(h).with_shows_cursor(false);
    let image = SCScreenshotManager::capture_image(filter, &config).ok()?;
    let rgba = image.rgba_data().ok()?;
    let (iw, ih) = (image.width() as u32, image.height() as u32);
    if rgba.len() < (iw * ih * 4) as usize {
        return None;
    }
    let mut rgb = Vec::with_capacity((iw * ih * 3) as usize);
    for px in rgba.chunks_exact(4).take((iw * ih) as usize) {
        rgb.extend_from_slice(&px[..3]);
    }
    let mut jpeg = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 60);
    enc.encode(&rgb, iw, ih, image::ExtendedColorType::Rgb8).ok()?;
    Some(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg)))
}

fn display_source(content: &SCShareableContent, d: &SCDisplay, index: usize, thumbs: bool) -> StreamSource {
    let (w, h) = (d.width(), d.height());
    let thumb = if thumbs {
        let filter = match own_app(content) {
            Some(me) => SCContentFilter::create().with_display(d).with_excluding_applications(&[&me], &[]).build(),
            None => SCContentFilter::create().with_display(d).with_excluding_windows(&[]).build(),
        };
        thumbnail(&filter, w, h)
    } else {
        None
    };
    StreamSource {
        id: SourceId::Display { id: d.display_id() },
        title: format!("Display {} ({}×{})", index + 1, w, h),
        app_name: None,
        width: w,
        height: h,
        thumbnail: thumb,
    }
}

fn window_source(w: &SCWindow, thumbs: bool) -> Option<StreamSource> {
    if !w.is_on_screen() || w.window_layer() != 0 {
        return None;
    }
    let frame = w.frame();
    if frame.size.width < 64.0 || frame.size.height < 64.0 {
        return None;
    }
    let title = w.title().unwrap_or_default();
    let app = w.owning_application();
    if app.as_ref().map(|a| a.process_id() == std::process::id() as i32).unwrap_or(false) {
        return None;
    }
    let app_name = app.map(|a| a.application_name()).filter(|n| !n.is_empty());
    if title.trim().is_empty() && app_name.is_none() {
        return None;
    }
    let (pw, ph) = (frame.size.width as u32, frame.size.height as u32);
    let thumb = if thumbs {
        let filter = SCContentFilter::create().with_window(w).build();
        thumbnail(&filter, pw, ph)
    } else {
        None
    };
    Some(StreamSource {
        id: SourceId::Window { id: w.window_id() },
        title: if title.trim().is_empty() { app_name.clone().unwrap_or_default() } else { title },
        app_name,
        width: pw,
        height: ph,
        thumbnail: thumb,
    })
}

pub struct VideoCapture {
    stream: Option<SCStream>,
    _synthetic: Option<SyntheticSource>,
}

impl VideoCapture {
    pub fn stop(self) {
        if let Some(s) = self.stream {
            if let Err(e) = s.stop_capture() {
                tracing::debug!("[omnidisc-media] stop_capture: {e}");
            }
        }
    }
}

pub struct AudioCapture {
    stream: Option<SCStream>,
}

impl AudioCapture {
    pub fn stop(self) {
        if let Some(s) = self.stream {
            if let Err(e) = s.stop_capture() {
                tracing::debug!("[omnidisc-media] audio stop_capture: {e}");
            }
        }
    }
}

fn video_handler(sink: VideoSink) -> impl Fn(CMSampleBuffer, SCStreamOutputType) + Send + Sync + 'static {
    move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
        if of_type != SCStreamOutputType::Screen {
            return;
        }
        let status = frame_status(&sample);
        if matches!(status, Some(0) | None) {
            if let Some(pb) = sample.image_buffer() {
                let (w, h) = (pb.width() as u32, pb.height() as u32);
                sink(VideoTick::Frame(CapturedFrame { pixel_buffer: pb, width: w, height: h, capture_us: unix_micros() }));
                return;
            }
        }
        sink(VideoTick::Idle);
    }
}

fn audio_handler(sink: AudioSink) -> impl Fn(CMSampleBuffer, SCStreamOutputType) + Send + Sync + 'static {
    let scratch: std::sync::Mutex<Vec<f32>> = std::sync::Mutex::new(Vec::with_capacity(4096));
    move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
        if of_type != SCStreamOutputType::Audio {
            return;
        }
        let Some(list) = sample.audio_buffer_list() else { return };
        let n = list.num_buffers();
        if n == 0 {
            return;
        }
        let Ok(mut out) = scratch.lock() else { return };
        out.clear();
        let bufs: Vec<&[f32]> = (0..n)
            .filter_map(|i| list.buffer(i))
            .map(|b| {
                let d = b.data();
                let (_, f, _) = unsafe { d.align_to::<f32>() };
                f
            })
            .collect();
        let frames = bufs.iter().map(|b| b.len()).min().unwrap_or(0);
        if frames == 0 {
            return;
        }
        match bufs.len() {
            1 => {
                for f in 0..frames {
                    out.push(bufs[0][f]);
                    out.push(bufs[0][f]);
                }
            }
            _ => {
                for f in 0..frames {
                    out.push(bufs[0][f]);
                    out.push(bufs[1][f]);
                }
            }
        }
        sink(&out);
    }
}

struct SyntheticSource {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for SyntheticSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn start_synthetic(width: u32, height: u32, fps: u16, sink: VideoSink) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
    let (width, height) = ((width.max(2)) & !1, (height.max(2)) & !1);
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("omnidisc-synthetic".into())
        .spawn(move || {
            let interval = Duration::from_nanos(1_000_000_000u64 / fps.max(1) as u64);
            let w = width as usize;
            let h = height as usize;
            let mut pattern = vec![0u8; w + 256];
            for (i, p) in pattern.iter_mut().enumerate() {
                *p = (16 + (i % 220)) as u8;
            }
            let mut next = Instant::now();
            let mut n: u64 = 0;
            while !stop2.load(Ordering::Relaxed) {
                match CVPixelBuffer::create(w, h, FOURCC_420V) {
                    Ok(pb) => {
                        if let Ok(mut guard) = pb.lock_read_write() {
                            let y_stride = guard.bytes_per_row_of_plane(0);
                            let uv_stride = guard.bytes_per_row_of_plane(1);
                            if let (Some(y_base), Some(uv_base)) = (guard.base_address_of_plane_mut(0), guard.base_address_of_plane_mut(1)) {
                                let shift = (n % 220) as usize;
                                for row in 0..h {
                                    let off = (shift + row / 4) % 220;
                                    unsafe {
                                        std::ptr::copy_nonoverlapping(pattern.as_ptr().add(off), y_base.add(row * y_stride), w);
                                    }
                                }
                                let u_val = (128 + ((n / 2) % 64) as i32 - 32) as u8;
                                let v_val = (128 + ((n / 3) % 64) as i32 - 32) as u8;
                                for row in 0..h / 2 {
                                    let dst = unsafe { std::slice::from_raw_parts_mut(uv_base.add(row * uv_stride), w) };
                                    for (i, p) in dst.iter_mut().enumerate() {
                                        *p = if i % 2 == 0 { u_val } else { v_val };
                                    }
                                }
                            }
                        }
                        sink(VideoTick::Frame(CapturedFrame { pixel_buffer: pb, width, height, capture_us: unix_micros() }));
                    }
                    Err(e) => tracing::warn!("[omnidisc-media] synthetic CVPixelBufferCreate failed: {e}"),
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
        VideoCapture { stream: None, _synthetic: Some(SyntheticSource { stop, thread: Some(thread) }) },
        CaptureGeometry { width, height, fps },
    ))
}

pub struct Platform;

impl CaptureApi for Platform {
    fn list_sources(thumbnails: bool) -> Result<StreamSources, StreamError> {
        let content = shareable()?;
        let displays: Vec<StreamSource> =
            content.displays().iter().enumerate().map(|(i, d)| display_source(&content, d, i, thumbnails)).collect();
        if displays.is_empty() {
            return Err(StreamError::Permission);
        }
        let mut windows: Vec<StreamSource> = Vec::new();
        for w in content.windows() {
            let thumbs = thumbnails && windows.len() < 16;
            if let Some(src) = window_source(&w, thumbs) {
                windows.push(src);
            }
            if windows.len() >= 40 {
                break;
            }
        }
        let own_pid = std::process::id() as i32;
        let mut app_pids: Vec<i32> = content
            .windows()
            .iter()
            .filter(|w| w.is_on_screen() && w.window_layer() == 0)
            .filter_map(|w| w.owning_application().map(|a| a.process_id()))
            .collect();
        app_pids.sort_unstable();
        app_pids.dedup();
        let apps: Vec<AudioApp> = content
            .applications()
            .into_iter()
            .filter(|a| a.process_id() != own_pid && !a.bundle_identifier().is_empty() && app_pids.contains(&a.process_id()))
            .map(|a| AudioApp { pid: a.process_id(), name: a.application_name(), bundle_id: a.bundle_identifier() })
            .collect();
        Ok(StreamSources { displays, windows, apps, app_audio_supported: true, system_audio_supported: true })
    }

    fn start_video(opts: &CaptureOptions, sink: VideoSink) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
        if let SourceId::Synthetic { width, height } = opts.source {
            return start_synthetic(width, height, opts.fps, sink);
        }
        let content = shareable()?;
        let me = own_app(&content);
        let (filter, native_w, native_h) = match opts.source {
            SourceId::Display { id } => {
                let d = content.displays().into_iter().find(|d| d.display_id() == id).ok_or(StreamError::SourceGone)?;
                let filter = match &me {
                    Some(app) => SCContentFilter::create().with_display(&d).with_excluding_applications(&[app], &[]).build(),
                    None => SCContentFilter::create().with_display(&d).with_excluding_windows(&[]).build(),
                };
                (filter, d.width(), d.height())
            }
            SourceId::Window { id } => {
                let w = content.windows().into_iter().find(|w| w.window_id() == id).ok_or(StreamError::SourceGone)?;
                let filter = SCContentFilter::create().with_window(&w).build();
                let frame = w.frame();
                let (pw, ph) = SCShareableContentInfo::for_filter(&filter)
                    .map(|i| i.pixel_size())
                    .filter(|(a, b)| *a > 0 && *b > 0)
                    .unwrap_or((frame.size.width as u32 * 2, frame.size.height as u32 * 2));
                (filter, pw, ph)
            }
            SourceId::Synthetic { .. } => return Err(StreamError::Unsupported),
        };
        let (width, height) = resolve_geometry(native_w, native_h, opts.height);
        let config = SCStreamConfiguration::new()
            .with_width(width)
            .with_height(height)
            .with_pixel_format(PixelFormat::YCbCr_420v)
            .with_shows_cursor(opts.cursor)
            .with_queue_depth(6)
            .with_minimum_frame_interval(&CMTime::new(1, opts.fps.max(1) as i32));
        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(video_handler(sink), SCStreamOutputType::Screen);
        stream.start_capture().map_err(map_sc_error)?;
        Ok((VideoCapture { stream: Some(stream), _synthetic: None }, CaptureGeometry { width, height, fps: opts.fps }))
    }

    fn start_audio(mode: AudioMode, sink: AudioSink) -> Result<(AudioCapture, AudioMode), StreamError> {
        if mode == AudioMode::None {
            return Ok((AudioCapture { stream: None }, AudioMode::None));
        }
        let content = shareable()?;
        let Some(display) = content.displays().into_iter().next() else {
            return Err(StreamError::Permission);
        };
        let mut attempt = mode;
        loop {
            let (filter, exclude_self) = match attempt {
                AudioMode::App { pid } => match content.applications().into_iter().find(|a| a.process_id() == pid) {
                    Some(app) => {
                        (SCContentFilter::create().with_display(&display).with_including_applications(&[&app], &[]).build(), false)
                    }
                    None => {
                        tracing::warn!("[omnidisc-media] audio app pid {pid} is not shareable; falling back to system audio");
                        attempt = AudioMode::System;
                        continue;
                    }
                },
                AudioMode::System => (SCContentFilter::create().with_display(&display).with_excluding_windows(&[]).build(), true),
                AudioMode::None => return Ok((AudioCapture { stream: None }, AudioMode::None)),
            };
            let config = SCStreamConfiguration::new()
                .with_width(2)
                .with_height(2)
                .with_minimum_frame_interval(&CMTime::new(1, 1))
                .with_captures_audio(true)
                .with_sample_rate(super::AUDIO_SAMPLE_RATE as i32)
                .with_channel_count(super::AUDIO_CHANNELS as i32)
                .with_excludes_current_process_audio(exclude_self);
            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(audio_handler(sink.clone()), SCStreamOutputType::Audio);
            match stream.start_capture() {
                Ok(()) => return Ok((AudioCapture { stream: Some(stream) }, attempt)),
                Err(e) => {
                    tracing::warn!("[omnidisc-media] audio capture {:?} failed: {e}", attempt);
                    attempt = match attempt {
                        AudioMode::App { .. } => AudioMode::System,
                        _ => return Ok((AudioCapture { stream: None }, AudioMode::None)),
                    };
                }
            }
        }
    }
}
