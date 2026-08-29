//! The PipeWire half of Linux capture.
//!
//! PipeWire objects are not `Send`, so each stream owns a dedicated OS thread
//! running its own main loop, and the rest of the app talks to it through a
//! `pipewire::channel`. Frames are copied out of the shared buffer on that
//! thread and handed to the sink; nothing borrows PipeWire memory afterwards.

use super::portal::ScreencastSession;
use super::{downscale_bgrx, CapturedFrame};
use crate::capture::{
    unix_micros, AudioSink, CaptureGeometry, CaptureOptions, VideoSink, VideoTick,
};
use crate::stream::StreamError;
use pipewire as pw;
use pw::spa;
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils;
use spa::param::video::{VideoFormat, VideoInfoRaw};
use spa::pod::Pod;
use spa::utils::{Direction, Rectangle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

/// The portal dialog is a human in the loop; format negotiation afterwards is
/// machinery and should be quick or broken.
const NEGOTIATE_TIMEOUT: Duration = Duration::from_secs(8);

pub enum Control {
    Stop,
}

pub struct StreamHandle {
    tx: Option<pw::channel::Sender<Control>>,
    thread: Option<std::thread::JoinHandle<()>>,
    stopped: Arc<AtomicBool>,
}

impl StreamHandle {
    pub fn stop(mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Control::Stop);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Control::Stop);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct VideoState {
    info: VideoInfoRaw,
    sink: VideoSink,
    target: Option<(usize, usize)>,
    announced: bool,
    geometry_tx: Option<mpsc::Sender<CaptureGeometry>>,
}

/// Start capturing the node the portal handed us.
pub fn start_video(
    session: ScreencastSession,
    opts: &CaptureOptions,
    sink: VideoSink,
) -> Result<(StreamHandle, CaptureGeometry), StreamError> {
    let (control_tx, control_rx) = pw::channel::channel::<Control>();
    let (geometry_tx, geometry_rx) = mpsc::channel::<CaptureGeometry>();
    let (error_tx, error_rx) = mpsc::channel::<String>();
    let stopped = Arc::new(AtomicBool::new(false));
    let node_id = session.node_id;
    let fd = session.fd;
    let fps = opts.fps.max(1);
    let cap_height = opts.height.map(|h| h as usize);
    let stopped_thread = stopped.clone();

    let thread = std::thread::Builder::new()
        .name("omnidisc-pw-video".into())
        .spawn(move || {
            if let Err(e) = run_video(
                fd,
                node_id,
                fps,
                cap_height,
                sink,
                geometry_tx,
                control_rx,
                stopped_thread,
            ) {
                let _ = error_tx.send(e);
            }
        })
        .map_err(|e| StreamError::Capture(format!("pipewire thread: {e}")))?;

    match geometry_rx.recv_timeout(NEGOTIATE_TIMEOUT) {
        Ok(geometry) => Ok((
            StreamHandle {
                tx: Some(control_tx),
                thread: Some(thread),
                stopped,
            },
            geometry,
        )),
        Err(_) => {
            stopped.store(true, Ordering::Release);
            let _ = control_tx.send(Control::Stop);
            let _ = thread.join();
            let detail = error_rx
                .try_recv()
                .unwrap_or_else(|_| "the screen never produced a frame we can read".into());
            Err(StreamError::Capture(detail))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_video(
    fd: std::os::fd::OwnedFd,
    node_id: u32,
    fps: u16,
    cap_height: Option<usize>,
    sink: VideoSink,
    geometry_tx: mpsc::Sender<CaptureGeometry>,
    control_rx: pw::channel::Receiver<Control>,
    stopped: Arc<AtomicBool>,
) -> Result<(), String> {
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None).map_err(|e| e.to_string())?;
    let context = pw::context::ContextRc::new(&main_loop, None).map_err(|e| e.to_string())?;
    let core = context.connect_fd_rc(fd, None).map_err(|e| e.to_string())?;

    let props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Video",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Screen",
    };
    let stream = pw::stream::StreamRc::new(core.clone(), "omnidisc-screen", props)
        .map_err(|e| e.to_string())?;

    let state = VideoState {
        info: VideoInfoRaw::default(),
        sink,
        target: None,
        announced: false,
        geometry_tx: Some(geometry_tx),
    };

    let _listener = stream
        .add_local_listener_with_user_data(state)
        .param_changed(move |_, user, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };
            if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
                return;
            }
            if user.info.parse(param).is_err() {
                return;
            }
            let size = user.info.size();
            let (w, h) = (size.width as usize, size.height as usize);
            if w == 0 || h == 0 {
                return;
            }
            // A capped preset means publishing fewer pixels than the screen has;
            // the encoder budget should go to the resolution the viewer asked
            // for, not the one the monitor happens to be.
            let (out_w, out_h) = match cap_height {
                Some(cap) if cap < h => {
                    let scaled_w = ((w * cap / h.max(1)) + 1) & !1usize;
                    (scaled_w.max(2), cap & !1usize)
                }
                _ => (w & !1usize, h & !1usize),
            };
            user.target = Some((out_w, out_h));
            if !user.announced {
                user.announced = true;
                if let Some(tx) = user.geometry_tx.take() {
                    let _ = tx.send(CaptureGeometry {
                        width: out_w as u32,
                        height: out_h as u32,
                        fps,
                    });
                }
            }
        })
        .process(move |stream, user| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                (user.sink)(VideoTick::Idle);
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                (user.sink)(VideoTick::Idle);
                return;
            }
            let data = &mut datas[0];
            let chunk_size = data.chunk().size() as usize;
            let stride = data.chunk().stride().max(0) as usize;
            let size = user.info.size();
            let (w, h) = (size.width as usize, size.height as usize);
            let Some((out_w, out_h)) = user.target else {
                (user.sink)(VideoTick::Idle);
                return;
            };
            let Some(bytes) = data.data() else {
                (user.sink)(VideoTick::Idle);
                return;
            };
            if chunk_size == 0 || stride == 0 || w == 0 || h == 0 {
                (user.sink)(VideoTick::Idle);
                return;
            }
            let swap_rb = matches!(user.info.format(), VideoFormat::RGBx | VideoFormat::RGBA);
            let frame = if (out_w, out_h) == (w, h) {
                let mut packed = vec![0u8; w * h * 4];
                for y in 0..h {
                    let src = y * stride;
                    let dst = y * w * 4;
                    if src + w * 4 > bytes.len() {
                        break;
                    }
                    packed[dst..dst + w * 4].copy_from_slice(&bytes[src..src + w * 4]);
                }
                packed
            } else {
                downscale_bgrx(bytes, w, h, stride, out_w, out_h)
            };
            let mut frame = frame;
            if swap_rb {
                for px in frame.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
            }
            (user.sink)(VideoTick::Frame(CapturedFrame {
                data: frame,
                width: out_w as u32,
                height: out_h as u32,
                stride: out_w * 4,
                capture_us: unix_micros(),
            }));
        })
        .register()
        .map_err(|e| e.to_string())?;

    let mut info = VideoInfoRaw::default();
    info.set_format(VideoFormat::BGRx);
    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: video_format_properties(fps),
    };
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .map_err(|e| format!("building the video format: {e}"))?
    .0
    .into_inner();
    let mut params = [Pod::from_bytes(&values).ok_or("invalid video format pod")?];

    stream
        .connect(
            Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .map_err(|e| e.to_string())?;

    let quit_loop = main_loop.clone();
    let _receiver = control_rx.attach(main_loop.loop_(), move |_| {
        quit_loop.quit();
    });
    let _ = &stopped;
    main_loop.run();
    let _ = stream.disconnect();
    Ok(())
}

/// One `EnumFormat` advertising the packed 32-bit layouts every compositor can
/// hand over through shared memory. The `modifier` property is deliberately
/// absent: omitting it is what tells the server we cannot take DMA-BUF.
fn video_format_properties(fps: u16) -> Vec<spa::pod::Property> {
    use spa::param::format::{FormatProperties, MediaSubtype as Sub, MediaType as Mt};
    use spa::pod::{property, Property};
    use spa::utils::Fraction;
    let props: Vec<Property> = vec![
        property!(FormatProperties::MediaType, Id, Mt::Video),
        property!(FormatProperties::MediaSubtype, Id, Sub::Raw),
        property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::BGRx,
            VideoFormat::BGRx,
            VideoFormat::RGBx,
            VideoFormat::BGRA,
            VideoFormat::RGBA
        ),
        property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            Rectangle {
                width: 1920,
                height: 1080
            },
            Rectangle {
                width: 1,
                height: 1
            },
            Rectangle {
                width: 8192,
                height: 8192
            }
        ),
        property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            Fraction {
                num: fps as u32,
                denom: 1
            },
            Fraction { num: 0, denom: 1 },
            Fraction { num: 240, denom: 1 }
        ),
    ];
    props
}

struct AudioState {
    info: spa::param::audio::AudioInfoRaw,
    sink: AudioSink,
}

/// The default sink's monitor. The ScreenCast portal carries no audio, so this
/// is a separate stream that follows whatever the desktop is playing.
pub fn start_monitor_audio(sink: AudioSink) -> Result<StreamHandle, StreamError> {
    let (control_tx, control_rx) = pw::channel::channel::<Control>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let stopped = Arc::new(AtomicBool::new(false));
    let thread = std::thread::Builder::new()
        .name("omnidisc-pw-audio".into())
        .spawn(move || {
            if let Err(e) = run_audio(sink, control_rx, ready_tx.clone()) {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| StreamError::Capture(format!("pipewire audio thread: {e}")))?;

    match ready_rx.recv_timeout(NEGOTIATE_TIMEOUT) {
        Ok(Ok(())) => Ok(StreamHandle {
            tx: Some(control_tx),
            thread: Some(thread),
            stopped,
        }),
        Ok(Err(e)) => {
            let _ = control_tx.send(Control::Stop);
            let _ = thread.join();
            Err(StreamError::Capture(e))
        }
        Err(_) => {
            let _ = control_tx.send(Control::Stop);
            let _ = thread.join();
            Err(StreamError::Capture("no audio from the desktop".into()))
        }
    }
}

fn run_audio(
    sink: AudioSink,
    control_rx: pw::channel::Receiver<Control>,
    ready_tx: mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None).map_err(|e| e.to_string())?;
    let context = pw::context::ContextRc::new(&main_loop, None).map_err(|e| e.to_string())?;
    let core = context.connect_rc(None).map_err(|e| e.to_string())?;

    let mut props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Screen",
    };
    props.insert(*pw::keys::STREAM_CAPTURE_SINK, "true");
    let stream = pw::stream::StreamRc::new(core.clone(), "omnidisc-screen-audio", props)
        .map_err(|e| e.to_string())?;

    let state = AudioState {
        info: spa::param::audio::AudioInfoRaw::new(),
        sink,
    };
    let _listener = stream
        .add_local_listener_with_user_data(state)
        .param_changed(|_, user, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };
            if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                return;
            }
            let _ = user.info.parse(param);
        })
        .process(|stream, user| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }
            let data = &mut datas[0];
            let size = data.chunk().size() as usize;
            let Some(bytes) = data.data() else { return };
            let n = (size / std::mem::size_of::<f32>()).min(bytes.len() / 4);
            if n == 0 {
                return;
            }
            let mut samples = Vec::with_capacity(n);
            for i in 0..n {
                let b = &bytes[i * 4..i * 4 + 4];
                samples.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            }
            (user.sink)(&samples);
        })
        .register()
        .map_err(|e| e.to_string())?;

    let (rate, channels) = super::audio_format();
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(rate);
    info.set_channels(channels);
    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .map_err(|e| format!("building the audio format: {e}"))?
    .0
    .into_inner();
    let mut params = [Pod::from_bytes(&values).ok_or("invalid audio format pod")?];

    stream
        .connect(
            Direction::Input,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .map_err(|e| e.to_string())?;

    let _ = ready_tx.send(Ok(()));
    let quit_loop = main_loop.clone();
    let _receiver = control_rx.attach(main_loop.loop_(), move |_| {
        quit_loop.quit();
    });
    main_loop.run();
    let _ = stream.disconnect();
    Ok(())
}

/// A moving test pattern, so the media pipeline can be exercised on a headless
/// box (CI) where there is no compositor to ask.
pub fn start_synthetic(
    width: u32,
    height: u32,
    fps: u16,
    sink: VideoSink,
) -> Result<(StreamHandle, CaptureGeometry), StreamError> {
    let (control_tx, control_rx) = pw::channel::channel::<Control>();
    let stopped = Arc::new(AtomicBool::new(false));
    let running = stopped.clone();
    let (w, h) = ((width.max(2) & !1) as usize, (height.max(2) & !1) as usize);
    let interval = Duration::from_nanos(1_000_000_000 / fps.max(1) as u64);
    let thread = std::thread::Builder::new()
        .name("omnidisc-synthetic".into())
        .spawn(move || {
            let mut tick: usize = 0;
            while !running.load(Ordering::Acquire) {
                let mut data = vec![0u8; w * h * 4];
                for y in 0..h {
                    for x in 0..w {
                        let i = (y * w + x) * 4;
                        data[i] = ((x + tick) % 256) as u8;
                        data[i + 1] = ((y + tick) % 256) as u8;
                        data[i + 2] = ((x + y) % 256) as u8;
                        data[i + 3] = 255;
                    }
                }
                sink(VideoTick::Frame(CapturedFrame {
                    data,
                    width: w as u32,
                    height: h as u32,
                    stride: w * 4,
                    capture_us: unix_micros(),
                }));
                tick = tick.wrapping_add(1);
                std::thread::sleep(interval);
            }
        })
        .map_err(|e| StreamError::Capture(format!("synthetic thread: {e}")))?;
    drop(control_rx);
    Ok((
        StreamHandle {
            tx: Some(control_tx),
            thread: Some(thread),
            stopped,
        },
        CaptureGeometry {
            width: w as u32,
            height: h as u32,
            fps,
        },
    ))
}
