use super::gateway::{self, ERR_NOT_CONNECTED};
use super::normalize_instance_url;
use omnidisc_media::{
    AudioDevices, AudioPrefs, ConnectOptions, DeviceKind, LiveKitBackend, MediaEngine, NullBackend,
    RoomKey, VoiceState, VoiceStats,
};
use omnidisc_proto::gateway::{Opcode, VoiceServerUpdate};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::{oneshot, Mutex};

pub const EVENT_VOICE: &str = "omnidisc://voice";
pub const ERR_VOICE_TIMEOUT: &str = "ERR_VOICE_TIMEOUT";
pub const ERR_VOICE_DENIED: &str = "ERR_VOICE_DENIED";
pub const ERR_VOICE_DM_UNSUPPORTED: &str = "ERR_VOICE_DM_UNSUPPORTED";
pub const ERR_VOICE_NOT_CONNECTED: &str = "ERR_VOICE_NOT_CONNECTED";

const SERVER_UPDATE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize)]
pub struct VoiceSessionInfo {
    pub url: String,
    /// `None` for a DM or group-DM call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    pub channel_id: String,
    /// The SFU room name. It is also the domain separator the MLS exporter is
    /// bound to, so both ends derive the same voice key without agreeing on
    /// anything extra.
    pub room: String,
    /// Whether frames on this call are end-to-end encrypted. Never guessed:
    /// it is true only when a room key was actually handed to the cryptor.
    pub e2ee: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Flags {
    muted: bool,
    deafened: bool,
    muted_before_deafen: bool,
    streaming: bool,
}

struct PendingJoin {
    url: String,
    channel_id: String,
    tx: oneshot::Sender<Result<VoiceServerUpdate, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceStatus {
    pub state: VoiceState,
    pub muted: bool,
    pub deafened: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<VoiceSessionInfo>,
    pub backend_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JoinResult {
    pub state: VoiceState,
    pub session: VoiceSessionInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mic_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_error: Option<String>,
}

pub struct VoiceManager {
    engine: Arc<MediaEngine>,
    backend: Option<Arc<omnidisc_media::LiveKitBackend>>,
    backend_available: bool,
    session: Mutex<Option<VoiceSessionInfo>>,
    pending: StdMutex<Option<PendingJoin>>,
    flags: StdMutex<Flags>,
    started: AtomicBool,
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceManager {
    pub fn new() -> Self {
        let (engine, backend, backend_available) = match LiveKitBackend::new() {
            Ok(backend) => {
                let backend = Arc::new(backend);
                (MediaEngine::new(backend.clone()), Some(backend), true)
            }
            Err(e) => {
                tracing::error!("[omnidisc] voice backend unavailable: {}", e);
                (MediaEngine::new(Arc::new(NullBackend)), None, false)
            }
        };
        Self {
            engine: Arc::new(engine),
            backend,
            backend_available,
            session: Mutex::new(None),
            pending: StdMutex::new(None),
            flags: StdMutex::new(Flags::default()),
            started: AtomicBool::new(false),
        }
    }

    pub fn engine(&self) -> &Arc<MediaEngine> {
        &self.engine
    }

    pub fn livekit_backend(&self) -> Option<Arc<omnidisc_media::LiveKitBackend>> {
        self.backend.clone()
    }

    pub async fn session_info(&self) -> Option<VoiceSessionInfo> {
        self.session.lock().await.clone()
    }

    fn flags(&self) -> Flags {
        self.flags.lock().map(|f| *f).unwrap_or_default()
    }

    fn set_flags(&self, f: Flags) {
        if let Ok(mut g) = self.flags.lock() {
            *g = f;
        }
    }

    async fn session_url(&self) -> Option<String> {
        self.session.lock().await.as_ref().map(|s| s.url.clone())
    }

    fn take_pending_if(&self, url: &str, channel_id: Option<&str>) -> Option<PendingJoin> {
        let mut guard = self.pending.lock().ok()?;
        let matches = guard
            .as_ref()
            .map(|p| p.url == url && channel_id.map(|c| c == p.channel_id).unwrap_or(true))
            .unwrap_or(false);
        if matches {
            guard.take()
        } else {
            None
        }
    }

    pub fn resolve_server_update(&self, url: &str, update: VoiceServerUpdate) {
        if let Some(p) = self.take_pending_if(url, Some(&update.channel_id.to_string())) {
            let _ = p.tx.send(Ok(update));
        }
    }

    pub fn reject_pending(&self, url: &str, reason: String) {
        if let Some(p) = self.take_pending_if(url, None) {
            let _ = p.tx.send(Err(reason));
        }
    }
}

pub fn on_dispatch(app: &tauri::AppHandle, url: &str, t: &str, d: &Value) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    match t {
        "READY" | "RESUMED" => {
            let manager = state.omnidisc_voice.clone();
            let gateways = state.omnidisc_gateways.clone();
            let url = url.to_string();
            tauri::async_runtime::spawn(async move {
                let session = manager.session.lock().await.clone();
                let Some(s) = session.filter(|s| s.url == url) else {
                    return;
                };
                if manager.engine.state().await != VoiceState::Connected {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
                let flags = manager.flags();
                if let Err(e) = send_voice_state(
                    &gateways,
                    &s.url,
                    s.guild_id.as_deref(),
                    Some(&s.channel_id),
                    flags,
                )
                .await
                {
                    tracing::debug!(
                        "[omnidisc] voice state not re-announced after resume: {}",
                        e
                    );
                }
            });
        }
        "VOICE_SERVER_UPDATE" => match serde_json::from_value::<VoiceServerUpdate>(d.clone()) {
            Ok(update) => state.omnidisc_voice.resolve_server_update(url, update),
            Err(e) => tracing::warn!(
                "[omnidisc] malformed VOICE_SERVER_UPDATE from {}: {}",
                url,
                e
            ),
        },
        "GATEWAY_ERROR" if d.get("code").and_then(Value::as_str) == Some("voice") => {
            let reason = d
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            tracing::warn!("[omnidisc] voice request rejected by {}: {}", url, reason);
            state
                .omnidisc_voice
                .reject_pending(url, ERR_VOICE_DENIED.to_string());
        }
        _ => {}
    }
}

pub fn start(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let manager = state.omnidisc_voice.clone();
    if manager.started.swap(true, Ordering::AcqRel) {
        return;
    }
    super::ducking::opt_out_of_system_ducking();
    let pump_engine = manager.engine.clone();
    tauri::async_runtime::spawn(async move { pump_engine.pump().await });
    let app = app.clone();
    let forward = manager.clone();
    tauri::async_runtime::spawn(async move {
        let mut rx = forward.engine.subscribe();
        loop {
            match rx.recv().await {
                Ok(n) => {
                    let url = forward.session_url().await;
                    let mut payload = serde_json::to_value(&n).unwrap_or(Value::Null);
                    if let Value::Object(map) = &mut payload {
                        map.insert("url".into(), url.map(Value::String).unwrap_or(Value::Null));
                    }
                    let _ = app.emit(EVENT_VOICE, payload);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!("[omnidisc] voice event forwarder lagged by {}", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

pub fn ptt_from_hotkey(app: &tauri::AppHandle, pressed: bool) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let manager = state.omnidisc_voice.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = manager.engine.set_ptt(true, pressed).await {
            tracing::debug!("[omnidisc] ptt: {}", e);
        }
    });
}

fn prefs_from_settings(app: &tauri::AppHandle) -> (AudioPrefs, bool) {
    let s = crate::storage::config::load_settings(app).omnidisc.voice;
    let relay_only = s.relay_only;
    (
        AudioPrefs {
            input_device: s.input_device.filter(|d| !d.trim().is_empty()),
            output_device: s.output_device.filter(|d| !d.trim().is_empty()),
            noise_suppression: s.noise_suppression,
            ptt_enabled: !s.ptt_key.trim().is_empty(),
            vad_threshold_db: Some(s.vad_threshold_db),
            ducking_percent: s.ducking_percent.min(100),
        },
        relay_only,
    )
}

async fn send_voice_state(
    gateways: &gateway::Gateways,
    url: &str,
    guild_id: Option<&str>,
    channel_id: Option<&str>,
    flags: Flags,
) -> Result<(), String> {
    gateway::send(
        gateways,
        url,
        Opcode::VoiceStateUpdate as u8,
        json!({
            "guild_id": guild_id,
            "channel_id": channel_id,
            "self_mute": flags.muted,
            "self_deaf": flags.deafened,
            "self_stream": flags.streaming,
        }),
    )
    .await
}

impl VoiceManager {
    pub fn set_streaming(&self, on: bool) {
        if let Ok(mut g) = self.flags.lock() {
            g.streaming = on;
        }
    }

    pub async fn announce_stream(&self, gateways: &gateway::Gateways) {
        push_flags(self, gateways).await;
    }
}

async fn leave_current(state: &crate::AppState) {
    let manager = &state.omnidisc_voice;
    let gateways = &state.omnidisc_gateways;
    super::stream::stop_active_for_leave(state).await;
    let previous = manager.session.lock().await.take();
    if let Err(e) = manager.engine.leave().await {
        tracing::warn!("[omnidisc] voice leave: {}", e);
    }
    if let Some(s) = previous {
        let flags = manager.flags();
        if let Err(e) = send_voice_state(gateways, &s.url, s.guild_id.as_deref(), None, flags).await
        {
            tracing::debug!("[omnidisc] voice leave not sent to {}: {}", s.url, e);
        }
    }
}

#[tauri::command]
pub async fn omnidisc_voice_join(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    url: String,
    guild_id: Option<String>,
    channel_id: String,
    recipient_ids: Option<Vec<String>>,
) -> Result<JoinResult, String> {
    let base = normalize_instance_url(&url)?;
    let guild_id = guild_id
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty());
    let manager = state.omnidisc_voice.clone();
    if !gateway::is_ready(&state.omnidisc_gateways, &base).await {
        return Err(ERR_NOT_CONNECTED.to_string());
    }

    {
        let current = manager.session.lock().await.clone();
        if let Some(s) = current {
            if s.url == base
                && s.channel_id == channel_id
                && manager.engine.state().await == VoiceState::Connected
            {
                return Ok(JoinResult {
                    state: VoiceState::Connected,
                    session: s,
                    mic_error: None,
                    output_error: None,
                });
            }
            leave_current(&state).await;
        }
    }

    let (tx, rx) = oneshot::channel();
    if let Ok(mut p) = manager.pending.lock() {
        *p = Some(PendingJoin {
            url: base.clone(),
            channel_id: channel_id.clone(),
            tx,
        });
    }
    let flags = manager.flags();
    if let Err(e) = send_voice_state(
        &state.omnidisc_gateways,
        &base,
        guild_id.as_deref(),
        Some(&channel_id),
        flags,
    )
    .await
    {
        let _ = manager.take_pending_if(&base, None);
        return Err(e);
    }
    // The gateway was ready before we sent, so silence on a DM means the server
    // predates DM calls and dropped op 4 without a guild — say that instead of
    // a bare timeout.
    let timeout_code = if guild_id.is_some() {
        ERR_VOICE_TIMEOUT
    } else {
        ERR_VOICE_DM_UNSUPPORTED
    };
    let update = match tokio::time::timeout(SERVER_UPDATE_TIMEOUT, rx).await {
        Ok(Ok(Ok(update))) => update,
        Ok(Ok(Err(reason))) => return Err(reason),
        Ok(Err(_)) => return Err(timeout_code.to_string()),
        Err(_) => {
            let _ = manager.take_pending_if(&base, None);
            return Err(timeout_code.to_string());
        }
    };

    // The key never leaves this process and never touches the server: it is
    // exported from the channel's MLS group, bound to the room name. No group
    // means no key, which means an honest "not encrypted" in the UI.
    let establish = recipient_ids.as_deref().filter(|r| !r.is_empty());
    let room_key = super::mls::voice_key_for(&state.omnidisc_mls, &base, &channel_id, establish)
        .await
        .map(|(epoch, key)| RoomKey::new(epoch, key));
    let session = VoiceSessionInfo {
        url: base.clone(),
        guild_id: guild_id.clone(),
        channel_id: channel_id.clone(),
        room: update.room.clone(),
        e2ee: room_key.is_some(),
    };
    *manager.session.lock().await = Some(session.clone());
    let (prefs, relay_only) = prefs_from_settings(&app);
    let options = ConnectOptions {
        room_key,
        relay_only,
    };
    match manager.engine.join(&update, &prefs, &options).await {
        Ok(outcome) => {
            let _ = manager.engine.set_muted(flags.muted).await;
            let _ = manager.engine.set_deafened(flags.deafened).await;
            Ok(JoinResult {
                state: VoiceState::Connected,
                session,
                mic_error: outcome.mic_error,
                output_error: outcome.output_error,
            })
        }
        Err(e) => {
            *manager.session.lock().await = None;
            let _ = send_voice_state(
                &state.omnidisc_gateways,
                &base,
                guild_id.as_deref(),
                None,
                flags,
            )
            .await;
            Err(e.code().to_string())
        }
    }
}

/// Which channel this app is talking in on `base`. Used by the MLS layer to
/// rekey an active call; the key is bound to the channel id and not to the
/// room name, which the server chooses.
pub async fn channel_in_call_on(app: &tauri::AppHandle, base: &str) -> Option<String> {
    let state = app.try_state::<crate::AppState>()?;
    let session = state.omnidisc_voice.session_info().await?;
    (session.url == base && session.e2ee).then_some(session.channel_id)
}

/// Push a freshly exported epoch key into the running call.
pub async fn push_room_key(app: &tauri::AppHandle, epoch: u64, key: [u8; 32]) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    if let Err(e) = state
        .omnidisc_voice
        .engine
        .set_room_key(RoomKey::new(epoch, key))
        .await
    {
        tracing::warn!("[omnidisc] the voice key could not be rotated: {}", e);
    }
}

#[tauri::command]
pub async fn omnidisc_voice_leave(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    leave_current(&state).await;
    Ok(())
}

async fn push_flags(manager: &VoiceManager, gateways: &gateway::Gateways) {
    let flags = manager.flags();
    let session = manager.session.lock().await.clone();
    if let Some(s) = session {
        if let Err(e) = send_voice_state(
            gateways,
            &s.url,
            s.guild_id.as_deref(),
            Some(&s.channel_id),
            flags,
        )
        .await
        {
            tracing::debug!("[omnidisc] voice flags not sent: {}", e);
        }
    }
}

#[tauri::command]
pub async fn omnidisc_voice_set_mute(
    state: tauri::State<'_, crate::AppState>,
    muted: bool,
) -> Result<VoiceStatus, String> {
    let manager = state.omnidisc_voice.clone();
    let mut flags = manager.flags();
    flags.muted = muted;
    if !muted && flags.deafened {
        flags.deafened = false;
        manager
            .engine
            .set_deafened(false)
            .await
            .map_err(|e| e.code().to_string())?;
    }
    manager.set_flags(flags);
    manager
        .engine
        .set_muted(muted)
        .await
        .map_err(|e| e.code().to_string())?;
    push_flags(&manager, &state.omnidisc_gateways).await;
    status_of(&manager).await
}

#[tauri::command]
pub async fn omnidisc_voice_set_deaf(
    state: tauri::State<'_, crate::AppState>,
    deafened: bool,
) -> Result<VoiceStatus, String> {
    let manager = state.omnidisc_voice.clone();
    let mut flags = manager.flags();
    if deafened && !flags.deafened {
        flags.muted_before_deafen = flags.muted;
        flags.muted = true;
    } else if !deafened && flags.deafened {
        flags.muted = flags.muted_before_deafen;
    }
    flags.deafened = deafened;
    manager.set_flags(flags);
    manager
        .engine
        .set_deafened(deafened)
        .await
        .map_err(|e| e.code().to_string())?;
    manager
        .engine
        .set_muted(flags.muted)
        .await
        .map_err(|e| e.code().to_string())?;
    push_flags(&manager, &state.omnidisc_gateways).await;
    status_of(&manager).await
}

#[tauri::command]
pub async fn omnidisc_voice_set_volume(
    state: tauri::State<'_, crate::AppState>,
    user_id: String,
    gain: f32,
) -> Result<(), String> {
    if !gain.is_finite() {
        return Err("ERR_BAD_REQUEST".to_string());
    }
    state
        .omnidisc_voice
        .engine
        .set_participant_volume(&user_id, gain.clamp(0.0, 2.0))
        .await
        .map_err(|e| e.code().to_string())
}

#[tauri::command]
pub async fn omnidisc_voice_devices(
    state: tauri::State<'_, crate::AppState>,
) -> Result<AudioDevices, String> {
    let engine = state.omnidisc_voice.engine.clone();
    tokio::task::spawn_blocking(move || engine.devices())
        .await
        .map_err(|e| format!("OmniDisc: device enumeration failed: {}", e))
}

#[tauri::command]
pub async fn omnidisc_voice_set_device(
    state: tauri::State<'_, crate::AppState>,
    kind: DeviceKind,
    id: Option<String>,
) -> Result<(), String> {
    state
        .omnidisc_voice
        .engine
        .set_device(kind, id)
        .await
        .map_err(|e| e.code().to_string())
}

#[tauri::command]
pub async fn omnidisc_voice_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<VoiceStats, String> {
    state
        .omnidisc_voice
        .engine
        .stats()
        .await
        .map_err(|e| e.code().to_string())
}

#[tauri::command]
pub async fn omnidisc_voice_ptt(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    pressed: bool,
) -> Result<(), String> {
    let enabled = !crate::storage::config::load_settings(&app)
        .omnidisc
        .voice
        .ptt_key
        .trim()
        .is_empty();
    state
        .omnidisc_voice
        .engine
        .set_ptt(enabled, pressed)
        .await
        .map_err(|e| e.code().to_string())
}

/// Whether the OS actually handed us the push-to-talk key.
///
/// It can refuse: Windows gives a combination to the first process that asks
/// and answers everyone else with `ERROR_HOTKEY_ALREADY_REGISTERED`, and macOS
/// needs Accessibility permission. Both used to fail into a log line, so the
/// key simply did nothing and the app looked broken.
#[derive(Debug, Clone, Serialize)]
pub struct PttStatus {
    pub binding: String,
    pub registered: bool,
}

#[tauri::command]
pub async fn omnidisc_voice_ptt_status(app: tauri::AppHandle) -> Result<PttStatus, String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let binding = crate::storage::config::load_settings(&app)
        .omnidisc
        .voice
        .ptt_key
        .trim()
        .to_string();
    if binding.is_empty() {
        return Ok(PttStatus {
            binding,
            registered: false,
        });
    }
    let registered = binding
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map(|s| app.global_shortcut().is_registered(s))
        .unwrap_or(false);
    Ok(PttStatus {
        binding,
        registered,
    })
}

#[tauri::command]
pub async fn omnidisc_voice_set_noise_suppression(
    state: tauri::State<'_, crate::AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .omnidisc_voice
        .engine
        .set_noise_suppression(enabled)
        .await
        .map_err(|e| e.code().to_string())
}

#[tauri::command]
pub async fn omnidisc_voice_set_ducking(
    state: tauri::State<'_, crate::AppState>,
    percent: u8,
) -> Result<(), String> {
    state
        .omnidisc_voice
        .engine
        .set_ducking(percent.min(100))
        .await
        .map_err(|e| e.code().to_string())
}

#[tauri::command]
pub async fn omnidisc_voice_mic_test(
    state: tauri::State<'_, crate::AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .omnidisc_voice
        .engine
        .set_mic_monitor(enabled)
        .await
        .map_err(|e| e.code().to_string())
}

async fn status_of(manager: &VoiceManager) -> Result<VoiceStatus, String> {
    let flags = manager.flags();
    Ok(VoiceStatus {
        state: manager.engine.state().await,
        muted: flags.muted,
        deafened: flags.deafened,
        session: manager.session.lock().await.clone(),
        backend_available: manager.backend_available,
    })
}

#[tauri::command]
pub async fn omnidisc_voice_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<VoiceStatus, String> {
    status_of(&state.omnidisc_voice).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnidisc_proto::Snowflake;

    fn update(channel: u64) -> VoiceServerUpdate {
        VoiceServerUpdate {
            guild_id: Some(Snowflake(1)),
            channel_id: Snowflake(channel),
            endpoint: "ws://sfu".into(),
            token: "t".into(),
            room: "r".into(),
            ice_servers: vec![],
        }
    }

    /// The plumbing that turns an MLS epoch into a `set_shared_key` call. The
    /// MLS derivation itself is tested in `omnidisc-mls`; what matters here is
    /// that two members feed the ring the same bytes and that a commit moves
    /// the index.
    #[test]
    fn an_epoch_change_rotates_the_key_ring_the_same_way_for_both_members() {
        use omnidisc_media::{KeyRing, KeyRotation, RoomKey};
        use omnidisc_mls::{ClaimedDevice, DeviceRef, MlsClient};
        use std::sync::{Arc, Mutex as StdMutex};

        #[derive(Default)]
        struct Recorder(StdMutex<Vec<(i32, Vec<u8>)>>);
        impl KeyRing for Recorder {
            fn set_shared_key(&self, key: &[u8], index: i32) {
                if let Ok(mut c) = self.0.lock() {
                    c.push((index, key.to_vec()));
                }
            }
        }

        const GROUP: &str = "od-42";
        const ROOM: &[u8] = b"dm-c42";
        let mut alice = MlsClient::new("1001", "desktop-a", &[71u8; 32]).expect("alice");
        let mut bob = MlsClient::new("2002", "phone-bbb", &[72u8; 32]).expect("bob");
        let mut carol = MlsClient::new("3003", "tablet-cc", &[73u8; 32]).expect("carol");
        alice.create_group(GROUP).expect("group");
        let claim = |c: &mut MlsClient| ClaimedDevice {
            device: DeviceRef::new(c.user_id(), c.device_id(), c.public_key().to_vec()),
            key_package: c.key_packages(1, false).expect("kp").remove(0),
        };
        let alice_ref = DeviceRef::new(
            alice.user_id(),
            alice.device_id(),
            alice.public_key().to_vec(),
        );
        let bob_kp = claim(&mut bob);
        let welcome = alice.add_members(GROUP, &[bob_kp]).expect("add");
        alice.merge_pending(GROUP).expect("merge");
        bob.join_welcome(
            &welcome.welcome.clone().expect("welcome"),
            GROUP,
            &[alice_ref],
        )
        .expect("join");

        let epoch = alice.epoch(GROUP).expect("epoch");
        assert_eq!(epoch, bob.epoch(GROUP).expect("epoch"));
        let key_a = alice.voice_key(GROUP, ROOM).expect("key");
        assert_eq!(key_a, bob.voice_key(GROUP, ROOM).expect("key"));

        let ring_a = Arc::new(Recorder::default());
        let ring_b = Arc::new(Recorder::default());
        let rot_a = KeyRotation::new();
        let rot_b = KeyRotation::new();
        rot_a.arm(ring_a.clone(), RoomKey::new(epoch, key_a));
        rot_b.arm(
            ring_b.clone(),
            RoomKey::new(epoch, bob.voice_key(GROUP, ROOM).expect("key")),
        );

        let carol_kp = claim(&mut carol);
        let commit = alice.add_members(GROUP, &[carol_kp]).expect("add carol");
        alice.merge_pending(GROUP).expect("merge");
        bob.process(GROUP, &commit.commit)
            .expect("bob processes the commit");

        let next = alice.epoch(GROUP).expect("epoch");
        assert_eq!(next, epoch + 1);
        let key_next = alice.voice_key(GROUP, ROOM).expect("key");
        assert_ne!(key_next, key_a, "the key must change with the epoch");
        assert_eq!(key_next, bob.voice_key(GROUP, ROOM).expect("key"));

        assert!(rot_a.apply(RoomKey::new(next, key_next)));
        assert!(rot_b.apply(RoomKey::new(next, bob.voice_key(GROUP, ROOM).expect("key"))));
        let calls_a = ring_a.0.lock().expect("lock").clone();
        let calls_b = ring_b.0.lock().expect("lock").clone();
        assert_eq!(calls_a, calls_b, "both members key the ring identically");
        assert_eq!(calls_a.len(), 2);
        assert_eq!(calls_a[0].0, (epoch % 16) as i32);
        assert_eq!(calls_a[1].0, (next % 16) as i32);
        assert_ne!(calls_a[0].1, calls_a[1].1);
    }

    #[tokio::test]
    async fn server_update_resolves_only_the_matching_pending_join() {
        let manager = VoiceManager::new();
        let (tx, rx) = oneshot::channel();
        *manager.pending.lock().unwrap() = Some(PendingJoin {
            url: "https://a".into(),
            channel_id: "7".into(),
            tx,
        });
        manager.resolve_server_update("https://b", update(7));
        assert!(manager.pending.lock().unwrap().is_some());
        manager.resolve_server_update("https://a", update(8));
        assert!(manager.pending.lock().unwrap().is_some());
        manager.resolve_server_update("https://a", update(7));
        assert!(manager.pending.lock().unwrap().is_none());
        assert_eq!(rx.await.unwrap().unwrap().channel_id, Snowflake(7));
    }

    #[tokio::test]
    async fn gateway_voice_error_rejects_pending_join() {
        let manager = VoiceManager::new();
        let (tx, rx) = oneshot::channel();
        *manager.pending.lock().unwrap() = Some(PendingJoin {
            url: "https://a".into(),
            channel_id: "7".into(),
            tx,
        });
        manager.reject_pending("https://a", ERR_VOICE_DENIED.into());
        assert_eq!(rx.await.unwrap().unwrap_err(), ERR_VOICE_DENIED);
    }

    #[test]
    fn deafen_implies_mute_and_restores_it() {
        let mut f = Flags::default();
        f.muted_before_deafen = f.muted;
        f.muted = true;
        f.deafened = true;
        assert!(f.muted && f.deafened);
        f.muted = f.muted_before_deafen;
        f.deafened = false;
        assert!(!f.muted && !f.deafened);
    }
}
