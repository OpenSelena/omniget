use super::voice::EVENT_VOICE;
use omnidisc_media::{
    start_stream, ActiveStream, AudioMode, LiveKitBackend, PublishStats, SourceId, StreamError,
    StreamMode, StreamRequest, StreamSources, StreamStats, Viewer, Viewport, WatchStats,
};
use omnidisc_proto::bitrate::StreamingPolicy;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;

pub struct StreamManager {
    active: Mutex<Option<ActiveStream>>,
    viewers: Mutex<HashMap<String, Viewer>>,
}

impl Default for StreamManager {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
            viewers: Mutex::new(HashMap::new()),
        }
    }
}

fn err(e: StreamError) -> String {
    e.code().to_string()
}

fn backend(state: &crate::AppState) -> Result<Arc<LiveKitBackend>, String> {
    state
        .omnidisc_voice
        .livekit_backend()
        .ok_or_else(|| "ERR_VOICE_UNAVAILABLE".to_string())
}

fn emit_voice(
    app: &tauri::AppHandle,
    url: Option<String>,
    event: &str,
    mut payload: serde_json::Value,
) {
    if let serde_json::Value::Object(map) = &mut payload {
        map.insert("type".into(), json!(event));
        map.insert(
            "url".into(),
            url.map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    let _ = app.emit(EVENT_VOICE, payload);
}

/// What this build can actually do with media, so the interface never has to
/// guess from the platform name.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MediaCapabilities {
    /// A voice backend was created at startup. False means calls are dead
    /// weight in the UI and the entry points should say so.
    pub voice: bool,
    /// Screen capture exists for this platform. Whether the user has granted
    /// the permission is a separate answer, and it belongs to the share dialog.
    pub screen_share: bool,
    /// Someone else's stream can be rendered into a window.
    pub stream_viewer: bool,
}

#[tauri::command]
pub async fn omnidisc_media_capabilities(
    state: tauri::State<'_, crate::AppState>,
) -> Result<MediaCapabilities, String> {
    // Deliberately not probed by calling into the capture layer: on macOS the
    // first enumeration is what raises the screen-recording prompt, and asking
    // for that permission before the user has asked to share anything is the
    // kind of thing that gets an app denied for good.
    Ok(MediaCapabilities {
        voice: state.omnidisc_voice.livekit_backend().is_some(),
        screen_share: cfg!(any(target_os = "macos", target_os = "windows")),
        stream_viewer: cfg!(any(target_os = "macos", target_os = "windows")),
    })
}

#[tauri::command]
pub async fn omnidisc_stream_sources(
    state: tauri::State<'_, crate::AppState>,
) -> Result<StreamSources, String> {
    let _ = state;
    tokio::task::spawn_blocking(|| omnidisc_media::capture::list_sources(true).map_err(err))
        .await
        .map_err(|e| format!("OmniDisc: source enumeration failed: {e}"))?
}

#[derive(Deserialize)]
pub struct StartArgs {
    pub source: SourceId,
    pub fps: u16,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub audio: AudioMode,
    #[serde(default)]
    pub bitrate_kbps: Option<u32>,
    #[serde(default)]
    pub mode: StreamMode,
    #[serde(default)]
    pub cursor: bool,
    #[serde(default)]
    pub policy: Option<StreamingPolicy>,
}

#[tauri::command]
pub async fn omnidisc_stream_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    args: StartArgs,
) -> Result<PublishStats, String> {
    let backend = backend(&state)?;
    if !backend.is_connected().await {
        return Err(StreamError::NotConnected.code().to_string());
    }
    let req = StreamRequest {
        source: args.source,
        fps: args.fps,
        height: args.height,
        audio: args.audio,
        bitrate_kbps: args.bitrate_kbps,
        mode: args.mode,
        cursor: args.cursor,
        policy: args.policy.unwrap_or_default(),
    };
    let stream = start_stream(backend, req).await.map_err(err)?;
    let audio_mode = stream.audio_mode();
    let resolved = stream.resolved.clone();
    {
        let mut active = state.omnidisc_stream.active.lock().await;
        if let Some(old) = active.take() {
            if let Some(b) = state.omnidisc_voice.livekit_backend() {
                if let Some(room) = b.current_room().await {
                    old.stop(&room).await;
                }
            }
        }
        // schedule the two-stage overdrive re-apply like Backspace (t+2 s, t+5 s)
        *active = Some(stream);
    }
    state.omnidisc_voice.set_streaming(true);
    state
        .omnidisc_voice
        .announce_stream(&state.omnidisc_gateways)
        .await;
    let url = state.omnidisc_voice.session_info().await.map(|s| s.url);
    schedule_overdrive(app.clone());
    let stats = current_publish_stats(&state).await.unwrap_or_default();
    emit_voice(
        &app,
        url.clone(),
        "stream_started",
        serde_json::to_value(&resolved).unwrap_or_default(),
    );
    emit_voice(
        &app,
        url,
        "stream_audio_mode",
        json!({ "audio": audio_mode }),
    );
    Ok(stats)
}

fn schedule_overdrive(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        for delay in [2u64, 5] {
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            if let Some(state) = app.try_state::<crate::AppState>() {
                if let Some(s) = state.omnidisc_stream.active.lock().await.as_ref() {
                    s.overdrive();
                }
            }
        }
    });
}

async fn current_publish_stats(state: &crate::AppState) -> Option<PublishStats> {
    let active = state.omnidisc_stream.active.lock().await;
    let stream = active.as_ref()?;
    Some(stream.stats().await)
}

#[tauri::command]
pub async fn omnidisc_stream_stop(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let stream = state.omnidisc_stream.active.lock().await.take();
    if let Some(stream) = stream {
        if let Some(b) = state.omnidisc_voice.livekit_backend() {
            if let Some(room) = b.current_room().await {
                stream.stop(&room).await;
            }
        }
    }
    state.omnidisc_voice.set_streaming(false);
    state
        .omnidisc_voice
        .announce_stream(&state.omnidisc_gateways)
        .await;
    let url = state.omnidisc_voice.session_info().await.map(|s| s.url);
    emit_voice(&app, url, "stream_stopped", json!({}));
    Ok(())
}

#[tauri::command]
pub async fn omnidisc_stream_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<StreamStats, String> {
    let publishing = current_publish_stats(&state).await;
    let mut watching: Vec<WatchStats> = Vec::new();
    let viewers = state.omnidisc_stream.viewers.lock().await;
    for v in viewers.values() {
        watching.push(v.stats().await);
    }
    Ok(StreamStats {
        publishing,
        watching,
    })
}

#[tauri::command]
pub async fn omnidisc_stream_set_volume(
    state: tauri::State<'_, crate::AppState>,
    user_id: String,
    gain: f32,
) -> Result<(), String> {
    if !gain.is_finite() {
        return Err("ERR_BAD_REQUEST".into());
    }
    if let Some(b) = state.omnidisc_voice.livekit_backend() {
        b.set_screen_audio_gain(&user_id, gain.clamp(0.0, 2.0));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct ViewportArgs {
    pub user_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    pub surface_width: u32,
    pub surface_height: u32,
    #[serde(default)]
    pub background: Option<[f32; 3]>,
}

#[tauri::command]
pub async fn omnidisc_stream_set_viewport(
    state: tauri::State<'_, crate::AppState>,
    args: ViewportArgs,
) -> Result<(), String> {
    let viewers = state.omnidisc_stream.viewers.lock().await;
    if let Some(v) = viewers.get(&args.user_id) {
        v.set_viewport(Some(Viewport {
            x: args.x,
            y: args.y,
            width: args.width,
            height: args.height,
            scale: if args.scale > 0.0 { args.scale } else { 1.0 },
            surface_width: args.surface_width,
            surface_height: args.surface_height,
            background: args.background.unwrap_or([0.0, 0.0, 0.0]),
        }));
    }
    Ok(())
}

#[tauri::command]
pub async fn omnidisc_stream_watch(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    user_id: String,
) -> Result<(), String> {
    let backend = backend(&state)?;
    if state
        .omnidisc_stream
        .viewers
        .lock()
        .await
        .contains_key(&user_id)
    {
        return Ok(());
    }
    let publication = backend
        .video_publication_for(&user_id)
        .ok_or_else(|| StreamError::NoSuchStream.code().to_string())?;
    publication.set_subscribed(true);

    let track = {
        let mut found = None;
        for _ in 0..50 {
            if let Some(rv) = backend.remote_video_for(&user_id) {
                found = Some(rv.track);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        found.ok_or_else(|| {
            StreamError::Viewer("stream did not arrive".into())
                .code()
                .to_string()
        })?
    };

    let label = format!("omnidisc-stream-{}", sanitize(&user_id));
    let win_label = label.clone();
    let uid = user_id.clone();
    let build = WebviewWindowBuilder::new(
        &app,
        &win_label,
        WebviewUrl::App(format!("/omnidisc/stream?user={user_id}").into()),
    )
    .title("OmniDisc — Stream")
    .inner_size(1280.0, 760.0)
    .min_inner_size(480.0, 320.0)
    .transparent(true);
    let window = build
        .build()
        .map_err(|e| format!("OmniDisc: could not open the stream window: {e}"))?;

    let renderer = match create_surface_on_main(&app, &window).await {
        Ok(r) => r,
        Err(e) => {
            // A window that cannot draw anything is worse than no window: it
            // looks like the stream is loading forever.
            let _ = window.close();
            publication.set_subscribed(false);
            return Err(e);
        }
    };
    let rt = backend
        .runtime_handle()
        .ok_or_else(|| "ERR_VOICE_UNAVAILABLE".to_string())?;
    let viewer = Viewer::new(renderer, track, uid, rt);
    state
        .omnidisc_stream
        .viewers
        .lock()
        .await
        .insert(user_id, viewer);
    Ok(())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

async fn create_surface_on_main(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<Arc<omnidisc_media::viewer::WgpuViewer>, String> {
    let size = window
        .inner_size()
        .map_err(|e| format!("OmniDisc: window size: {e}"))?;
    #[cfg(target_os = "macos")]
    {
        let ns_view = window
            .ns_view()
            .map_err(|e| format!("OmniDisc: ns_view: {e}"))?;
        let ptr = ns_view as usize;
        let (w, h) = (size.width, size.height);
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let result = unsafe {
                omnidisc_media::viewer::create_appkit_surface(ptr as *mut std::ffi::c_void, w, h)
            };
            let _ = tx.send(result.map_err(err));
        })
        .map_err(|e| format!("OmniDisc: main thread dispatch: {e}"))?;
        rx.await
            .map_err(|_| "OmniDisc: surface creation dropped".to_string())?
    }
    #[cfg(target_os = "windows")]
    {
        let hwnd = window.hwnd().map_err(|e| format!("OmniDisc: hwnd: {e}"))?.0 as isize;
        let hinstance = window_hinstance(hwnd);
        let (w, h) = (size.width, size.height);
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let result =
                unsafe { omnidisc_media::viewer::create_win32_surface(hwnd, hinstance, w, h) };
            let _ = tx.send(result.map_err(err));
        })
        .map_err(|e| format!("OmniDisc: main thread dispatch: {e}"))?;
        rx.await
            .map_err(|_| "OmniDisc: surface creation dropped".to_string())?
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (app, window, size);
        Err(StreamError::Unsupported.code().to_string())
    }
}

/// The module handle Vulkan asks for. DX12 ignores it, so a zero here only
/// costs the Vulkan backend, never the whole viewer.
#[cfg(target_os = "windows")]
fn window_hinstance(hwnd: isize) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_HINSTANCE};
        unsafe { GetWindowLongPtrW(HWND(hwnd as *mut std::ffi::c_void), GWLP_HINSTANCE) }
    }
    #[cfg(not(target_pointer_width = "64"))]
    {
        let _ = hwnd;
        0
    }
}

#[tauri::command]
pub async fn omnidisc_stream_unwatch(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    user_id: String,
) -> Result<(), String> {
    let removed = state.omnidisc_stream.viewers.lock().await.remove(&user_id);
    if removed.is_some() {
        if let Some(b) = state.omnidisc_voice.livekit_backend() {
            if let Some(p) = b.video_publication_for(&user_id) {
                p.set_subscribed(false);
            }
        }
        let label = format!("omnidisc-stream-{}", sanitize(&user_id));
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.close();
        }
    }
    Ok(())
}
