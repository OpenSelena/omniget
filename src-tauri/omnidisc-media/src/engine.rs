use crate::audio::io::DeviceLoss;
use crate::audio::{AudioDevices, DeviceKind};
use crate::e2ee::RoomKey;
use crate::state::{VoiceEvent, VoiceState, VoiceStateMachine};
use async_trait::async_trait;
use futures::FutureExt;
use omnidisc_proto::gateway::VoiceServerUpdate;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("device error: {0}")]
    Device(String),
    #[error("not connected")]
    NotConnected,
    #[error("media engine panicked")]
    Panicked,
}

impl MediaError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "ERR_VOICE_UNAVAILABLE",
            Self::Connection(_) => "ERR_VOICE_UNREACHABLE",
            Self::Device(_) => "ERR_VOICE_NO_AUDIO_DEVICE",
            Self::NotConnected => "ERR_VOICE_NOT_CONNECTED",
            Self::Panicked => "ERR_VOICE_ENGINE_CRASHED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Excellent,
    Good,
    Poor,
    Lost,
}

/// Where an audio device ended up after it disappeared mid-call. The UI needs
/// the distinction: a device that came back on its own deserves a quiet note,
/// a mic that ended in `ListenOnly` deserves a loud one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Lost,
    Recovered,
    SwitchedToDefault,
    ListenOnly,
    Silent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineNotification {
    State {
        state: VoiceState,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Speaking {
        user_id: String,
        speaking: bool,
    },
    ParticipantJoined {
        user_id: String,
    },
    ParticipantLeft {
        user_id: String,
    },
    Quality {
        user_id: String,
        quality: Quality,
    },
    Level {
        rms_db: f32,
        peak: f32,
    },
    Device {
        kind: DeviceKind,
        status: DeviceStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        cause: Option<DeviceLoss>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug)]
pub enum BackendEvent {
    Transport(VoiceEvent),
    Notify(EngineNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct VoiceStats {
    pub rtt_ms: Option<f64>,
    pub packet_loss: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub bitrate_out_kbps: f64,
    pub bitrate_in_kbps: f64,
    pub participants: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioPrefs {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub noise_suppression: bool,
    pub ptt_enabled: bool,
    pub vad_threshold_db: Option<f32>,
    /// How much of everyone else's audio to duck while the local user speaks,
    /// 0–100 %. 0 is off, which is the default.
    #[serde(default)]
    pub ducking_percent: u8,
}

/// Everything about a connection that is not audio: the MLS room key when the
/// channel has a group, and the ICE policy.
#[derive(Debug, Clone, Default)]
pub struct ConnectOptions {
    pub room_key: Option<RoomKey>,
    /// Force every candidate through a TURN relay so peers never see the local
    /// IP. Useless without a TURN server in `VoiceServerUpdate.ice_servers`.
    pub relay_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mic_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_error: Option<String>,
}

#[async_trait]
pub trait MediaBackend: Send + Sync {
    fn set_events(&self, tx: mpsc::UnboundedSender<BackendEvent>);
    async fn connect(
        &self,
        target: &VoiceServerUpdate,
        prefs: &AudioPrefs,
        options: &ConnectOptions,
    ) -> Result<ConnectOutcome, MediaError>;
    async fn disconnect(&self) -> Result<(), MediaError>;
    async fn set_muted(&self, muted: bool) -> Result<(), MediaError>;
    async fn set_deafened(&self, deafened: bool) -> Result<(), MediaError>;
    async fn set_participant_volume(&self, user_id: &str, gain: f32) -> Result<(), MediaError>;
    async fn set_master_volume(&self, gain: f32) -> Result<(), MediaError>;
    async fn set_device(&self, kind: DeviceKind, id: Option<String>) -> Result<(), MediaError>;
    async fn set_noise_suppression(&self, on: bool) -> Result<(), MediaError>;
    async fn set_ptt(&self, enabled: bool, pressed: bool) -> Result<(), MediaError>;
    async fn set_mic_monitor(&self, on: bool) -> Result<(), MediaError>;
    async fn set_ducking(&self, percent: u8) -> Result<(), MediaError>;
    /// Push the key of a new MLS epoch. A no-op when the room is not encrypted.
    async fn set_room_key(&self, key: RoomKey) -> Result<(), MediaError>;
    fn e2ee_epoch(&self) -> Option<u64>;
    async fn stats(&self) -> Result<VoiceStats, MediaError>;
    fn devices(&self) -> AudioDevices;
}

pub struct MediaEngine {
    backend: Arc<dyn MediaBackend>,
    machine: Mutex<VoiceStateMachine>,
    notify: broadcast::Sender<EngineNotification>,
    backend_rx: Mutex<Option<mpsc::UnboundedReceiver<BackendEvent>>>,
}

async fn guard<T>(fut: impl Future<Output = Result<T, MediaError>>) -> Result<T, MediaError> {
    match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
        Ok(r) => r,
        Err(_) => {
            tracing::error!("[omnidisc-media] backend panicked");
            Err(MediaError::Panicked)
        }
    }
}

impl MediaEngine {
    pub fn new(backend: Arc<dyn MediaBackend>) -> Self {
        let (notify, _) = broadcast::channel(256);
        let (tx, rx) = mpsc::unbounded_channel();
        backend.set_events(tx);
        Self {
            backend,
            machine: Mutex::new(VoiceStateMachine::default()),
            notify,
            backend_rx: Mutex::new(Some(rx)),
        }
    }

    pub fn backend(&self) -> &Arc<dyn MediaBackend> {
        &self.backend
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineNotification> {
        self.notify.subscribe()
    }

    pub async fn state(&self) -> VoiceState {
        self.machine.lock().await.state()
    }

    pub async fn pump(&self) {
        let rx = self.backend_rx.lock().await.take();
        let Some(mut rx) = rx else { return };
        while let Some(ev) = rx.recv().await {
            match ev {
                BackendEvent::Transport(t) => {
                    let before = self.state().await;
                    let after = self.transition(&t, None).await;
                    if after == VoiceState::Failed && before != VoiceState::Failed {
                        let _ = guard(self.backend.disconnect()).await;
                    }
                }
                BackendEvent::Notify(n) => {
                    let _ = self.notify.send(n);
                }
            }
        }
    }

    pub async fn join(
        &self,
        target: &VoiceServerUpdate,
        prefs: &AudioPrefs,
        options: &ConnectOptions,
    ) -> Result<ConnectOutcome, MediaError> {
        self.transition(&VoiceEvent::Join, None).await;
        match guard(self.backend.connect(target, prefs, options)).await {
            Ok(outcome) => {
                self.transition(&VoiceEvent::Connected, None).await;
                Ok(outcome)
            }
            Err(e) => {
                self.transition(
                    &VoiceEvent::Disconnected { recoverable: false },
                    Some(e.code()),
                )
                .await;
                let _ = self.notify.send(EngineNotification::Error {
                    code: e.code().into(),
                    message: e.to_string(),
                });
                let _ = guard(self.backend.disconnect()).await;
                Err(e)
            }
        }
    }

    pub async fn leave(&self) -> Result<VoiceState, MediaError> {
        let r = guard(self.backend.disconnect()).await;
        let s = self.transition(&VoiceEvent::Leave, None).await;
        r.map(|_| s)
    }

    pub async fn set_muted(&self, muted: bool) -> Result<(), MediaError> {
        guard(self.backend.set_muted(muted)).await
    }

    pub async fn set_deafened(&self, deafened: bool) -> Result<(), MediaError> {
        guard(self.backend.set_deafened(deafened)).await
    }

    pub async fn set_participant_volume(&self, user_id: &str, gain: f32) -> Result<(), MediaError> {
        guard(self.backend.set_participant_volume(user_id, gain)).await
    }

    pub async fn set_master_volume(&self, gain: f32) -> Result<(), MediaError> {
        guard(self.backend.set_master_volume(gain)).await
    }

    pub async fn set_device(&self, kind: DeviceKind, id: Option<String>) -> Result<(), MediaError> {
        guard(self.backend.set_device(kind, id)).await
    }

    pub async fn set_noise_suppression(&self, on: bool) -> Result<(), MediaError> {
        guard(self.backend.set_noise_suppression(on)).await
    }

    pub async fn set_ptt(&self, enabled: bool, pressed: bool) -> Result<(), MediaError> {
        guard(self.backend.set_ptt(enabled, pressed)).await
    }

    pub async fn set_mic_monitor(&self, on: bool) -> Result<(), MediaError> {
        guard(self.backend.set_mic_monitor(on)).await
    }

    pub async fn set_ducking(&self, percent: u8) -> Result<(), MediaError> {
        guard(self.backend.set_ducking(percent)).await
    }

    pub async fn set_room_key(&self, key: RoomKey) -> Result<(), MediaError> {
        guard(self.backend.set_room_key(key)).await
    }

    pub fn e2ee_epoch(&self) -> Option<u64> {
        self.backend.e2ee_epoch()
    }

    pub async fn stats(&self) -> Result<VoiceStats, MediaError> {
        guard(self.backend.stats()).await
    }

    pub fn devices(&self) -> AudioDevices {
        self.backend.devices()
    }

    async fn transition(&self, ev: &VoiceEvent, reason: Option<&str>) -> VoiceState {
        let s = self.machine.lock().await.apply(ev);
        let _ = self.notify.send(EngineNotification::State {
            state: s,
            reason: reason.map(str::to_string),
        });
        s
    }
}

pub struct NullBackend;

#[async_trait]
impl MediaBackend for NullBackend {
    fn set_events(&self, _: mpsc::UnboundedSender<BackendEvent>) {}
    async fn connect(
        &self,
        _: &VoiceServerUpdate,
        _: &AudioPrefs,
        _: &ConnectOptions,
    ) -> Result<ConnectOutcome, MediaError> {
        Err(MediaError::Unavailable(
            "no media backend compiled in".into(),
        ))
    }
    async fn disconnect(&self) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_muted(&self, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_deafened(&self, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_participant_volume(&self, _: &str, _: f32) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_master_volume(&self, _: f32) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_device(&self, _: DeviceKind, _: Option<String>) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_noise_suppression(&self, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_ptt(&self, _: bool, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_mic_monitor(&self, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_ducking(&self, _: u8) -> Result<(), MediaError> {
        Ok(())
    }
    async fn set_room_key(&self, _: RoomKey) -> Result<(), MediaError> {
        Ok(())
    }
    fn e2ee_epoch(&self) -> Option<u64> {
        None
    }
    async fn stats(&self) -> Result<VoiceStats, MediaError> {
        Err(MediaError::NotConnected)
    }
    fn devices(&self) -> AudioDevices {
        AudioDevices::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnidisc_proto::Snowflake;

    struct PanickyBackend;

    #[async_trait]
    impl MediaBackend for PanickyBackend {
        fn set_events(&self, _: mpsc::UnboundedSender<BackendEvent>) {}
        async fn connect(
            &self,
            _: &VoiceServerUpdate,
            _: &AudioPrefs,
            _: &ConnectOptions,
        ) -> Result<ConnectOutcome, MediaError> {
            panic!("boom");
        }
        async fn disconnect(&self) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_muted(&self, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_deafened(&self, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_participant_volume(&self, _: &str, _: f32) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_master_volume(&self, _: f32) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_device(&self, _: DeviceKind, _: Option<String>) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_noise_suppression(&self, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_ptt(&self, _: bool, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_mic_monitor(&self, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_ducking(&self, _: u8) -> Result<(), MediaError> {
            Ok(())
        }
        async fn set_room_key(&self, _: RoomKey) -> Result<(), MediaError> {
            Ok(())
        }
        fn e2ee_epoch(&self) -> Option<u64> {
            None
        }
        async fn stats(&self) -> Result<VoiceStats, MediaError> {
            Err(MediaError::NotConnected)
        }
        fn devices(&self) -> AudioDevices {
            AudioDevices::default()
        }
    }

    fn target() -> VoiceServerUpdate {
        VoiceServerUpdate {
            guild_id: None,
            channel_id: Snowflake(1),
            endpoint: "ws://127.0.0.1:1".into(),
            token: "t".into(),
            room: "r".into(),
            ice_servers: vec![],
        }
    }

    #[tokio::test]
    async fn null_backend_join_fails_into_failed_state() {
        let engine = MediaEngine::new(Arc::new(NullBackend));
        let mut rx = engine.subscribe();
        assert!(engine
            .join(
                &target(),
                &AudioPrefs::default(),
                &ConnectOptions::default()
            )
            .await
            .is_err());
        assert_eq!(engine.state().await, VoiceState::Failed);
        let first = rx.recv().await.unwrap();
        assert_eq!(
            first,
            EngineNotification::State {
                state: VoiceState::Connecting,
                reason: None
            }
        );
        assert_eq!(engine.leave().await.unwrap(), VoiceState::Idle);
    }

    #[tokio::test]
    async fn panic_in_backend_is_contained() {
        let engine = MediaEngine::new(Arc::new(PanickyBackend));
        let err = engine
            .join(
                &target(),
                &AudioPrefs::default(),
                &ConnectOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, MediaError::Panicked));
        assert_eq!(engine.state().await, VoiceState::Failed);
    }
}
