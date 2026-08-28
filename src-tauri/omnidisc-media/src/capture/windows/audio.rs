use crate::capture::{AudioSink, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};
use crate::stream::{AudioApp, StreamError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use windows::core::{implement, Interface, Ref};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::{
    eRender, ActivateAudioInterfaceAsync, IActivateAudioInterfaceAsyncOperation,
    IActivateAudioInterfaceCompletionHandler, IActivateAudioInterfaceCompletionHandler_Impl,
    IAudioCaptureClient, IAudioClient, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS,
    AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
    AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS, DEVICE_STATE_ACTIVE,
    PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE,
    PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE, VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
    WAVEFORMATEX,
};
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PROPVARIANT_0_0};
use windows::Win32::System::Com::{CoCreateInstance, CoIncrementMTAUsage, BLOB, CLSCTX_ALL};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};
use windows::Win32::System::Variant::VT_BLOB;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
/// `ActivateAudioInterfaceAsync` can sit on a slow audio engine; the TODO
/// budget for "screen share started" is 12 s.
const ACTIVATE_TIMEOUT_MS: u32 = 12_000;
const BUFFER_DURATION_100NS: i64 = 20 * 10_000;
const HEARTBEAT_QUIET: Duration = Duration::from_millis(250);
const HEARTBEAT_FRAMES: usize = (AUDIO_SAMPLE_RATE as usize / 100) * AUDIO_CHANNELS as usize;

/// WASAPI process loopback landed in Windows 10 build 20348 (Server 2022 /
/// Windows 11). Below that there is no per-application audio at all.
pub const MIN_LOOPBACK_BUILD: u32 = 20348;

pub fn windows_build() -> u32 {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.is_ok() {
        info.dwBuildNumber
    } else {
        0
    }
}

pub fn process_loopback_supported() -> bool {
    windows_build() >= MIN_LOOPBACK_BUILD
}

pub fn unsupported_build_error() -> StreamError {
    StreamError::Capture(format!(
        "sharing application audio needs Windows 10 build {MIN_LOOPBACK_BUILD} or newer (this machine reports build {}); update Windows or share without audio",
        windows_build()
    ))
}

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct Completion {
    event: isize,
}

impl IActivateAudioInterfaceCompletionHandler_Impl for Completion_Impl {
    fn ActivateCompleted(
        &self,
        _operation: Ref<'_, IActivateAudioInterfaceAsyncOperation>,
    ) -> windows::core::Result<()> {
        unsafe {
            let _ = SetEvent(HANDLE(self.event as *mut std::ffi::c_void));
        }
        Ok(())
    }
}

struct Event(HANDLE);

impl Event {
    fn new() -> Result<Self, StreamError> {
        let handle = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|e| StreamError::Capture(format!("CreateEventW: {e}")))?;
        Ok(Self(handle))
    }

    fn raw(&self) -> HANDLE {
        self.0
    }

    fn wait(&self, ms: u32) -> bool {
        unsafe { WaitForSingleObject(self.0, ms) == WAIT_OBJECT_0 }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn wave_format(float: bool) -> WAVEFORMATEX {
    let bits: u16 = if float { 32 } else { 16 };
    let channels = AUDIO_CHANNELS as u16;
    let block_align = channels * bits / 8;
    WAVEFORMATEX {
        wFormatTag: if float {
            WAVE_FORMAT_IEEE_FLOAT
        } else {
            WAVE_FORMAT_PCM
        },
        nChannels: channels,
        nSamplesPerSec: AUDIO_SAMPLE_RATE,
        nAvgBytesPerSec: AUDIO_SAMPLE_RATE * block_align as u32,
        nBlockAlign: block_align,
        wBitsPerSample: bits,
        cbSize: 0,
    }
}

struct Activated {
    client: IAudioClient,
    float: bool,
}

fn activate(pid: u32, include_tree: bool) -> Result<IAudioClient, StreamError> {
    let mut params = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
            ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                TargetProcessId: pid,
                ProcessLoopbackMode: if include_tree {
                    PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE
                } else {
                    PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE
                },
            },
        },
    };
    // Never let this variant be dropped: PROPVARIANT's Drop runs
    // PropVariantClear, which for VT_BLOB hands pBlobData to the COM allocator —
    // and this blob points at the stack local above. Freeing that corrupts the
    // heap, which is precisely how this failed the first time. The variant only
    // borrows `params` for the duration of the call, so there is nothing to
    // release either way.
    let mut prop = std::mem::ManuallyDrop::new(PROPVARIANT::default());
    unsafe {
        let inner: &mut PROPVARIANT_0_0 = &mut prop.Anonymous.Anonymous;
        inner.vt = VT_BLOB;
        inner.Anonymous.blob = BLOB {
            cbSize: std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
            pBlobData: &mut params as *mut AUDIOCLIENT_ACTIVATION_PARAMS as *mut u8,
        };
    }

    let event = Event::new()?;
    let handler: IActivateAudioInterfaceCompletionHandler = Completion {
        event: event.raw().0 as isize,
    }
    .into();
    let operation = unsafe {
        ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID,
            Some(&*prop as *const PROPVARIANT),
            &handler,
        )
    }
    .map_err(|e| StreamError::Capture(format!("ActivateAudioInterfaceAsync: {e}")))?;
    if !event.wait(ACTIVATE_TIMEOUT_MS) {
        return Err(StreamError::Capture(
            "the Windows audio engine did not answer within 12 s; try sharing without audio".into(),
        ));
    }
    let mut hr = windows::core::HRESULT(0);
    let mut raw: Option<windows::core::IUnknown> = None;
    unsafe {
        operation
            .GetActivateResult(&mut hr, &mut raw)
            .map_err(|e| StreamError::Capture(format!("GetActivateResult: {e}")))?;
    }
    hr.ok()
        .map_err(|e| StreamError::Capture(format!("process loopback activation refused: {e}")))?;
    raw.ok_or_else(|| StreamError::Capture("process loopback returned no audio client".into()))?
        .cast::<IAudioClient>()
        .map_err(|e| StreamError::Capture(format!("IAudioClient: {e}")))
}

fn try_initialize(pid: u32, include_tree: bool, float: bool) -> Result<Activated, StreamError> {
    let client = activate(pid, include_tree)?;
    let format = wave_format(float);
    unsafe {
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                BUFFER_DURATION_100NS,
                0,
                &format,
                None,
            )
            .map_err(|e| {
                StreamError::Capture(format!(
                    "the audio engine refused 48 kHz stereo {} capture: {e}",
                    if float { "float" } else { "16-bit" }
                ))
            })?;
    }
    Ok(Activated { client, float })
}

fn initialize(pid: u32, include_tree: bool) -> Result<Activated, StreamError> {
    match try_initialize(pid, include_tree, true) {
        Ok(activated) => Ok(activated),
        Err(e) => {
            tracing::debug!("[omnidisc-media] {e}; retrying as 16-bit PCM");
            try_initialize(pid, include_tree, false)
        }
    }
}

pub struct AudioCaptureHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioCaptureHandle {
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for AudioCaptureHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn pump(
    activated: Activated,
    sink: AudioSink,
    stop: Arc<AtomicBool>,
    ready: &Event,
) -> Result<(), StreamError> {
    let client = activated.client;
    unsafe {
        client
            .SetEventHandle(ready.raw())
            .map_err(|e| StreamError::Capture(format!("SetEventHandle: {e}")))?;
    }
    let capture: IAudioCaptureClient = unsafe { client.GetService() }
        .map_err(|e| StreamError::Capture(format!("IAudioCaptureClient: {e}")))?;
    unsafe {
        client
            .Start()
            .map_err(|e| StreamError::Capture(format!("IAudioClient::Start: {e}")))?;
    }
    let silence = vec![0f32; HEARTBEAT_FRAMES];
    // The publish side expects whole 10 ms frames; WASAPI packet sizes are not
    // guaranteed to line up, so partial frames are carried over instead of
    // being dropped.
    let mut pending: Vec<f32> = Vec::with_capacity(HEARTBEAT_FRAMES * 4);
    let mut last_pcm = Instant::now();
    while !stop.load(Ordering::Acquire) {
        ready.wait(100);
        loop {
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut frames: u32 = 0;
            let mut flags: u32 = 0;
            let got = unsafe { capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None) };
            // A successful call with zero frames means the buffer is empty
            // (AUDCLNT_S_BUFFER_EMPTY); nothing was acquired, so nothing is
            // released.
            if got.is_err() || frames == 0 {
                break;
            }
            let channels = AUDIO_CHANNELS as usize;
            let samples = frames as usize * channels;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                pending.resize(pending.len() + samples, 0.0);
            } else if activated.float {
                let src = unsafe { std::slice::from_raw_parts(data as *const f32, samples) };
                pending.extend_from_slice(src);
            } else {
                let src = unsafe { std::slice::from_raw_parts(data as *const i16, samples) };
                pending.extend(src.iter().map(|s| *s as f32 / i16::MAX as f32));
            }
            unsafe {
                let _ = capture.ReleaseBuffer(frames);
            }
            let whole = pending.len() / HEARTBEAT_FRAMES * HEARTBEAT_FRAMES;
            if whole > 0 {
                sink(&pending[..whole]);
                pending.drain(..whole);
                last_pcm = Instant::now();
            }
        }
        if last_pcm.elapsed() >= HEARTBEAT_QUIET {
            // WHY: process loopback delivers nothing at all while the target is
            // silent, and the publish gate upstream waits for a first packet.
            sink(&silence);
            last_pcm = Instant::now();
        }
    }
    unsafe {
        let _ = client.Stop();
    }
    Ok(())
}

pub fn start_process_loopback(
    pid: u32,
    include_tree: bool,
    sink: AudioSink,
) -> Result<AudioCaptureHandle, StreamError> {
    if !process_loopback_supported() {
        return Err(unsupported_build_error());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let (tx, rx) = mpsc::channel::<Result<(), StreamError>>();
    let thread = std::thread::Builder::new()
        .name("omnidisc-wasapi".into())
        .spawn(move || {
            ensure_mta();
            let ready = match Event::new() {
                Ok(e) => e,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            match initialize(pid, include_tree) {
                Ok(activated) => {
                    let _ = tx.send(Ok(()));
                    if let Err(e) = pump(activated, sink, stop_thread, &ready) {
                        tracing::warn!("[omnidisc-media] process loopback stopped: {e}");
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
        })
        .map_err(|e| StreamError::Capture(format!("wasapi thread: {e}")))?;
    match rx.recv() {
        Ok(Ok(())) => Ok(AudioCaptureHandle {
            stop,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            let _ = thread.join();
            Err(StreamError::Capture(
                "the audio capture thread stopped before it started".into(),
            ))
        }
    }
}

/// Enumerates render sessions across **every** active endpoint, not just the
/// default one: engines like FMOD and OpenAL open their own device, and a
/// session that only exists on a secondary endpoint would otherwise be missing.
/// Silent sessions stay in the list so a paused tab is still pickable.
pub fn audio_apps() -> Vec<AudioApp> {
    let own_pid = std::process::id();
    let mut out: Vec<AudioApp> = Vec::new();
    let mut seen: Vec<u32> = Vec::new();
    let enumerator: IMMDeviceEnumerator =
        match unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) } {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("[omnidisc-media] MMDeviceEnumerator: {e}");
                return out;
            }
        };
    let devices = match unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) } {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("[omnidisc-media] EnumAudioEndpoints: {e}");
            return out;
        }
    };
    let count = unsafe { devices.GetCount() }.unwrap_or(0);
    for i in 0..count {
        let Ok(device) = (unsafe { devices.Item(i) }) else {
            continue;
        };
        let manager: IAudioSessionManager2 = match unsafe { device.Activate(CLSCTX_ALL, None) } {
            Ok(m) => m,
            Err(_) => continue,
        };
        let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else {
            continue;
        };
        let n = unsafe { sessions.GetCount() }.unwrap_or(0);
        for s in 0..n {
            let Ok(control) = (unsafe { sessions.GetSession(s) }) else {
                continue;
            };
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                continue;
            };
            if unsafe { control2.IsSystemSoundsSession() }.is_ok() {
                continue;
            }
            let Ok(pid) = (unsafe { control2.GetProcessId() }) else {
                continue;
            };
            if pid == 0 || pid == own_pid || seen.contains(&pid) {
                continue;
            }
            seen.push(pid);
            if let Some(app) = super::sources::audio_app(pid) {
                out.push(app);
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// COM apartment guard for the enumeration helpers, which may run on a tokio
/// blocking thread whose apartment we do not control.
/// Guarantees the process has a multithreaded apartment, once, for good.
///
/// The obvious shape — initialise COM on entry, uninitialise on the way out —
/// crashes. `CoUninitialize` tears the apartment down, but the activation
/// factories the windows crate caches for WinRT types survive it, so the next
/// `GraphicsCaptureSession::IsSupported` dereferences freed memory: opening the
/// share picker a second time was enough to fault. `CoIncrementMTAUsage` is the
/// API for this exact need — it keeps an implicit MTA alive for every thread
/// that never initialises one itself, and the cookie is deliberately never
/// released.
pub fn ensure_mta() {
    static MTA: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    MTA.get_or_init(|| {
        if let Err(e) = unsafe { CoIncrementMTAUsage() } {
            tracing::warn!("[omnidisc-media] could not hold an MTA open: {e}");
        }
    });
}
