use crate::audio::{
    capture::{CaptureEvent, CaptureFlags, Feeder, FeederMsg, FRAME_SAMPLES, SAMPLE_RATE},
    devices,
    io::{classify_loss, AudioIo, AudioIoError, DeviceLoss, StreamFault},
    playback::{Mixer, MIX_RATE},
    AudioDevices, DeviceKind,
};
use crate::e2ee::{KeyRing, KeyRotation, RoomKey};
use crate::engine::{
    AudioPrefs, BackendEvent, ConnectOptions, ConnectOutcome, DeviceStatus, EngineNotification, MediaBackend,
    MediaError, Quality, VoiceStats,
};
use crate::state::VoiceEvent;
use crate::tone::ToneMeter;
use async_trait::async_trait;
use futures::StreamExt;
use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions};
use livekit::e2ee::{E2eeOptions, EncryptionType};
use livekit::prelude::*;
use livekit::webrtc::audio_source::{native::NativeAudioSource, AudioSourceOptions, RtcAudioSource};
use livekit::webrtc::audio_stream::native::NativeAudioStream;
use livekit::webrtc::peer_connection_factory::{IceServer, IceTransportsType};
use livekit::webrtc::stats::RtcStats;
use livekit::options::TrackPublishOptions;
use livekit::{DisconnectReason, RoomEvent, RoomOptions};
use omnidisc_proto::gateway::VoiceServerUpdate;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const PTT_RELEASE_DELAY: Duration = Duration::from_millis(200);
const REMOTE_RING_SAMPLES: usize = MIX_RATE as usize;

struct Session {
    room: Arc<Room>,
    audio_track: LocalAudioTrack,
    remote_audio: Arc<StdMutex<HashMap<String, RemoteAudioTrack>>>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

pub const SCREEN_AUDIO_SUFFIX: &str = "#screen";

#[derive(Clone)]
pub struct RemoteVideo {
    pub user_id: String,
    pub sid: String,
    pub track: RemoteVideoTrack,
}

struct Shared {
    events: StdMutex<Option<mpsc::UnboundedSender<BackendEvent>>>,
    flags: Arc<CaptureFlags>,
    mixer: Arc<Mixer>,
    feeder: Arc<Feeder>,
    io: AudioIo,
    session: Mutex<Option<Session>>,
    local_identity: StdMutex<Option<String>>,
    volumes: StdMutex<HashMap<String, f32>>,
    input_device: StdMutex<Option<String>>,
    output_device: StdMutex<Option<String>>,
    input_running: AtomicBool,
    output_running: AtomicBool,
    monitor: AtomicBool,
    ptt_generation: AtomicU64,
    last_stats: StdMutex<Option<(Instant, u64, u64)>>,
    remote_nonsilent_frames: AtomicU64,
    remote_tone: StdMutex<ToneMeter>,
    video_publications: StdMutex<HashMap<String, RemoteTrackPublication>>,
    remote_video: StdMutex<HashMap<String, RemoteVideo>>,
    rotation: KeyRotation,
    recovering_input: AtomicBool,
    recovering_output: AtomicBool,
}

/// LiveKit's frame cryptor behind the crate's transport-free `KeyRing` trait.
struct LiveKitRing(KeyProvider);

impl KeyRing for LiveKitRing {
    fn set_shared_key(&self, key: &[u8], index: i32) {
        self.0.set_shared_key(key.to_vec(), index);
    }
}

impl Shared {
    fn emit(&self, n: EngineNotification) {
        if let Ok(guard) = self.events.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(BackendEvent::Notify(n));
            }
        }
    }

    fn transport(&self, ev: VoiceEvent) {
        if let Ok(guard) = self.events.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(BackendEvent::Transport(ev));
            }
        }
    }

    fn local_identity(&self) -> Option<String> {
        self.local_identity.lock().ok().and_then(|g| g.clone())
    }

    fn volume_for(&self, user_id: &str) -> f32 {
        self.volumes.lock().ok().and_then(|v| v.get(user_id).copied()).unwrap_or(1.0)
    }

    fn input_device(&self) -> Option<String> {
        self.input_device.lock().ok().and_then(|d| d.clone())
    }

    fn output_device(&self) -> Option<String> {
        self.output_device.lock().ok().and_then(|d| d.clone())
    }

    fn ensure_input(&self) -> Result<(), AudioIoError> {
        if self.input_running.load(Ordering::Acquire) {
            return Ok(());
        }
        self.io.start_input(self.input_device())?;
        self.input_running.store(true, Ordering::Release);
        Ok(())
    }

    fn stop_input_if_idle(&self) {
        let in_call = self.session.try_lock().map(|s| s.is_some()).unwrap_or(true);
        if !in_call && !self.monitor.load(Ordering::Acquire) {
            self.io.stop_input();
            self.input_running.store(false, Ordering::Release);
        }
    }

    fn recovering(&self, kind: DeviceKind) -> &AtomicBool {
        match kind {
            DeviceKind::Input => &self.recovering_input,
            DeviceKind::Output => &self.recovering_output,
        }
    }

    fn in_call(&self) -> bool {
        self.session.try_lock().map(|s| s.is_some()).unwrap_or(true)
    }

    /// Is this device still worth re-opening? A mic is, while the call is up or
    /// the mic test is running; an output only while the call is up.
    fn still_wanted(&self, kind: DeviceKind) -> bool {
        match kind {
            DeviceKind::Input => self.in_call() || self.monitor.load(Ordering::Acquire),
            DeviceKind::Output => self.in_call(),
        }
    }

    fn ensure_output(&self) -> Result<(), AudioIoError> {
        if self.output_running.load(Ordering::Acquire) {
            return Ok(());
        }
        self.io.start_output(self.output_device())?;
        self.output_running.store(true, Ordering::Release);
        Ok(())
    }
}

pub struct LiveKitBackend {
    rt: Option<tokio::runtime::Runtime>,
    shared: Arc<Shared>,
}

impl Drop for LiveKitBackend {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

fn map_io_error(e: &AudioIoError, kind: DeviceKind) -> String {
    match (e, kind) {
        (AudioIoError::PermissionDenied, DeviceKind::Input) => "ERR_VOICE_MIC_PERMISSION".into(),
        (AudioIoError::NoDevice, DeviceKind::Input) => "ERR_VOICE_NO_INPUT_DEVICE".into(),
        (AudioIoError::NoDevice, DeviceKind::Output) => "ERR_VOICE_NO_OUTPUT_DEVICE".into(),
        (AudioIoError::DeviceBusy, _) => "ERR_VOICE_DEVICE_BUSY".into(),
        (_, DeviceKind::Input) => "ERR_VOICE_MIC_FAILED".into(),
        (_, DeviceKind::Output) => "ERR_VOICE_OUTPUT_FAILED".into(),
    }
}

/// One device disappeared mid-call. Re-open the same device first, then the
/// system default, and only then give up — a USB mic that was yanked and put
/// back must not need a rejoin, and a permission the user revoked must not look
/// like a broken app.
fn recover(shared: &Arc<Shared>, kind: DeviceKind) {
    const BACKOFF_MS: [u64; 3] = [250, 1_000, 3_000];
    let wanted = match kind {
        DeviceKind::Input => shared.input_device(),
        DeviceKind::Output => shared.output_device(),
    };
    for (attempt, wait) in BACKOFF_MS.iter().enumerate() {
        std::thread::sleep(Duration::from_millis(*wait));
        if !shared.still_wanted(kind) {
            return;
        }
        let probe = match kind {
            DeviceKind::Input => shared.io.start_input(wanted.clone()),
            DeviceKind::Output => shared.io.start_output(wanted.clone()),
        };
        let error = match probe {
            Ok(()) => {
                match kind {
                    DeviceKind::Input => shared.input_running.store(true, Ordering::Release),
                    DeviceKind::Output => shared.output_running.store(true, Ordering::Release),
                }
                shared.emit(EngineNotification::Device { kind, status: DeviceStatus::Recovered, cause: None });
                return;
            }
            Err(e) => e,
        };
        let still_listed = wanted.as_deref().map(|id| devices::exists(kind, id)).unwrap_or(true);
        let cause = classify_loss(still_listed, &error);
        if cause == DeviceLoss::PermissionRevoked {
            give_up(shared, kind, cause);
            return;
        }
        if wanted.is_some() && cause == DeviceLoss::Unplugged {
            let fallback = match kind {
                DeviceKind::Input => shared.io.start_input(None),
                DeviceKind::Output => shared.io.start_output(None),
            };
            if fallback.is_ok() {
                match kind {
                    DeviceKind::Input => {
                        if let Ok(mut d) = shared.input_device.lock() {
                            *d = None;
                        }
                        shared.input_running.store(true, Ordering::Release);
                    }
                    DeviceKind::Output => {
                        if let Ok(mut d) = shared.output_device.lock() {
                            *d = None;
                        }
                        shared.output_running.store(true, Ordering::Release);
                    }
                }
                shared.emit(EngineNotification::Device {
                    kind,
                    status: DeviceStatus::SwitchedToDefault,
                    cause: Some(cause),
                });
                return;
            }
        }
        if attempt + 1 == BACKOFF_MS.len() {
            give_up(shared, kind, cause);
        }
    }
}

fn give_up(shared: &Arc<Shared>, kind: DeviceKind, cause: DeviceLoss) {
    let status = match kind {
        DeviceKind::Input => {
            shared.io.stop_input();
            shared.input_running.store(false, Ordering::Release);
            DeviceStatus::ListenOnly
        }
        DeviceKind::Output => {
            shared.io.stop_output();
            shared.output_running.store(false, Ordering::Release);
            DeviceStatus::Silent
        }
    };
    tracing::warn!("[omnidisc-media] giving up on the {:?} device: {:?}", kind, cause);
    shared.emit(EngineNotification::Device { kind, status, cause: Some(cause) });
}

impl LiveKitBackend {
    pub fn new() -> Result<Self, MediaError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("omnidisc-media")
            .enable_all()
            .build()
            .map_err(|e| MediaError::Unavailable(format!("media runtime: {e}")))?;
        let flags = Arc::new(CaptureFlags::default());
        let mixer = Arc::new(Mixer::default());
        let events: StdMutex<Option<mpsc::UnboundedSender<BackendEvent>>> = StdMutex::new(None);
        let shared_slot: Arc<StdMutex<Option<std::sync::Weak<Shared>>>> = Arc::new(StdMutex::new(None));
        let capture_slot = shared_slot.clone();
        let capture_sink: crate::audio::capture::CaptureSink = Arc::new(move |ev| {
            let Some(shared) = capture_slot.lock().ok().and_then(|s| s.as_ref().and_then(|w| w.upgrade())) else {
                return;
            };
            match ev {
                CaptureEvent::Level { rms_db, peak } => shared.emit(EngineNotification::Level { rms_db, peak }),
                CaptureEvent::Speaking(speaking) => {
                    shared.mixer.set_ducking(speaking);
                    if let Some(id) = shared.local_identity() {
                        shared.emit(EngineNotification::Speaking { user_id: id, speaking });
                    }
                }
                CaptureEvent::Underrun => tracing::debug!("[omnidisc-media] microphone underrun"),
            }
        });
        let feeder = Arc::new(Feeder::spawn(flags.clone(), capture_sink).map_err(MediaError::Device)?);
        let fault_slot = shared_slot.clone();
        let fault_sink: crate::audio::io::FaultSink = Arc::new(move |fault, message| {
            let Some(shared) = fault_slot.lock().ok().and_then(|s| s.as_ref().and_then(|w| w.upgrade())) else {
                return;
            };
            tracing::warn!("[omnidisc-media] audio stream fault {:?}: {}", fault, message);
            let kind = match fault {
                StreamFault::Input => {
                    shared.input_running.store(false, Ordering::Release);
                    shared.emit(EngineNotification::Error { code: "ERR_VOICE_MIC_LOST".into(), message });
                    DeviceKind::Input
                }
                StreamFault::Output => {
                    shared.output_running.store(false, Ordering::Release);
                    shared.emit(EngineNotification::Error { code: "ERR_VOICE_OUTPUT_LOST".into(), message });
                    DeviceKind::Output
                }
            };
            shared.emit(EngineNotification::Device { kind, status: DeviceStatus::Lost, cause: None });
            // The cpal error callback runs on the device thread: re-opening a
            // device from here would deadlock the very thread that has to answer.
            if shared.recovering(kind).swap(true, Ordering::AcqRel) {
                return;
            }
            let worker = shared.clone();
            if std::thread::Builder::new()
                .name("omnidisc-audio-recovery".into())
                .spawn(move || {
                    recover(&worker, kind);
                    worker.recovering(kind).store(false, Ordering::Release);
                })
                .is_err()
            {
                shared.recovering(kind).store(false, Ordering::Release);
            }
        });
        let io = AudioIo::spawn(feeder.clone(), mixer.clone(), fault_sink).map_err(MediaError::Device)?;
        let shared = Arc::new(Shared {
            events,
            flags,
            mixer,
            feeder,
            io,
            session: Mutex::new(None),
            local_identity: StdMutex::new(None),
            volumes: StdMutex::new(HashMap::new()),
            input_device: StdMutex::new(None),
            output_device: StdMutex::new(None),
            input_running: AtomicBool::new(false),
            output_running: AtomicBool::new(false),
            monitor: AtomicBool::new(false),
            ptt_generation: AtomicU64::new(0),
            last_stats: StdMutex::new(None),
            remote_nonsilent_frames: AtomicU64::new(0),
            remote_tone: StdMutex::new(ToneMeter::new(440.0, MIX_RATE as f64)),
            video_publications: StdMutex::new(HashMap::new()),
            remote_video: StdMutex::new(HashMap::new()),
            rotation: KeyRotation::new(),
            recovering_input: AtomicBool::new(false),
            recovering_output: AtomicBool::new(false),
        });
        if let Ok(mut slot) = shared_slot.lock() {
            *slot = Some(Arc::downgrade(&shared));
        }
        Ok(Self { rt: Some(rt), shared })
    }

    pub fn remote_nonsilent_frames(&self) -> u64 {
        self.shared.remote_nonsilent_frames.load(Ordering::Relaxed)
    }

    /// Share of the received remote energy sitting at 440 Hz — the test tone.
    /// Amplitude alone cannot distinguish delivered audio from the comfort noise
    /// Opus invents when frames fail to decrypt, so the e2e tests assert on this.
    pub fn remote_tone_ratio(&self) -> f64 {
        self.shared.remote_tone.lock().map(|m| m.ratio()).unwrap_or(0.0)
    }

    pub fn set_test_tone(&self, hz: Option<f32>) {
        self.shared.feeder.send(FeederMsg::TestTone(hz));
    }

    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.rt.as_ref().map(|rt| rt.handle().clone())
    }

    pub async fn current_room(&self) -> Option<Arc<Room>> {
        self.shared.session.lock().await.as_ref().map(|s| s.room.clone())
    }

    pub async fn is_connected(&self) -> bool {
        self.shared.session.lock().await.is_some()
    }

    pub fn set_screen_audio_gain(&self, user_id: &str, gain: f32) {
        let key = format!("{user_id}{SCREEN_AUDIO_SUFFIX}");
        if let Ok(mut v) = self.shared.volumes.lock() {
            v.insert(key.clone(), gain.clamp(0.0, 2.0));
        }
        self.shared.mixer.set_user_gain(&key, gain.clamp(0.0, 2.0));
    }

    pub fn remote_video_for(&self, user_id: &str) -> Option<RemoteVideo> {
        self.shared.remote_video.lock().ok().and_then(|m| m.get(user_id).cloned())
    }

    pub fn video_publication_for(&self, user_id: &str) -> Option<RemoteTrackPublication> {
        self.shared.video_publications.lock().ok().and_then(|m| m.get(user_id).cloned())
    }

    pub fn streaming_user_ids(&self) -> Vec<String> {
        self.shared.video_publications.lock().map(|m| m.keys().cloned().collect()).unwrap_or_default()
    }

    async fn on_runtime<T: Send + 'static>(
        &self,
        fut: impl std::future::Future<Output = Result<T, MediaError>> + Send + 'static,
    ) -> Result<T, MediaError> {
        let Some(rt) = self.rt.as_ref() else { return Err(MediaError::Unavailable("media runtime stopped".into())) };
        match rt.spawn(fut).await {
            Ok(r) => r,
            Err(e) if e.is_panic() => Err(MediaError::Panicked),
            Err(e) => Err(MediaError::Unavailable(e.to_string())),
        }
    }
}

fn quality_of(q: ConnectionQuality) -> Quality {
    match q {
        ConnectionQuality::Excellent => Quality::Excellent,
        ConnectionQuality::Good => Quality::Good,
        ConnectionQuality::Poor => Quality::Poor,
        ConnectionQuality::Lost => Quality::Lost,
    }
}

fn subscribe_audio(publication: &RemoteTrackPublication) {
    if publication.kind() == TrackKind::Audio && !publication.is_subscribed() {
        publication.set_subscribed(true);
    }
}

fn spawn_remote_audio(
    shared: Arc<Shared>,
    remote_audio: Arc<StdMutex<HashMap<String, RemoteAudioTrack>>>,
    track: RemoteAudioTrack,
    _user_id: String,
    gain_key: String,
) -> tokio::task::JoinHandle<()> {
    let key = track.sid().to_string();
    if let Ok(mut m) = remote_audio.lock() {
        m.insert(key.clone(), track.clone());
    }
    tokio::spawn(async move {
        let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(REMOTE_RING_SAMPLES);
        shared.mixer.add_source(key.clone(), gain_key.clone(), consumer, shared.volume_for(&gain_key));
        let mut stream = NativeAudioStream::new(track.rtc_track(), MIX_RATE as i32, 1);
        let mut scratch: Vec<f32> = Vec::with_capacity(FRAME_SAMPLES * 4);
        while let Some(frame) = stream.next().await {
            scratch.clear();
            scratch.extend(frame.data.iter().map(|s| *s as f32 / 32_768.0));
            if scratch.iter().any(|s| s.abs() > 0.01) {
                shared.remote_nonsilent_frames.fetch_add(1, Ordering::Relaxed);
            }
            if let Ok(mut meter) = shared.remote_tone.lock() {
                meter.push(&scratch);
            }
            let _ = producer.push_partial_slice(&scratch);
        }
        shared.mixer.remove_source(&key);
        if let Ok(mut m) = remote_audio.lock() {
            m.remove(&key);
        }
    })
}

async fn run_room_events(
    shared: Arc<Shared>,
    remote_audio: Arc<StdMutex<HashMap<String, RemoteAudioTrack>>>,
    local_identity: String,
    mut events: mpsc::UnboundedReceiver<RoomEvent>,
) {
    let mut speakers: HashSet<String> = HashSet::new();
    let mut feeders: HashMap<String, tokio::task::JoinHandle<()>> = HashMap::new();
    while let Some(ev) = events.recv().await {
        match ev {
            RoomEvent::ParticipantConnected(p) => {
                shared.emit(EngineNotification::ParticipantJoined { user_id: p.identity().0 });
            }
            RoomEvent::ParticipantDisconnected(p) => {
                let id = p.identity().0;
                if speakers.remove(&id) {
                    shared.emit(EngineNotification::Speaking { user_id: id.clone(), speaking: false });
                }
                shared.emit(EngineNotification::ParticipantLeft { user_id: id });
            }
            RoomEvent::TrackPublished { publication, participant } => {
                if publication.kind() == TrackKind::Video {
                    if let Ok(mut m) = shared.video_publications.lock() {
                        m.insert(participant.identity().0, publication.clone());
                    }
                } else {
                    subscribe_audio(&publication);
                }
            }
            RoomEvent::TrackUnpublished { publication, participant } => {
                if publication.kind() == TrackKind::Video {
                    if let Ok(mut m) = shared.video_publications.lock() {
                        m.remove(&participant.identity().0);
                    }
                    if let Ok(mut m) = shared.remote_video.lock() {
                        m.remove(&participant.identity().0);
                    }
                }
            }
            RoomEvent::TrackSubscribed { track, participant, publication } => match track {
                RemoteTrack::Audio(audio) => {
                    let sid = audio.sid().to_string();
                    let user = participant.identity().0;
                    let gain_key = if publication.source() == TrackSource::ScreenshareAudio {
                        format!("{user}{SCREEN_AUDIO_SUFFIX}")
                    } else {
                        user
                    };
                    let handle = spawn_remote_audio(shared.clone(), remote_audio.clone(), audio, participant.identity().0, gain_key);
                    if let Some(old) = feeders.insert(sid, handle) {
                        old.abort();
                    }
                }
                RemoteTrack::Video(video) => {
                    if let Ok(mut m) = shared.remote_video.lock() {
                        m.insert(
                            participant.identity().0.clone(),
                            RemoteVideo { user_id: participant.identity().0, sid: video.sid().to_string(), track: video },
                        );
                    }
                }
            },
            RoomEvent::TrackUnsubscribed { track, participant, .. } => match track {
                RemoteTrack::Audio(audio) => {
                    let sid = audio.sid().to_string();
                    if let Some(h) = feeders.remove(&sid) {
                        h.abort();
                    }
                    shared.mixer.remove_source(&sid);
                    if let Ok(mut m) = remote_audio.lock() {
                        m.remove(&sid);
                    }
                }
                RemoteTrack::Video(_) => {
                    if let Ok(mut m) = shared.remote_video.lock() {
                        m.remove(&participant.identity().0);
                    }
                }
            },
            RoomEvent::ActiveSpeakersChanged { speakers: now } => {
                let next: HashSet<String> = now
                    .iter()
                    .map(|p| p.identity().0)
                    .filter(|id| *id != local_identity)
                    .collect();
                for id in speakers.difference(&next) {
                    shared.emit(EngineNotification::Speaking { user_id: id.clone(), speaking: false });
                }
                for id in next.difference(&speakers) {
                    shared.emit(EngineNotification::Speaking { user_id: id.clone(), speaking: true });
                }
                speakers = next;
            }
            RoomEvent::ConnectionQualityChanged { quality, participant } => {
                shared.emit(EngineNotification::Quality { user_id: participant.identity().0, quality: quality_of(quality) });
            }
            RoomEvent::Reconnecting => shared.transport(VoiceEvent::Disconnected { recoverable: true }),
            RoomEvent::Reconnected => shared.transport(VoiceEvent::Reconnected),
            RoomEvent::Disconnected { reason } => {
                if reason != DisconnectReason::ClientInitiated {
                    tracing::warn!("[omnidisc-media] room disconnected: {:?}", reason);
                    shared.emit(EngineNotification::Error {
                        code: "ERR_VOICE_DISCONNECTED".into(),
                        message: format!("{reason:?}"),
                    });
                    shared.transport(VoiceEvent::Disconnected { recoverable: false });
                }
                break;
            }
            _ => {}
        }
    }
    for (_, h) in feeders.drain() {
        h.abort();
    }
}

async fn teardown(shared: &Arc<Shared>) {
    let session = shared.session.lock().await.take();
    shared.feeder.send(FeederMsg::Source(None));
    if let Ok(mut id) = shared.local_identity.lock() {
        *id = None;
    }
    if let Some(s) = session {
        for t in s.tasks {
            t.abort();
        }
        if let Err(e) = tokio::time::timeout(Duration::from_secs(5), s.room.close()).await {
            tracing::debug!("[omnidisc-media] room close timed out: {e}");
        }
    }
    if let Ok(mut m) = shared.video_publications.lock() {
        m.clear();
    }
    if let Ok(mut m) = shared.remote_video.lock() {
        m.clear();
    }
    shared.rotation.disarm();
    shared.mixer.set_ducking(false);
    shared.mixer.clear();
    shared.io.stop_output();
    shared.output_running.store(false, Ordering::Release);
    shared.stop_input_if_idle();
    if let Ok(mut l) = shared.last_stats.lock() {
        *l = None;
    }
}

#[async_trait]
impl MediaBackend for LiveKitBackend {
    fn set_events(&self, tx: mpsc::UnboundedSender<BackendEvent>) {
        if let Ok(mut g) = self.shared.events.lock() {
            *g = Some(tx);
        }
    }

    async fn connect(
        &self,
        target: &VoiceServerUpdate,
        prefs: &AudioPrefs,
        options: &ConnectOptions,
    ) -> Result<ConnectOutcome, MediaError> {
        let shared = self.shared.clone();
        let target = target.clone();
        let prefs = prefs.clone();
        let connect_options = options.clone();
        self.on_runtime(async move {
            teardown(&shared).await;
            if let Ok(mut d) = shared.input_device.lock() {
                *d = prefs.input_device.clone();
            }
            if let Ok(mut d) = shared.output_device.lock() {
                *d = prefs.output_device.clone();
            }
            shared.flags.denoise.store(prefs.noise_suppression && cfg!(feature = "rnnoise"), Ordering::Relaxed);
            shared.flags.ptt_enabled.store(prefs.ptt_enabled, Ordering::Relaxed);
            if let Some(db) = prefs.vad_threshold_db {
                shared.flags.set_vad_threshold_db(db);
            }
            shared.mixer.set_duck_percent(prefs.ducking_percent);
            shared.mixer.set_ducking(false);

            let mut options = RoomOptions::default();
            options.auto_subscribe = false;
            options.adaptive_stream = true;
            options.dynacast = true;
            options.connect_timeout = CONNECT_TIMEOUT;
            if !target.ice_servers.is_empty() {
                options.rtc_config.ice_servers = target
                    .ice_servers
                    .iter()
                    .map(|s| IceServer {
                        urls: s.urls.clone(),
                        username: s.username.clone().unwrap_or_default(),
                        password: s.credential.clone().unwrap_or_default(),
                    })
                    .collect();
            }
            if connect_options.relay_only {
                options.rtc_config.ice_transport_type = IceTransportsType::Relay;
            }
            // Shared-key mode: every member derives the same key from the MLS
            // group, so nothing about it is negotiated over the wire. The ring
            // index is the epoch, which is what lets a member who has not merged
            // the newest commit yet still decode the previous epoch's frames.
            if let Some(room_key) = connect_options.room_key {
                let provider =
                    KeyProvider::with_shared_key(KeyProviderOptions::default(), room_key.key.to_vec());
                options.encryption = Some(E2eeOptions {
                    encryption_type: EncryptionType::Gcm,
                    key_provider: provider.clone(),
                });
                shared.rotation.arm(Arc::new(LiveKitRing(provider)), room_key);
                tracing::info!("[omnidisc-media] room {} is end-to-end encrypted", target.room);
            }
            let connect = Room::connect(&target.endpoint, &target.token, options);
            let (room, events) = match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    tracing::warn!("[omnidisc-media] livekit connect to {} failed: {}", target.endpoint, e);
                    return Err(MediaError::Connection(e.to_string()));
                }
                Err(_) => return Err(MediaError::Connection("timed out".into())),
            };
            let room = Arc::new(room);
            let local_identity = room.local_participant().identity().0;
            if let Ok(mut id) = shared.local_identity.lock() {
                *id = Some(local_identity.clone());
            }

            let source = NativeAudioSource::new(
                AudioSourceOptions {
                    echo_cancellation: true,
                    noise_suppression: !(prefs.noise_suppression && cfg!(feature = "rnnoise")),
                    auto_gain_control: true,
                },
                SAMPLE_RATE,
                1,
                0,
            );
            let audio_track = LocalAudioTrack::create_audio_track("microphone", RtcAudioSource::Native(source.clone()));
            room.local_participant()
                .publish_track(
                    LocalTrack::Audio(audio_track.clone()),
                    TrackPublishOptions { source: TrackSource::Microphone, dtx: true, ..Default::default() },
                )
                .await
                .map_err(|e| MediaError::Connection(format!("publish microphone: {e}")))?;
            shared.feeder.send(FeederMsg::Source(Some(source)));
            if shared.flags.muted.load(Ordering::Relaxed) {
                audio_track.mute();
            }

            let remote_audio: Arc<StdMutex<HashMap<String, RemoteAudioTrack>>> = Arc::new(StdMutex::new(HashMap::new()));
            for (_, participant) in room.remote_participants() {
                shared.emit(EngineNotification::ParticipantJoined { user_id: participant.identity().0 });
                for (_, publication) in participant.track_publications() {
                    if publication.kind() == TrackKind::Video {
                        if let Ok(mut m) = shared.video_publications.lock() {
                            m.insert(participant.identity().0, publication.clone());
                        }
                    } else {
                        subscribe_audio(&publication);
                    }
                }
            }
            let events_task = tokio::spawn(run_room_events(shared.clone(), remote_audio.clone(), local_identity.clone(), events));

            let mut outcome = ConnectOutcome::default();
            if let Err(e) = shared.ensure_output() {
                tracing::warn!("[omnidisc-media] output device: {}", e);
                outcome.output_error = Some(map_io_error(&e, DeviceKind::Output));
            }
            if let Err(e) = shared.ensure_input() {
                tracing::warn!("[omnidisc-media] input device: {}", e);
                outcome.mic_error = Some(map_io_error(&e, DeviceKind::Input));
            }

            *shared.session.lock().await = Some(Session {
                room,
                audio_track,
                remote_audio,
                tasks: vec![events_task],
            });
            Ok(outcome)
        })
        .await
    }

    async fn disconnect(&self) -> Result<(), MediaError> {
        let shared = self.shared.clone();
        self.on_runtime(async move {
            teardown(&shared).await;
            Ok(())
        })
        .await
    }

    async fn set_muted(&self, muted: bool) -> Result<(), MediaError> {
        self.shared.flags.muted.store(muted, Ordering::Relaxed);
        let shared = self.shared.clone();
        self.on_runtime(async move {
            if let Some(s) = shared.session.lock().await.as_ref() {
                if muted {
                    s.audio_track.mute();
                } else {
                    s.audio_track.unmute();
                }
            }
            Ok(())
        })
        .await
    }

    async fn set_deafened(&self, deafened: bool) -> Result<(), MediaError> {
        self.shared.mixer.set_deafened(deafened);
        Ok(())
    }

    async fn set_participant_volume(&self, user_id: &str, gain: f32) -> Result<(), MediaError> {
        let gain = gain.clamp(0.0, 2.0);
        if let Ok(mut v) = self.shared.volumes.lock() {
            v.insert(user_id.to_string(), gain);
        }
        self.shared.mixer.set_user_gain(user_id, gain);
        Ok(())
    }

    async fn set_master_volume(&self, gain: f32) -> Result<(), MediaError> {
        self.shared.mixer.set_master(gain);
        Ok(())
    }

    async fn set_device(&self, kind: DeviceKind, id: Option<String>) -> Result<(), MediaError> {
        let shared = self.shared.clone();
        self.on_runtime(async move {
            let id = id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            match kind {
                DeviceKind::Input => {
                    if let Ok(mut d) = shared.input_device.lock() {
                        *d = id;
                    }
                    if shared.input_running.load(Ordering::Acquire) {
                        shared.input_running.store(false, Ordering::Release);
                        shared.ensure_input().map_err(|e| MediaError::Device(map_io_error(&e, DeviceKind::Input)))?;
                    }
                }
                DeviceKind::Output => {
                    if let Ok(mut d) = shared.output_device.lock() {
                        *d = id;
                    }
                    if shared.output_running.load(Ordering::Acquire) {
                        shared.output_running.store(false, Ordering::Release);
                        shared.ensure_output().map_err(|e| MediaError::Device(map_io_error(&e, DeviceKind::Output)))?;
                    }
                }
            }
            Ok(())
        })
        .await
    }

    async fn set_noise_suppression(&self, on: bool) -> Result<(), MediaError> {
        self.shared.flags.denoise.store(on && cfg!(feature = "rnnoise"), Ordering::Relaxed);
        Ok(())
    }

    async fn set_ptt(&self, enabled: bool, pressed: bool) -> Result<(), MediaError> {
        let flags = self.shared.flags.clone();
        flags.ptt_enabled.store(enabled, Ordering::Relaxed);
        let generation = self.shared.ptt_generation.fetch_add(1, Ordering::AcqRel) + 1;
        if pressed || !enabled {
            flags.ptt_pressed.store(pressed, Ordering::Relaxed);
            return Ok(());
        }
        let shared = self.shared.clone();
        let Some(rt) = self.rt.as_ref() else { return Ok(()) };
        rt.spawn(async move {
            tokio::time::sleep(PTT_RELEASE_DELAY).await;
            if shared.ptt_generation.load(Ordering::Acquire) == generation {
                shared.flags.ptt_pressed.store(false, Ordering::Relaxed);
            }
        });
        Ok(())
    }

    async fn set_mic_monitor(&self, on: bool) -> Result<(), MediaError> {
        let shared = self.shared.clone();
        self.on_runtime(async move {
            shared.monitor.store(on, Ordering::Release);
            shared.flags.monitor.store(on, Ordering::Relaxed);
            if on {
                shared.ensure_input().map_err(|e| MediaError::Device(map_io_error(&e, DeviceKind::Input)))?;
            } else {
                shared.stop_input_if_idle();
            }
            Ok(())
        })
        .await
    }

    async fn set_ducking(&self, percent: u8) -> Result<(), MediaError> {
        self.shared.mixer.set_duck_percent(percent);
        Ok(())
    }

    async fn set_room_key(&self, key: RoomKey) -> Result<(), MediaError> {
        if self.shared.rotation.apply(key) {
            tracing::debug!("[omnidisc-media] voice key rotated to epoch {}", key.epoch);
        }
        Ok(())
    }

    fn e2ee_epoch(&self) -> Option<u64> {
        self.shared.rotation.epoch()
    }

    async fn stats(&self) -> Result<VoiceStats, MediaError> {
        let shared = self.shared.clone();
        self.on_runtime(async move {
            let (track, remotes, participants) = {
                let guard = shared.session.lock().await;
                let Some(s) = guard.as_ref() else { return Err(MediaError::NotConnected) };
                let remotes: Vec<RemoteAudioTrack> =
                    s.remote_audio.lock().map(|m| m.values().cloned().collect()).unwrap_or_default();
                (s.audio_track.clone(), remotes, s.room.remote_participants().len())
            };
            let mut out = VoiceStats { participants, ..Default::default() };
            let mut bytes_out = 0u64;
            let mut bytes_in = 0u64;
            if let Ok(stats) = track.get_stats().await {
                for s in stats {
                    match s {
                        RtcStats::OutboundRtp(o) => bytes_out += o.sent.bytes_sent,
                        RtcStats::RemoteInboundRtp(r) => {
                            if r.remote_inbound.round_trip_time > 0.0 {
                                out.rtt_ms = Some(r.remote_inbound.round_trip_time * 1000.0);
                            }
                            out.packet_loss = Some(r.remote_inbound.fraction_lost.clamp(0.0, 1.0));
                            out.jitter_ms = Some(r.received.jitter * 1000.0);
                        }
                        RtcStats::CandidatePair(p) if p.candidate_pair.nominated => {
                            if out.rtt_ms.is_none() && p.candidate_pair.current_round_trip_time > 0.0 {
                                out.rtt_ms = Some(p.candidate_pair.current_round_trip_time * 1000.0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            let mut lost = 0i64;
            let mut received = 0u64;
            for r in remotes {
                if let Ok(stats) = r.get_stats().await {
                    for s in stats {
                        if let RtcStats::InboundRtp(i) = s {
                            bytes_in += i.inbound.bytes_received;
                            lost += i.received.packets_lost;
                            received += i.received.packets_received;
                        }
                    }
                }
            }
            if out.packet_loss.is_none() && received > 0 {
                out.packet_loss = Some((lost.max(0) as f64 / (received as f64 + lost.max(0) as f64)).clamp(0.0, 1.0));
            }
            let now = Instant::now();
            if let Ok(mut last) = shared.last_stats.lock() {
                if let Some((t, o, i)) = *last {
                    let dt = now.duration_since(t).as_secs_f64();
                    if dt > 0.2 {
                        out.bitrate_out_kbps = (bytes_out.saturating_sub(o) as f64 * 8.0 / 1000.0) / dt;
                        out.bitrate_in_kbps = (bytes_in.saturating_sub(i) as f64 * 8.0 / 1000.0) / dt;
                    }
                }
                *last = Some((now, bytes_out, bytes_in));
            }
            Ok(out)
        })
        .await
    }

    fn devices(&self) -> AudioDevices {
        devices::enumerate()
    }
}
