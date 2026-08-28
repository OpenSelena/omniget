use super::{EncoderConfig, EncoderCounters};
use crate::capture::CapturedFrame;
use crate::stream::{StreamCodec, StreamError, StreamMode};
use livekit::webrtc::video_frame::{EncodedFrameType, EncodedVideoCodec, EncodedVideoFrame};
use livekit::webrtc::video_source::{native::NativeVideoSource, VideoResolution};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{
    kCMSampleAttachmentKey_NotSync, kCMVideoCodecType_H264, kCMVideoCodecType_HEVC, CMSampleBuffer, CMTime, CMTimeFlags,
    CMVideoFormatDescriptionGetH264ParameterSetAtIndex, CMVideoFormatDescriptionGetHEVCParameterSetAtIndex,
};
use objc2_core_video::{
    kCVPixelBufferHeightKey, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey,
    kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, CVPixelBuffer,
};
use objc2_video_toolbox::{
    kVTCompressionPropertyKey_AllowFrameReordering, kVTCompressionPropertyKey_AverageBitRate,
    kVTCompressionPropertyKey_DataRateLimits, kVTCompressionPropertyKey_ExpectedFrameRate,
    kVTCompressionPropertyKey_H264EntropyMode, kVTCompressionPropertyKey_MaxKeyFrameInterval,
    kVTCompressionPropertyKey_PrioritizeEncodingSpeedOverQuality, kVTCompressionPropertyKey_ProfileLevel,
    kVTCompressionPropertyKey_RealTime, kVTCompressionPropertyKey_UsingHardwareAcceleratedVideoEncoder,
    kVTEncodeFrameOptionKey_ForceKeyFrame, kVTH264EntropyMode_CABAC, kVTProfileLevel_H264_High_AutoLevel,
    kVTProfileLevel_HEVC_Main_AutoLevel, kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder,
    VTCompressionSession, VTEncodeInfoFlags, VTSessionCopyProperty, VTSessionSetProperty,
};
use std::ffi::c_void;
use std::ptr::{null_mut, NonNull};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const KEYFRAME_INTERVAL: i32 = 240;

struct FrameCtx {
    submit: Instant,
    capture_us: i64,
}

struct Shared {
    codec: StreamCodec,
    width: u32,
    height: u32,
    source: NativeVideoSource,
    counters: Arc<EncoderCounters>,
    param_sets: Mutex<Vec<Vec<u8>>>,
}

fn keyframe_of(sample: &CMSampleBuffer) -> bool {
    unsafe {
        let Some(attachments) = sample.sample_attachments_array(false) else { return true };
        let arr: &CFArray = &attachments;
        if arr.len() == 0 {
            return true;
        }
        let dict_ptr = arr.value_at_index(0) as *const CFDictionary;
        if dict_ptr.is_null() {
            return true;
        }
        let dict: &CFDictionary = &*dict_ptr;
        let key: &CFString = kCMSampleAttachmentKey_NotSync;
        let v = dict.value(key as *const CFString as *const c_void);
        if v.is_null() {
            return true;
        }
        let b: &CFBoolean = &*(v as *const CFBoolean);
        !b.value()
    }
}

unsafe fn parameter_sets(shared: &Shared, sample: &CMSampleBuffer) -> (Vec<Vec<u8>>, usize) {
    let mut sets: Vec<Vec<u8>> = Vec::new();
    let mut nal_len_size = 4usize;
    let Some(fd) = sample.format_description() else { return (sets, nal_len_size) };
    let mut count: usize = 0;
    let mut idx = 0usize;
    loop {
        let mut ptr: *const u8 = std::ptr::null();
        let mut size: usize = 0;
        let mut hdr_len: std::ffi::c_int = 4;
        let st = match shared.codec {
            StreamCodec::H264 => CMVideoFormatDescriptionGetH264ParameterSetAtIndex(&fd, idx, &mut ptr, &mut size, &mut count, &mut hdr_len),
            StreamCodec::H265 => CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(&fd, idx, &mut ptr, &mut size, &mut count, &mut hdr_len),
        };
        if st != 0 || ptr.is_null() {
            break;
        }
        nal_len_size = hdr_len as usize;
        sets.push(std::slice::from_raw_parts(ptr, size).to_vec());
        idx += 1;
        if idx >= count {
            break;
        }
    }
    (sets, nal_len_size)
}

unsafe extern "C-unwind" fn vt_output_callback(
    refcon: *mut c_void,
    frame_refcon: *mut c_void,
    status: i32,
    flags: VTEncodeInfoFlags,
    sample: *mut CMSampleBuffer,
) {
    if refcon.is_null() {
        return;
    }
    let shared: &Shared = &*(refcon as *const Shared);
    let ctx: Option<Box<FrameCtx>> = if frame_refcon.is_null() { None } else { Some(Box::from_raw(frame_refcon as *mut FrameCtx)) };
    let now = Instant::now();
    if status != 0 {
        shared.counters.errors.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if flags.contains(VTEncodeInfoFlags::FrameDropped) || sample.is_null() {
        shared.counters.dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let sample: &CMSampleBuffer = &*sample;
    let capture_us = ctx.as_ref().map(|c| c.capture_us).unwrap_or(0);
    if let Some(c) = ctx.as_ref() {
        let latency = now.duration_since(c.submit).as_nanos() as u64;
        shared.counters.latency_ns_sum.fetch_add(latency, Ordering::Relaxed);
        shared.counters.latency_ns_max.fetch_max(latency, Ordering::Relaxed);
    }

    let keyframe = keyframe_of(sample);
    let Some(block) = sample.data_buffer() else {
        shared.counters.errors.fetch_add(1, Ordering::Relaxed);
        return;
    };
    let total = block.data_length();
    let mut avcc = vec![0u8; total];
    let Some(dst) = NonNull::new(avcc.as_mut_ptr() as *mut c_void) else { return };
    if block.copy_data_bytes(0, total, dst) != 0 {
        shared.counters.errors.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let mut nal_len_size = 4usize;
    if keyframe {
        let (sets, len) = parameter_sets(shared, sample);
        nal_len_size = len;
        if !sets.is_empty() {
            if let Ok(mut ps) = shared.param_sets.lock() {
                *ps = sets;
            }
        }
    }

    let mut annexb: Vec<u8> = Vec::with_capacity(total + 128);
    if keyframe {
        if let Ok(ps) = shared.param_sets.lock() {
            for s in ps.iter() {
                annexb.extend_from_slice(&[0, 0, 0, 1]);
                annexb.extend_from_slice(s);
            }
        }
    }
    let mut off = 0usize;
    while off + nal_len_size <= avcc.len() {
        let mut len = 0usize;
        for i in 0..nal_len_size {
            len = (len << 8) | avcc[off + i] as usize;
        }
        off += nal_len_size;
        if off + len > avcc.len() {
            shared.counters.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        annexb.extend_from_slice(&[0, 0, 0, 1]);
        annexb.extend_from_slice(&avcc[off..off + len]);
        off += len;
    }

    shared.counters.encoded.fetch_add(1, Ordering::Relaxed);
    shared.counters.bytes.fetch_add(annexb.len() as u64, Ordering::Relaxed);
    if keyframe {
        shared.counters.keyframes.fetch_add(1, Ordering::Relaxed);
    }
    let frame = EncodedVideoFrame {
        codec: match shared.codec {
            StreamCodec::H264 => EncodedVideoCodec::H264,
            StreamCodec::H265 => EncodedVideoCodec::H265,
        },
        payload: &annexb,
        timestamp_us: capture_us,
        frame_type: if keyframe { EncodedFrameType::Key } else { EncodedFrameType::Delta },
        resolution: VideoResolution { width: shared.width, height: shared.height },
        frame_metadata: None,
    };
    if shared.source.capture_encoded_frame(&frame) {
        shared.counters.captured_ok.fetch_add(1, Ordering::Relaxed);
    } else {
        shared.counters.captured_rejected.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct VideoEncoder {
    session: CFRetained<VTCompressionSession>,
    shared: Arc<Shared>,
    hw: Option<bool>,
    start: Instant,
    counters: Arc<EncoderCounters>,
    finished: AtomicBool,
}

unsafe impl Send for VideoEncoder {}
unsafe impl Sync for VideoEncoder {}

fn set_prop(session: &VTCompressionSession, key: &CFString, value: &CFType) -> i32 {
    unsafe { VTSessionSetProperty(session, key, Some(value)) }
}

fn data_rate_limits(bps: u64) -> CFRetained<CFArray<CFNumber>> {
    let bytes_per_sec = CFNumber::new_i64((bps / 8 * 3 / 2) as i64);
    let one_sec = CFNumber::new_f64(1.0);
    CFArray::<CFNumber>::from_retained_objects(&[bytes_per_sec, one_sec])
}

impl VideoEncoder {
    pub fn new(cfg: EncoderConfig, source: NativeVideoSource, counters: Arc<EncoderCounters>) -> Result<Self, StreamError> {
        let shared = Arc::new(Shared {
            codec: cfg.codec,
            width: cfg.width,
            height: cfg.height,
            source,
            counters: counters.clone(),
            param_sets: Mutex::new(Vec::new()),
        });
        let truthy: &CFType = CFBoolean::new(true);
        let falsy: &CFType = CFBoolean::new(false);
        let spec_keys: [&CFString; 1] = [unsafe { kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder }];
        let spec = CFDictionary::<CFString, CFType>::from_slices(&spec_keys, &[truthy]);
        let fmt = CFNumber::new_i32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange as i32);
        let w = CFNumber::new_i32(cfg.width as i32);
        let h = CFNumber::new_i32(cfg.height as i32);
        let iosurface = CFDictionary::<CFString, CFType>::from_slices(&[], &[]);
        let iosurface_ref: &CFType = iosurface.as_opaque();
        let src_keys: [&CFString; 4] =
            unsafe { [kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelBufferHeightKey, kCVPixelBufferIOSurfacePropertiesKey] };
        let src_vals: [&CFType; 4] = [&fmt, &w, &h, iosurface_ref];
        let src_attrs = CFDictionary::<CFString, CFType>::from_slices(&src_keys, &src_vals);

        let shared_ptr = Arc::as_ptr(&shared) as *mut c_void;
        let mut raw: *mut VTCompressionSession = null_mut();
        let status = unsafe {
            VTCompressionSession::create(
                None,
                cfg.width as i32,
                cfg.height as i32,
                match cfg.codec {
                    StreamCodec::H264 => kCMVideoCodecType_H264,
                    StreamCodec::H265 => kCMVideoCodecType_HEVC,
                },
                Some(spec.as_opaque()),
                Some(src_attrs.as_opaque()),
                None,
                Some(vt_output_callback),
                shared_ptr,
                NonNull::from(&mut raw),
            )
        };
        if status != 0 || raw.is_null() {
            return Err(StreamError::Encoder(format!("VTCompressionSessionCreate failed ({status}); the hardware encoder may be unavailable")));
        }
        let session = unsafe { CFRetained::from_raw(NonNull::new_unchecked(raw)) };
        let bps = cfg.bitrate_kbps as u64 * 1000;
        unsafe {
            set_prop(&session, kVTCompressionPropertyKey_RealTime, truthy);
            set_prop(
                &session,
                kVTCompressionPropertyKey_ProfileLevel,
                match cfg.codec {
                    StreamCodec::H264 => kVTProfileLevel_H264_High_AutoLevel,
                    StreamCodec::H265 => kVTProfileLevel_HEVC_Main_AutoLevel,
                },
            );
            set_prop(&session, kVTCompressionPropertyKey_AllowFrameReordering, falsy);
            let fps = CFNumber::new_f64(cfg.fps as f64);
            set_prop(&session, kVTCompressionPropertyKey_ExpectedFrameRate, &fps);
            let br = CFNumber::new_i64(bps as i64);
            set_prop(&session, kVTCompressionPropertyKey_AverageBitRate, &br);
            set_prop(&session, kVTCompressionPropertyKey_DataRateLimits, data_rate_limits(bps).as_opaque());
            let keyint = CFNumber::new_i32(KEYFRAME_INTERVAL);
            set_prop(&session, kVTCompressionPropertyKey_MaxKeyFrameInterval, &keyint);
            if cfg.codec == StreamCodec::H264 {
                set_prop(&session, kVTCompressionPropertyKey_H264EntropyMode, kVTH264EntropyMode_CABAC);
            }
            if cfg.mode == StreamMode::Game {
                set_prop(&session, kVTCompressionPropertyKey_PrioritizeEncodingSpeedOverQuality, truthy);
            }
        }
        let st = unsafe { session.prepare_to_encode_frames() };
        if st != 0 {
            return Err(StreamError::Encoder(format!("VTCompressionSessionPrepareToEncodeFrames failed ({st})")));
        }
        let mut hw = None;
        unsafe {
            let mut out: *const CFType = std::ptr::null();
            let st = VTSessionCopyProperty(
                &session,
                kVTCompressionPropertyKey_UsingHardwareAcceleratedVideoEncoder,
                None,
                &mut out as *mut *const CFType as *mut c_void,
            );
            if st == 0 && !out.is_null() {
                let v = CFRetained::from_raw(NonNull::new_unchecked(out as *mut CFType));
                if let Some(b) = v.downcast_ref::<CFBoolean>() {
                    hw = Some(b.value());
                }
            }
        }
        counters.applied_bps.store(bps, Ordering::Relaxed);
        Ok(Self { session, shared, hw, start: Instant::now(), counters, finished: AtomicBool::new(false) })
    }

    pub fn encode(&self, frame: &CapturedFrame, force_key: bool) -> Result<(), StreamError> {
        let pb_ptr = frame.as_raw() as *const CVPixelBuffer;
        if pb_ptr.is_null() {
            return Err(StreamError::Encoder("null pixel buffer".into()));
        }
        let pb: &CVPixelBuffer = unsafe { &*pb_ptr };
        let pts_us = self.start.elapsed().as_micros() as i64;
        let pts = CMTime { value: pts_us, timescale: 1_000_000, flags: CMTimeFlags::Valid, epoch: 0 };
        let dur = CMTime { value: 0, timescale: 1_000_000, flags: CMTimeFlags::empty(), epoch: 0 };
        let props = if force_key {
            let keys: [&CFString; 1] = [unsafe { kVTEncodeFrameOptionKey_ForceKeyFrame }];
            let truthy: &CFType = CFBoolean::new(true);
            Some(CFDictionary::<CFString, CFType>::from_slices(&keys, &[truthy]))
        } else {
            None
        };
        let ctx = Box::new(FrameCtx { submit: Instant::now(), capture_us: frame.capture_us() });
        let mut flags = VTEncodeInfoFlags::empty();
        let st = unsafe {
            self.session.encode_frame(pb, pts, dur, props.as_ref().map(|d| d.as_opaque()), Box::into_raw(ctx) as *mut c_void, &mut flags)
        };
        if st == 0 {
            self.counters.submitted.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.counters.errors.fetch_add(1, Ordering::Relaxed);
            Err(StreamError::Encoder(format!("VTCompressionSessionEncodeFrame failed ({st})")))
        }
    }

    pub fn set_bitrate(&self, bps: u64) {
        let br = CFNumber::new_i64(bps as i64);
        set_prop(&self.session, unsafe { kVTCompressionPropertyKey_AverageBitRate }, &br);
        set_prop(&self.session, unsafe { kVTCompressionPropertyKey_DataRateLimits }, data_rate_limits(bps).as_opaque());
        self.counters.applied_bps.store(bps, Ordering::Relaxed);
    }

    pub fn set_framerate(&self, fps: u16) {
        let v = CFNumber::new_f64(fps as f64);
        set_prop(&self.session, unsafe { kVTCompressionPropertyKey_ExpectedFrameRate }, &v);
    }

    pub fn hardware(&self) -> Option<bool> {
        self.hw
    }

    pub fn name(&self) -> &'static str {
        "VideoToolbox"
    }

    pub fn finish(&self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        unsafe {
            self.session.complete_frames(CMTime { value: 0, timescale: 1, flags: CMTimeFlags::empty(), epoch: 0 });
            self.session.invalidate();
        }
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        self.finish();
        let _ = &self.shared;
    }
}
