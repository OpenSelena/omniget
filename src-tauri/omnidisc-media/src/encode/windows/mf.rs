use crate::capture::{gpu, CapturedFrame, Gpu, Nv12Converter};
use crate::encode::{EncoderConfig, EncoderCounters};
use crate::stream::{StreamError, StreamMode};
use livekit::webrtc::video_frame::{EncodedFrameType, EncodedVideoCodec, EncodedVideoFrame};
use livekit::webrtc::video_source::{native::NativeVideoSource, VideoResolution};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Instant;
use windows::core::{implement, Interface, Ref};
use windows::Win32::Foundation::E_NOTIMPL;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Media::MediaFoundation::{
    eAVEncCommonRateControlMode_CBR, eAVEncH264VProfile_High, eAVEncH264VProfile_Main,
    CODECAPI_AVEncCommonMeanBitRate, CODECAPI_AVEncCommonRateControlMode, CODECAPI_AVEncMPVGOPSize,
    CODECAPI_AVEncVideoForceKeyFrame, CODECAPI_AVLowLatencyMode, ICodecAPI, IMF2DBuffer,
    IMFActivate, IMFAsyncCallback, IMFAsyncCallback_Impl, IMFAsyncResult, IMFDXGIDeviceManager,
    IMFMediaEventGenerator, IMFMediaType, IMFSample, IMFTransform, METransformHaveOutput,
    METransformNeedInput, MFCreateDXGIDeviceManager, MFCreateDXGISurfaceBuffer, MFCreateMediaType,
    MFCreateMemoryBuffer, MFCreateSample, MFMediaType_Video, MFSampleExtension_CleanPoint,
    MFStartup, MFTEnumEx, MFT_FRIENDLY_NAME_Attribute, MFVideoFormat_H264, MFVideoFormat_NV12,
    MFVideoInterlace_Progressive, MFSTARTUP_LITE, MFT_CATEGORY_VIDEO_ENCODER,
    MFT_ENUM_FLAG_HARDWARE, MFT_ENUM_FLAG_SORTANDFILTER, MFT_MESSAGE_COMMAND_FLUSH,
    MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, MFT_MESSAGE_NOTIFY_END_OF_STREAM,
    MFT_MESSAGE_NOTIFY_END_STREAMING, MFT_MESSAGE_NOTIFY_START_OF_STREAM,
    MFT_MESSAGE_SET_D3D_MANAGER, MFT_OUTPUT_DATA_BUFFER, MFT_REGISTER_TYPE_INFO, MF_LOW_LATENCY,
    MF_MT_AVG_BITRATE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE,
    MF_MT_MPEG2_PROFILE, MF_MT_MPEG_SEQUENCE_HEADER, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE,
    MF_SA_D3D11_AWARE, MF_TRANSFORM_ASYNC, MF_TRANSFORM_ASYNC_UNLOCK, MF_VERSION,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::{VARIANT, VARIANT_0_0, VT_BOOL, VT_UI4};

const KEYFRAME_INTERVAL: u32 = 240;
const QUEUE_DEPTH: usize = 2;

fn enc_err(what: &str, e: windows::core::Error) -> StreamError {
    StreamError::Encoder(format!("{what}: {e}"))
}

fn mf_startup() -> Result<(), StreamError> {
    static STARTED: OnceLock<Result<(), String>> = OnceLock::new();
    STARTED
        .get_or_init(|| unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_LITE).map_err(|e| format!("MFStartup: {e}"))
        })
        .clone()
        .map_err(StreamError::Encoder)
}

fn var_u32(value: u32) -> VARIANT {
    let mut v = VARIANT::default();
    unsafe {
        let inner: &mut VARIANT_0_0 = &mut v.Anonymous.Anonymous;
        inner.vt = VT_UI4;
        inner.Anonymous.ulVal = value;
    }
    v
}

fn var_bool(value: bool) -> VARIANT {
    let mut v = VARIANT::default();
    unsafe {
        let inner: &mut VARIANT_0_0 = &mut v.Anonymous.Anonymous;
        inner.vt = VT_BOOL;
        inner.Anonymous.boolVal =
            windows::Win32::Foundation::VARIANT_BOOL(if value { -1 } else { 0 });
    }
    v
}

fn pack2(a: u32, b: u32) -> u64 {
    ((a as u64) << 32) | b as u64
}

struct Pending {
    hungry: usize,
    queue: VecDeque<(IMFSample, Instant)>,
}

struct Shared {
    transform: IMFTransform,
    generator: IMFMediaEventGenerator,
    source: NativeVideoSource,
    counters: Arc<EncoderCounters>,
    width: u32,
    height: u32,
    io: Mutex<()>,
    pending: Mutex<Pending>,
    callback: Mutex<Option<IMFAsyncCallback>>,
    seq_header: Mutex<Vec<u8>>,
    stopping: AtomicBool,
    in_flight: Mutex<VecDeque<Instant>>,
}

// SAFETY: every call into the MFT and its event generator goes through the
// `io` mutex or is itself thread-safe (`IMFMediaEventGenerator` is documented
// free-threaded), and the LiveKit source is `Send + Sync`.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

#[implement(IMFAsyncCallback)]
struct EventCallback {
    shared: Weak<Shared>,
}

impl IMFAsyncCallback_Impl for EventCallback_Impl {
    fn GetParameters(&self, _flags: *mut u32, _queue: *mut u32) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Invoke(&self, result: Ref<'_, IMFAsyncResult>) -> windows::core::Result<()> {
        let Some(shared) = self.shared.upgrade() else {
            return Ok(());
        };
        let Ok(result) = result.ok() else {
            return Ok(());
        };
        let event = match unsafe { shared.generator.EndGetEvent(result) } {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };
        let kind = unsafe { event.GetType() }.unwrap_or(0);
        if kind == METransformNeedInput.0 as u32 {
            shared.on_need_input();
        } else if kind == METransformHaveOutput.0 as u32 {
            shared.on_have_output();
        }
        if !shared.stopping.load(Ordering::Acquire) {
            let cb = shared.callback.lock().ok().and_then(|g| g.clone());
            if let Some(cb) = cb {
                let _ = unsafe { shared.generator.BeginGetEvent(&cb, None) };
            }
        }
        Ok(())
    }
}

impl Shared {
    fn on_need_input(&self) {
        let next = {
            let Ok(mut pending) = self.pending.lock() else {
                return;
            };
            match pending.queue.pop_front() {
                Some(item) => Some(item),
                None => {
                    pending.hungry += 1;
                    None
                }
            }
        };
        if let Some((sample, submitted)) = next {
            self.submit(sample, submitted);
        }
    }

    fn submit(&self, sample: IMFSample, submitted: Instant) {
        let _guard = self.io.lock();
        if unsafe { self.transform.ProcessInput(0, &sample, 0) }.is_err() {
            self.counters.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.counters.submitted.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut q) = self.in_flight.lock() {
            while q.len() > 8 {
                q.pop_front();
            }
            q.push_back(submitted);
        }
    }

    fn on_have_output(&self) {
        let mut buffers = [MFT_OUTPUT_DATA_BUFFER {
            dwStreamID: 0,
            pSample: std::mem::ManuallyDrop::new(None),
            dwStatus: 0,
            pEvents: std::mem::ManuallyDrop::new(None),
        }];
        let mut status: u32 = 0;
        let ok = {
            let _guard = self.io.lock();
            unsafe { self.transform.ProcessOutput(0, &mut buffers, &mut status) }
        };
        let sample = std::mem::ManuallyDrop::into_inner(std::mem::replace(
            &mut buffers[0].pSample,
            std::mem::ManuallyDrop::new(None),
        ));
        let _events = std::mem::ManuallyDrop::into_inner(std::mem::replace(
            &mut buffers[0].pEvents,
            std::mem::ManuallyDrop::new(None),
        ));
        if ok.is_err() {
            self.counters.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let Some(sample) = sample else {
            return;
        };
        if let Err(e) = self.publish(&sample) {
            tracing::debug!("[omnidisc-media] mf output: {e}");
            self.counters.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn publish(&self, sample: &IMFSample) -> Result<(), StreamError> {
        let keyframe = unsafe { sample.GetUINT32(&MFSampleExtension_CleanPoint) }.unwrap_or(0) != 0;
        let timestamp_us = unsafe { sample.GetSampleTime() }.unwrap_or(0) / 10;
        let buffer = unsafe { sample.ConvertToContiguousBuffer() }
            .map_err(|e| enc_err("ConvertToContiguousBuffer", e))?;
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut max_len: u32 = 0;
        let mut len: u32 = 0;
        unsafe {
            buffer
                .Lock(&mut data, Some(&mut max_len), Some(&mut len))
                .map_err(|e| enc_err("IMFMediaBuffer::Lock", e))?;
        }
        let mut payload: Vec<u8> = Vec::with_capacity(len as usize + 128);
        if keyframe {
            if let Ok(header) = self.seq_header.lock() {
                let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };
                // Hardware MFTs usually inline SPS/PPS ahead of every IDR; the
                // ones that do not leave them only in MF_MT_MPEG_SEQUENCE_HEADER,
                // and libwebrtc's packetizer needs them in the access unit.
                if !header.is_empty() && !starts_with_parameter_set(slice) {
                    payload.extend_from_slice(header.as_slice());
                }
            }
        }
        unsafe {
            payload.extend_from_slice(std::slice::from_raw_parts(data, len as usize));
            let _ = buffer.Unlock();
        }
        if payload.is_empty() {
            return Ok(());
        }
        self.counters.encoded.fetch_add(1, Ordering::Relaxed);
        self.counters
            .bytes
            .fetch_add(payload.len() as u64, Ordering::Relaxed);
        // Low-latency H.264 without B-frames leaves the MFT in submission
        // order, so the oldest outstanding submit time belongs to this frame.
        let latency = self
            .in_flight
            .lock()
            .ok()
            .and_then(|mut q| q.pop_front())
            .map(|t| t.elapsed().as_nanos() as u64)
            .unwrap_or(0);
        self.counters
            .latency_ns_sum
            .fetch_add(latency, Ordering::Relaxed);
        self.counters
            .latency_ns_max
            .fetch_max(latency, Ordering::Relaxed);
        if keyframe {
            self.counters.keyframes.fetch_add(1, Ordering::Relaxed);
        }
        let frame = EncodedVideoFrame {
            codec: EncodedVideoCodec::H264,
            payload: &payload,
            timestamp_us,
            frame_type: if keyframe {
                EncodedFrameType::Key
            } else {
                EncodedFrameType::Delta
            },
            resolution: VideoResolution {
                width: self.width,
                height: self.height,
            },
            frame_metadata: None,
        };
        if self.source.capture_encoded_frame(&frame) {
            self.counters.captured_ok.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters
                .captured_rejected
                .fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

fn starts_with_parameter_set(data: &[u8]) -> bool {
    let nal = if data.starts_with(&[0, 0, 0, 1]) {
        data.get(4)
    } else if data.starts_with(&[0, 0, 1]) {
        data.get(3)
    } else {
        return false;
    };
    matches!(nal.map(|b| b & 0x1f), Some(7) | Some(8))
}

struct HardwareMft {
    transform: IMFTransform,
    name: String,
}

fn enumerate_hardware_h264() -> Result<Vec<HardwareMft>, StreamError> {
    mf_startup()?;
    let output = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };
    let mut list: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count: u32 = 0;
    unsafe {
        MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            None,
            Some(&output),
            &mut list,
            &mut count,
        )
        .map_err(|e| enc_err("MFTEnumEx", e))?;
    }
    let mut found = Vec::new();
    for i in 0..count as isize {
        let activate = unsafe { (*list.offset(i)).clone() };
        let Some(activate) = activate else { continue };
        let name = read_friendly_name(&activate);
        if let Ok(transform) = unsafe { activate.ActivateObject::<IMFTransform>() } {
            found.push(HardwareMft { transform, name });
        }
    }
    if !list.is_null() {
        unsafe { CoTaskMemFree(Some(list as *const std::ffi::c_void)) };
    }
    Ok(found)
}

fn read_friendly_name(activate: &IMFActivate) -> String {
    let mut ptr = windows::core::PWSTR::null();
    let mut len: u32 = 0;
    let ok =
        unsafe { activate.GetAllocatedString(&MFT_FRIENDLY_NAME_Attribute, &mut ptr, &mut len) };
    if ok.is_err() || ptr.is_null() {
        return "Media Foundation".into();
    }
    let name = unsafe { ptr.to_string() }.unwrap_or_else(|_| "Media Foundation".into());
    unsafe { CoTaskMemFree(Some(ptr.0 as *const std::ffi::c_void)) };
    name
}

/// True when at least one vendor-neutral hardware H.264 MFT (Intel QSV, AMD
/// VCN or NVIDIA through Media Foundation) can be activated on this machine.
pub fn hardware_h264_available() -> bool {
    match enumerate_hardware_h264() {
        Ok(list) => !list.is_empty(),
        Err(_) => false,
    }
}

pub struct MfEncoder {
    shared: Arc<Shared>,
    codec_api: Option<CodecApi>,
    converter: Mutex<Option<Nv12Converter>>,
    gpu: Arc<Gpu>,
    dxgi_manager: Option<DxgiManager>,
    cfg: EncoderConfig,
    name: String,
    finished: AtomicBool,
}

struct CodecApi(ICodecAPI);
// SAFETY: ICodecAPI on an encoder MFT is free-threaded; every use here is a
// simple property set.
unsafe impl Send for CodecApi {}
unsafe impl Sync for CodecApi {}

struct DxgiManager(#[allow(dead_code)] IMFDXGIDeviceManager);
// SAFETY: the manager is only held to keep the D3D device alive for the MFT.
unsafe impl Send for DxgiManager {}
unsafe impl Sync for DxgiManager {}

impl MfEncoder {
    pub fn new(
        cfg: EncoderConfig,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        let gpu = gpu().map_err(|e| StreamError::Encoder(e.to_string()))?;
        let candidates = enumerate_hardware_h264()?;
        if candidates.is_empty() {
            return Err(StreamError::Encoder(
                "no hardware H.264 encoder is registered with Media Foundation".into(),
            ));
        }
        let mut last: Option<StreamError> = None;
        for candidate in candidates {
            match Self::configure(&candidate, &cfg, &gpu, source.clone(), counters.clone()) {
                Ok(enc) => return Ok(enc),
                Err(e) => {
                    tracing::debug!(
                        "[omnidisc-media] hardware encoder '{}' unusable: {e}",
                        candidate.name
                    );
                    last = Some(e);
                }
            }
        }
        Err(last.unwrap_or_else(|| {
            StreamError::Encoder("no hardware H.264 encoder could be configured".into())
        }))
    }

    fn configure(
        mft: &HardwareMft,
        cfg: &EncoderConfig,
        gpu: &Arc<Gpu>,
        source: NativeVideoSource,
        counters: Arc<EncoderCounters>,
    ) -> Result<Self, StreamError> {
        let transform = mft.transform.clone();
        let attributes = unsafe { transform.GetAttributes() }
            .map_err(|e| enc_err("IMFTransform::GetAttributes", e))?;
        let is_async = unsafe { attributes.GetUINT32(&MF_TRANSFORM_ASYNC) }.unwrap_or(0) == 1;
        if is_async {
            unsafe {
                attributes
                    .SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)
                    .map_err(|e| enc_err("MF_TRANSFORM_ASYNC_UNLOCK", e))?;
            }
        }
        let _ = unsafe { attributes.SetUINT32(&MF_LOW_LATENCY, 1) };

        let d3d_aware = unsafe { attributes.GetUINT32(&MF_SA_D3D11_AWARE) }.unwrap_or(0) == 1;
        let mut dxgi_manager = None;
        if d3d_aware {
            match Self::attach_d3d(&transform, gpu) {
                Ok(m) => dxgi_manager = Some(DxgiManager(m)),
                Err(e) => tracing::debug!("[omnidisc-media] D3D11 hand-off refused: {e}"),
            }
        }

        let bps = cfg.bitrate_kbps.max(1) * 1000;
        let out_type =
            unsafe { MFCreateMediaType() }.map_err(|e| enc_err("MFCreateMediaType", e))?;
        unsafe {
            out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).ok();
            out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).ok();
            out_type.SetUINT32(&MF_MT_AVG_BITRATE, bps).ok();
            out_type
                .SetUINT64(&MF_MT_FRAME_SIZE, pack2(cfg.width, cfg.height))
                .ok();
            out_type
                .SetUINT64(&MF_MT_FRAME_RATE, pack2(cfg.fps.max(1) as u32, 1))
                .ok();
            out_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .ok();
            out_type
                .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack2(1, 1))
                .ok();
            out_type
                .SetUINT32(
                    &MF_MT_MPEG2_PROFILE,
                    if cfg.mode == StreamMode::Game {
                        eAVEncH264VProfile_Main.0 as u32
                    } else {
                        eAVEncH264VProfile_High.0 as u32
                    },
                )
                .ok();
            transform
                .SetOutputType(0, &out_type, 0)
                .map_err(|e| enc_err("SetOutputType(H264)", e))?;
        }

        let in_type =
            unsafe { MFCreateMediaType() }.map_err(|e| enc_err("MFCreateMediaType", e))?;
        unsafe {
            in_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).ok();
            in_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12).ok();
            in_type
                .SetUINT64(&MF_MT_FRAME_SIZE, pack2(cfg.width, cfg.height))
                .ok();
            in_type
                .SetUINT64(&MF_MT_FRAME_RATE, pack2(cfg.fps.max(1) as u32, 1))
                .ok();
            in_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .ok();
            in_type
                .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack2(1, 1))
                .ok();
            transform
                .SetInputType(0, &in_type, 0)
                .map_err(|e| enc_err("SetInputType(NV12)", e))?;
        }

        let codec_api = transform.cast::<ICodecAPI>().ok().map(CodecApi);
        if let Some(api) = codec_api.as_ref() {
            unsafe {
                let _ = api.0.SetValue(&CODECAPI_AVLowLatencyMode, &var_bool(true));
                let _ = api.0.SetValue(
                    &CODECAPI_AVEncCommonRateControlMode,
                    &var_u32(eAVEncCommonRateControlMode_CBR.0 as u32),
                );
                let _ = api
                    .0
                    .SetValue(&CODECAPI_AVEncCommonMeanBitRate, &var_u32(bps));
                let _ = api
                    .0
                    .SetValue(&CODECAPI_AVEncMPVGOPSize, &var_u32(KEYFRAME_INTERVAL));
            }
        }

        let provides_samples = unsafe { transform.GetOutputStreamInfo(0) }
            .map(|info| {
                info.dwFlags
                    & windows::Win32::Media::MediaFoundation::MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0
                        as u32
                    != 0
            })
            .unwrap_or(false);

        if !provides_samples {
            return Err(StreamError::Encoder(format!(
                "the '{}' encoder wants the caller to allocate output samples, which this build does not do",
                mft.name
            )));
        }
        let seq_header = read_sequence_header(&transform);

        let generator = transform
            .cast::<IMFMediaEventGenerator>()
            .map_err(|e| enc_err("IMFMediaEventGenerator (encoder is not asynchronous)", e))?;

        unsafe {
            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)
                .map_err(|e| enc_err("NOTIFY_BEGIN_STREAMING", e))?;
            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)
                .map_err(|e| enc_err("NOTIFY_START_OF_STREAM", e))?;
        }

        let shared = Arc::new(Shared {
            transform,
            generator,
            source,
            counters: counters.clone(),
            width: cfg.width,
            height: cfg.height,
            io: Mutex::new(()),
            pending: Mutex::new(Pending {
                hungry: 0,
                queue: VecDeque::with_capacity(QUEUE_DEPTH),
            }),
            callback: Mutex::new(None),
            seq_header: Mutex::new(seq_header),
            stopping: AtomicBool::new(false),
            in_flight: Mutex::new(VecDeque::with_capacity(8)),
        });
        let callback: IMFAsyncCallback = EventCallback {
            shared: Arc::downgrade(&shared),
        }
        .into();
        if let Ok(mut slot) = shared.callback.lock() {
            *slot = Some(callback.clone());
        }
        unsafe {
            shared
                .generator
                .BeginGetEvent(&callback, None)
                .map_err(|e| enc_err("BeginGetEvent", e))?;
        }
        counters.applied_bps.store(bps as u64, Ordering::Relaxed);
        Ok(Self {
            shared,
            codec_api,
            converter: Mutex::new(None),
            gpu: gpu.clone(),
            dxgi_manager,
            cfg: *cfg,
            name: format!("Media Foundation ({})", mft.name),
            finished: AtomicBool::new(false),
        })
    }

    fn attach_d3d(
        transform: &IMFTransform,
        gpu: &Arc<Gpu>,
    ) -> Result<IMFDXGIDeviceManager, StreamError> {
        let mut token: u32 = 0;
        let mut manager: Option<IMFDXGIDeviceManager> = None;
        unsafe {
            MFCreateDXGIDeviceManager(&mut token, &mut manager)
                .map_err(|e| enc_err("MFCreateDXGIDeviceManager", e))?;
        }
        let manager =
            manager.ok_or_else(|| StreamError::Encoder("no DXGI device manager".into()))?;
        unsafe {
            manager
                .ResetDevice(gpu.device(), token)
                .map_err(|e| enc_err("IMFDXGIDeviceManager::ResetDevice", e))?;
            transform
                .ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager.as_raw() as usize)
                .map_err(|e| enc_err("MFT_MESSAGE_SET_D3D_MANAGER", e))?;
        }
        Ok(manager)
    }

    fn sample_from(
        &self,
        nv12: &ID3D11Texture2D,
        capture_us: i64,
    ) -> Result<IMFSample, StreamError> {
        let sample = unsafe { MFCreateSample() }.map_err(|e| enc_err("MFCreateSample", e))?;
        let buffer = if self.dxgi_manager.is_some() {
            let buffer = unsafe {
                MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, nv12, 0, false)
                    .map_err(|e| enc_err("MFCreateDXGISurfaceBuffer", e))?
            };
            let len = buffer
                .cast::<IMF2DBuffer>()
                .ok()
                .and_then(|b| unsafe { b.GetContiguousLength() }.ok())
                .unwrap_or(self.cfg.width * self.cfg.height * 3 / 2);
            unsafe {
                let _ = buffer.SetCurrentLength(len);
            }
            buffer
        } else {
            let mut converter = self
                .converter
                .lock()
                .map_err(|_| StreamError::Encoder("converter poisoned".into()))?;
            let cpu = converter
                .as_mut()
                .ok_or_else(|| StreamError::Encoder("no converter".into()))?
                .read_back(nv12)?;
            let tight = (self.cfg.width * self.cfg.height * 3 / 2) as usize;
            let buffer = unsafe { MFCreateMemoryBuffer(tight as u32) }
                .map_err(|e| enc_err("MFCreateMemoryBuffer", e))?;
            let mut data: *mut u8 = std::ptr::null_mut();
            unsafe {
                buffer
                    .Lock(&mut data, None, None)
                    .map_err(|e| enc_err("IMFMediaBuffer::Lock", e))?;
                let (y, uv) = cpu.planes();
                for row in 0..self.cfg.height as usize {
                    let src = row * cpu.stride_y as usize;
                    let dst = row * self.cfg.width as usize;
                    if src + self.cfg.width as usize <= y.len() {
                        std::ptr::copy_nonoverlapping(
                            y.as_ptr().add(src),
                            data.add(dst),
                            self.cfg.width as usize,
                        );
                    }
                }
                let uv_base = (self.cfg.width * self.cfg.height) as usize;
                for row in 0..(self.cfg.height / 2) as usize {
                    let src = row * cpu.stride_uv as usize;
                    let dst = uv_base + row * self.cfg.width as usize;
                    if src + self.cfg.width as usize <= uv.len() {
                        std::ptr::copy_nonoverlapping(
                            uv.as_ptr().add(src),
                            data.add(dst),
                            self.cfg.width as usize,
                        );
                    }
                }
                let _ = buffer.Unlock();
                let _ = buffer.SetCurrentLength(tight as u32);
            }
            buffer
        };
        unsafe {
            sample
                .AddBuffer(&buffer)
                .map_err(|e| enc_err("IMFSample::AddBuffer", e))?;
            let _ = sample.SetSampleTime(capture_us * 10);
            let _ = sample.SetSampleDuration(10_000_000i64 / self.cfg.fps.max(1) as i64);
        }
        Ok(sample)
    }

    pub fn encode(&self, frame: &CapturedFrame, force_key: bool) -> Result<(), StreamError> {
        let Some(texture) = frame.texture() else {
            return Err(StreamError::Encoder("captured frame has no texture".into()));
        };
        let nv12 = {
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
            slot.as_mut()
                .ok_or_else(|| StreamError::Encoder("no converter".into()))?
                .convert(texture)?
        };
        if force_key {
            if let Some(api) = self.codec_api.as_ref() {
                unsafe {
                    let _ = api
                        .0
                        .SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &var_u32(1));
                }
            }
        }
        let sample = self.sample_from(&nv12, frame.capture_us())?;
        let submitted = Instant::now();
        let hungry = {
            let Ok(mut pending) = self.shared.pending.lock() else {
                return Err(StreamError::Encoder("encoder queue poisoned".into()));
            };
            if pending.hungry > 0 {
                pending.hungry -= 1;
                true
            } else {
                while pending.queue.len() >= QUEUE_DEPTH {
                    pending.queue.pop_front();
                    self.shared.counters.dropped.fetch_add(1, Ordering::Relaxed);
                }
                pending.queue.push_back((sample.clone(), submitted));
                false
            }
        };
        if hungry {
            self.shared.submit(sample, submitted);
        }
        Ok(())
    }

    pub fn set_bitrate(&self, bps: u64) {
        if let Some(api) = self.codec_api.as_ref() {
            unsafe {
                let _ = api
                    .0
                    .SetValue(&CODECAPI_AVEncCommonMeanBitRate, &var_u32(bps as u32));
            }
        }
        self.shared
            .counters
            .applied_bps
            .store(bps, Ordering::Relaxed);
    }

    pub fn set_framerate(&self, _fps: u16) {}

    pub fn hardware(&self) -> Option<bool> {
        Some(true)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn finish(&self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        self.shared.stopping.store(true, Ordering::Release);
        let _guard = self.shared.io.lock();
        unsafe {
            let _ = self
                .shared
                .transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
            let _ = self
                .shared
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
            let _ = self
                .shared
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
        }
        if let Ok(mut slot) = self.shared.callback.lock() {
            *slot = None;
        }
    }
}

impl Drop for MfEncoder {
    fn drop(&mut self) {
        self.finish();
    }
}

fn read_sequence_header(transform: &IMFTransform) -> Vec<u8> {
    let Ok(media_type): Result<IMFMediaType, _> = (unsafe { transform.GetOutputCurrentType(0) })
    else {
        return Vec::new();
    };
    let mut ptr: *mut u8 = std::ptr::null_mut();
    let mut len: u32 = 0;
    let ok =
        unsafe { media_type.GetAllocatedBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut ptr, &mut len) };
    if ok.is_err() || ptr.is_null() || len == 0 {
        return Vec::new();
    }
    let out = unsafe { std::slice::from_raw_parts(ptr, len as usize) }.to_vec();
    unsafe { CoTaskMemFree(Some(ptr as *const std::ffi::c_void)) };
    out
}
