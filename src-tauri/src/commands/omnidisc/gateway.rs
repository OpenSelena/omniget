use super::api::{ERR_NO_SESSION, ERR_UNAUTHORIZED, ERR_UNREACHABLE};
use super::{normalize_instance_url, store};
use futures::{FutureExt, SinkExt, StreamExt};
use omnidisc_proto::gateway::{Opcode, HEARTBEAT_INTERVAL_MS};
use omnidisc_proto::PROTOCOL_VERSION;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

pub const EVENT_DISPATCH: &str = "omnidisc://dispatch";
pub const EVENT_STATUS: &str = "omnidisc://status";
pub const ERR_NOT_CONNECTED: &str = "ERR_NOT_CONNECTED";
pub const ERR_PROTOCOL: &str = "ERR_PROTOCOL";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Connecting,
    Ready,
    Reconnecting,
    Disconnected,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Ready => "ready",
            Self::Reconnecting => "reconnecting",
            Self::Disconnected => "disconnected",
        }
    }
}

pub trait GatewaySink: Send + Sync {
    fn dispatch(&self, url: &str, t: &str, d: Value);
    fn status(&self, url: &str, status: Status, error: Option<&str>);
}

struct TauriSink(tauri::AppHandle);

impl GatewaySink for TauriSink {
    fn dispatch(&self, url: &str, t: &str, d: Value) {
        super::voice::on_dispatch(&self.0, url, t, &d);
        super::mls::on_dispatch(&self.0, url, t, &d);
        let _ = self.0.emit(EVENT_DISPATCH, json!({ "url": url, "t": t, "d": d }));
    }

    fn status(&self, url: &str, status: Status, error: Option<&str>) {
        let _ = self
            .0
            .emit(EVENT_STATUS, json!({ "url": url, "status": status.as_str(), "error": error }));
    }
}

#[derive(Clone, Serialize)]
pub struct StatusSnapshot {
    pub url: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct GatewayHandle {
    cancel: CancellationToken,
    outbound: mpsc::UnboundedSender<String>,
    shared: Arc<Shared>,
}

pub type Gateways = Arc<Mutex<HashMap<String, GatewayHandle>>>;

pub fn new_gateways() -> Gateways {
    Arc::new(Mutex::new(HashMap::new()))
}

struct Shared {
    url: String,
    sink: Arc<dyn GatewaySink>,
    status: std::sync::Mutex<(Status, Option<String>)>,
    finished: std::sync::atomic::AtomicBool,
}

impl Shared {
    fn set_status(&self, status: Status, error: Option<&str>) {
        if let Ok(mut s) = self.status.lock() {
            *s = (status, error.map(str::to_string));
        }
        self.sink.status(&self.url, status, error);
    }

    fn snapshot(&self) -> StatusSnapshot {
        let (status, error) = self
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or((Status::Disconnected, None));
        StatusSnapshot { url: self.url.clone(), status, error }
    }

    fn is_finished(&self) -> bool {
        self.finished.load(std::sync::atomic::Ordering::Acquire)
    }
}

enum Outcome {
    Cancelled,
    Stop(String),
    Retry { resume: bool, error: Option<String> },
}

#[derive(Default)]
struct SessionState {
    session_id: Option<String>,
    seq: u64,
    attempt: u32,
}

pub fn ws_url(base: &str) -> String {
    let ws = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        base.to_string()
    };
    format!("{}/gateway", ws)
}

pub fn backoff(attempt: u32) -> Duration {
    let exp = BACKOFF_MIN.saturating_mul(1u32 << attempt.min(5));
    let capped = exp.min(BACKOFF_MAX);
    let jitter_ms: u64 = rand::RngExt::random_range(&mut rand::rng(), 0..1000);
    capped + Duration::from_millis(jitter_ms)
}

fn frame_text(op: Opcode, d: Value) -> String {
    json!({ "op": op as u8, "d": d }).to_string()
}

pub fn identify_payload(token: &str) -> Value {
    json!({
        "token": token,
        "protocol_version": PROTOCOL_VERSION,
        "compress": "none",
        "properties": {
            "os": std::env::consts::OS,
            "client": "omniget",
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}

pub async fn run_gateway(
    base: String,
    token: String,
    outbound: mpsc::UnboundedReceiver<String>,
    cancel: CancellationToken,
    sink: Arc<dyn GatewaySink>,
) {
    let shared = Arc::new(Shared {
        url: base.clone(),
        sink,
        status: std::sync::Mutex::new((Status::Connecting, None)),
        finished: std::sync::atomic::AtomicBool::new(false),
    });
    run_with_shared(base, token, outbound, cancel, shared).await;
}

async fn run_with_shared(
    base: String,
    token: String,
    mut outbound: mpsc::UnboundedReceiver<String>,
    cancel: CancellationToken,
    shared: Arc<Shared>,
) {
    let mut state = SessionState::default();
    let url = ws_url(&base);
    loop {
        if cancel.is_cancelled() {
            shared.set_status(Status::Disconnected, None);
            break;
        }
        let status = if state.attempt == 0 && state.session_id.is_none() {
            Status::Connecting
        } else {
            Status::Reconnecting
        };
        shared.set_status(status, None);
        let outcome = connect_once(&url, &token, &mut outbound, &cancel, &shared, &mut state).await;
        match outcome {
            Outcome::Cancelled => {
                shared.set_status(Status::Disconnected, None);
                break;
            }
            Outcome::Stop(error) => {
                tracing::warn!("[omnidisc] gateway {} stopped: {}", base, error);
                shared.set_status(Status::Disconnected, Some(&error));
                break;
            }
            Outcome::Retry { resume, error } => {
                if !resume {
                    state.session_id = None;
                    state.seq = 0;
                }
                let delay = backoff(state.attempt);
                state.attempt = state.attempt.saturating_add(1);
                tracing::info!(
                    "[omnidisc] gateway {} dropped ({}); retry in {:?}",
                    base,
                    error.as_deref().unwrap_or("closed"),
                    delay
                );
                shared.set_status(Status::Reconnecting, error.as_deref());
                tokio::select! {
                    _ = cancel.cancelled() => {
                        shared.set_status(Status::Disconnected, None);
                        break;
                    }
                    _ = tokio::time::sleep(delay) => {}
                }
            }
        }
    }
    shared.finished.store(true, std::sync::atomic::Ordering::Release);
}

async fn connect_once(
    url: &str,
    token: &str,
    outbound: &mut mpsc::UnboundedReceiver<String>,
    cancel: &CancellationToken,
    shared: &Shared,
    state: &mut SessionState,
) -> Outcome {
    let connect = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url));
    let ws = tokio::select! {
        _ = cancel.cancelled() => return Outcome::Cancelled,
        res = connect => match res {
            Ok(Ok((ws, _))) => ws,
            Ok(Err(e)) => {
                tracing::warn!("[omnidisc] gateway connect {} failed: {}", url, e);
                return Outcome::Retry { resume: true, error: Some(ERR_UNREACHABLE.to_string()) };
            }
            Err(_) => return Outcome::Retry { resume: true, error: Some(ERR_UNREACHABLE.to_string()) },
        },
    };
    let (mut tx, mut rx) = ws.split();

    let hello = tokio::select! {
        _ = cancel.cancelled() => return Outcome::Cancelled,
        msg = tokio::time::timeout(HELLO_TIMEOUT, rx.next()) => msg,
    };
    let heartbeat_ms = match hello {
        Ok(Some(Ok(Message::Text(t)))) => match serde_json::from_str::<Value>(&t) {
            Ok(v) if v.get("op").and_then(Value::as_u64) == Some(Opcode::Hello as u64) => v
                .get("d")
                .and_then(|d| d.get("heartbeat_interval"))
                .and_then(Value::as_u64)
                .unwrap_or(HEARTBEAT_INTERVAL_MS),
            _ => return Outcome::Retry { resume: true, error: Some(ERR_PROTOCOL.to_string()) },
        },
        _ => return Outcome::Retry { resume: true, error: Some(ERR_UNREACHABLE.to_string()) },
    };

    let first = match &state.session_id {
        Some(sid) if state.seq > 0 => frame_text(
            Opcode::Resume,
            json!({ "token": token, "session_id": sid, "seq": state.seq }),
        ),
        _ => frame_text(Opcode::Identify, identify_payload(token)),
    };
    if tx.send(Message::Text(first.into())).await.is_err() {
        return Outcome::Retry { resume: true, error: Some(ERR_UNREACHABLE.to_string()) };
    }

    let mut heartbeat = tokio::time::interval(Duration::from_millis(heartbeat_ms.max(1000)));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await;
    let mut ack_pending = false;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = tx.send(Message::Close(None)).await;
                return Outcome::Cancelled;
            }
            out = outbound.recv() => {
                match out {
                    Some(text) => {
                        if tx.send(Message::Text(text.into())).await.is_err() {
                            return Outcome::Retry { resume: true, error: None };
                        }
                    }
                    None => return Outcome::Cancelled,
                }
            }
            _ = heartbeat.tick() => {
                if ack_pending {
                    return Outcome::Retry { resume: true, error: Some("heartbeat timeout".to_string()) };
                }
                let beat = frame_text(Opcode::Heartbeat, json!(state.seq));
                if tx.send(Message::Text(beat.into())).await.is_err() {
                    return Outcome::Retry { resume: true, error: None };
                }
                ack_pending = true;
            }
            msg = rx.next() => {
                let text = match msg {
                    None | Some(Err(_)) => return Outcome::Retry { resume: true, error: None },
                    Some(Ok(Message::Text(t))) => t.to_string(),
                    Some(Ok(Message::Binary(b))) => String::from_utf8_lossy(&b).to_string(),
                    Some(Ok(Message::Close(frame))) => {
                        let code = frame.as_ref().map(|f| u16::from(f.code)).unwrap_or(1000);
                        return close_outcome(code);
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = tx.send(Message::Pong(p)).await;
                        continue;
                    }
                    Some(Ok(_)) => continue,
                };
                let frame: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let op = frame.get("op").and_then(Value::as_u64).unwrap_or(u64::MAX);
                if op == Opcode::Dispatch as u64 {
                    if let Some(s) = frame.get("s").and_then(Value::as_u64) {
                        state.seq = s;
                    }
                    let t = frame.get("t").and_then(Value::as_str).unwrap_or("").to_string();
                    let d = frame.get("d").cloned().unwrap_or(Value::Null);
                    let became_ready = t == "READY" || t == "RESUMED";
                    if t == "READY" {
                        state.session_id = d.get("session_id").and_then(Value::as_str).map(str::to_string);
                    }
                    shared.sink.dispatch(&shared.url, &t, d);
                    if became_ready {
                        state.attempt = 0;
                        shared.set_status(Status::Ready, None);
                    }
                } else if op == Opcode::HeartbeatAck as u64 {
                    ack_pending = false;
                } else if op == Opcode::Heartbeat as u64 {
                    let beat = frame_text(Opcode::Heartbeat, json!(state.seq));
                    if tx.send(Message::Text(beat.into())).await.is_err() {
                        return Outcome::Retry { resume: true, error: None };
                    }
                } else if op == Opcode::Reconnect as u64 {
                    return Outcome::Retry { resume: true, error: None };
                } else if op == Opcode::InvalidSession as u64 {
                    let resumable = frame.get("d").and_then(Value::as_bool).unwrap_or(false);
                    return Outcome::Retry { resume: resumable, error: None };
                } else if op == Opcode::GatewayError as u64 {
                    let d = frame.get("d").cloned().unwrap_or(Value::Null);
                    shared.sink.dispatch(&shared.url, "GATEWAY_ERROR", d);
                }
            }
        }
    }
}

fn close_outcome(code: u16) -> Outcome {
    match code {
        4004 => Outcome::Stop(ERR_UNAUTHORIZED.to_string()),
        4012 => Outcome::Stop(ERR_PROTOCOL.to_string()),
        4001 | 4002 | 4003 | 4005 | 4007 | 4009 => {
            Outcome::Retry { resume: false, error: Some(format!("gateway closed {}", code)) }
        }
        4000 => Outcome::Retry { resume: false, error: None },
        _ => Outcome::Retry { resume: true, error: None },
    }
}

pub async fn start(
    gateways: &Gateways,
    base: String,
    token: String,
    sink: Arc<dyn GatewaySink>,
) -> StatusSnapshot {
    let mut map = gateways.lock().await;
    if let Some(existing) = map.get(&base) {
        if !existing.shared.is_finished() {
            let snap = existing.shared.snapshot();
            existing.shared.sink.status(&base, snap.status, snap.error.as_deref());
            return snap;
        }
        map.remove(&base);
    }
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let cancel = CancellationToken::new();
    let shared = Arc::new(Shared {
        url: base.clone(),
        sink,
        status: std::sync::Mutex::new((Status::Connecting, None)),
        finished: std::sync::atomic::AtomicBool::new(false),
    });
    let handle = GatewayHandle { cancel: cancel.clone(), outbound: tx, shared: shared.clone() };
    map.insert(base.clone(), handle);
    let task_shared = shared.clone();
    let task_base = base.clone();
    tauri::async_runtime::spawn(async move {
        let fut = run_with_shared(task_base.clone(), token, rx, cancel, task_shared.clone());
        if std::panic::AssertUnwindSafe(fut).catch_unwind().await.is_err() {
            tracing::error!("[omnidisc] gateway task for {} panicked", task_base);
            task_shared.set_status(Status::Disconnected, Some(ERR_PROTOCOL));
            task_shared.finished.store(true, std::sync::atomic::Ordering::Release);
        }
    });
    shared.snapshot()
}

pub async fn stop(gateways: &Gateways, base: &str) {
    let removed = gateways.lock().await.remove(base);
    if let Some(handle) = removed {
        handle.cancel.cancel();
    }
}

pub async fn send(gateways: &Gateways, base: &str, op: u8, d: Value) -> Result<(), String> {
    let map = gateways.lock().await;
    let handle = map.get(base).ok_or_else(|| ERR_NOT_CONNECTED.to_string())?;
    if handle.shared.is_finished() || handle.shared.snapshot().status != Status::Ready {
        return Err(ERR_NOT_CONNECTED.to_string());
    }
    handle
        .outbound
        .send(json!({ "op": op, "d": d }).to_string())
        .map_err(|_| ERR_NOT_CONNECTED.to_string())
}

pub async fn is_ready(gateways: &Gateways, base: &str) -> bool {
    let map = gateways.lock().await;
    map.get(base)
        .map(|h| !h.shared.is_finished() && h.shared.snapshot().status == Status::Ready)
        .unwrap_or(false)
}

#[tauri::command]
pub async fn omnidisc_gateway_connect(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    url: String,
) -> Result<StatusSnapshot, String> {
    let base = normalize_instance_url(&url)?;
    let token = store::load_token(&base)?.ok_or_else(|| ERR_NO_SESSION.to_string())?;
    Ok(start(&state.omnidisc_gateways, base, token, Arc::new(TauriSink(app))).await)
}

#[tauri::command]
pub async fn omnidisc_gateway_disconnect(
    state: tauri::State<'_, crate::AppState>,
    url: String,
) -> Result<(), String> {
    let base = normalize_instance_url(&url)?;
    stop(&state.omnidisc_gateways, &base).await;
    Ok(())
}

#[tauri::command]
pub async fn omnidisc_gateway_send(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    op: u8,
    d: Value,
) -> Result<(), String> {
    let base = normalize_instance_url(&url)?;
    send(&state.omnidisc_gateways, &base, op, d).await
}

#[tauri::command]
pub async fn omnidisc_gateway_status(
    state: tauri::State<'_, crate::AppState>,
    url: String,
) -> Result<StatusSnapshot, String> {
    let base = normalize_instance_url(&url)?;
    let map = state.omnidisc_gateways.lock().await;
    Ok(map
        .get(&base)
        .filter(|h| !h.shared.is_finished())
        .map(|h| h.shared.snapshot())
        .unwrap_or(StatusSnapshot { url: base, status: Status::Disconnected, error: None }))
}

#[tauri::command]
pub async fn omnidisc_typing(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    channel_id: String,
) -> Result<(), String> {
    let base = normalize_instance_url(&url)?;
    let via_gateway = send(
        &state.omnidisc_gateways,
        &base,
        Opcode::Typing as u8,
        json!({ "channel_id": channel_id }),
    )
    .await;
    match via_gateway {
        Ok(()) => Ok(()),
        Err(_) => super::api::typing_rest(&base, &channel_id).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_swaps_scheme_and_appends_gateway() {
        assert_eq!(ws_url("https://chat.example.org"), "wss://chat.example.org/gateway");
        assert_eq!(ws_url("http://localhost:8080"), "ws://localhost:8080/gateway");
    }

    #[test]
    fn backoff_grows_and_caps() {
        assert!(backoff(0) >= Duration::from_secs(1) && backoff(0) < Duration::from_secs(2));
        assert!(backoff(3) >= Duration::from_secs(8) && backoff(3) < Duration::from_secs(9));
        assert!(backoff(12) >= Duration::from_secs(30) && backoff(12) < Duration::from_secs(31));
    }

    #[test]
    fn close_codes_decide_resume() {
        assert!(matches!(close_outcome(4004), Outcome::Stop(e) if e == ERR_UNAUTHORIZED));
        assert!(matches!(close_outcome(4009), Outcome::Retry { resume: false, .. }));
        assert!(matches!(close_outcome(1006), Outcome::Retry { resume: true, .. }));
    }

    #[test]
    fn identify_has_required_fields() {
        let p = identify_payload("od1.x");
        assert_eq!(p["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(p["compress"], "none");
        assert_eq!(p["properties"]["client"], "omniget");
    }
}
