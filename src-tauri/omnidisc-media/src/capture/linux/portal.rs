//! The xdg-desktop-portal ScreenCast handshake.
//!
//! The portal is the only screen-capture interface a Wayland compositor will
//! answer, and it insists on showing its own picker — that is the whole point of
//! it. We keep the returned restore token so the second share skips the dialog;
//! the token is single-use and rotates, so it is written back every time.

use crate::stream::StreamError;
use ashpd::desktop::screencast::{
    CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, SourceType,
    StartCastOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use std::io::Write;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::time::Duration;

/// Long enough that a person can read the dialog and choose, short enough that a
/// portal which never answers does not wedge the share forever.
const PICKER_TIMEOUT: Duration = Duration::from_secs(120);

pub struct ScreencastSession {
    pub node_id: u32,
    /// What the portal thinks the source measures. Advisory only: the real
    /// geometry comes from PipeWire's format negotiation, which is the one the
    /// frames actually arrive in.
    #[allow(dead_code)]
    pub size: Option<(i32, i32)>,
    pub fd: OwnedFd,
}

fn token_path() -> Option<PathBuf> {
    let base = std::env::var_os("OMNIGET_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .map(|p| p.join("wtf.tonho.omniget"))
        })
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".local/share")
                    .join("wtf.tonho.omniget")
            })
        })?;
    Some(base.join("omnidisc-screencast.json"))
}

fn load_token() -> Option<String> {
    let raw = std::fs::read_to_string(token_path()?).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("restore_token")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn save_token(token: Option<&str>) {
    let Some(path) = token_path() else { return };
    let Some(token) = token else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = serde_json::json!({ "restore_token": token }).to_string();
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = f.write_all(body.as_bytes());
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, StreamError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| StreamError::Capture(format!("portal runtime: {e}")))
}

fn map_err(e: ashpd::Error) -> StreamError {
    let text = e.to_string();
    let lower = text.to_lowercase();
    if lower.contains("cancel") {
        StreamError::Permission
    } else {
        StreamError::Capture(format!("screen sharing portal: {text}"))
    }
}

/// Does this desktop implement ScreenCast at all? `xdg-desktop-portal-gtk` does
/// not, so XFCE/MATE/i3 setups with only that backend must be told which package
/// is missing instead of being shown a picker that leads nowhere.
pub fn probe() -> Result<(), StreamError> {
    let rt = runtime()?;
    rt.block_on(async {
        let proxy = Screencast::new().await.map_err(map_err)?;
        let sources = proxy.available_source_types().await.map_err(map_err)?;
        if sources.is_empty() {
            return Err(StreamError::Capture(
                "this desktop has no screen-sharing portal; install xdg-desktop-portal-gnome, -kde or -wlr".into(),
            ));
        }
        Ok(())
    })
}

pub fn open_screencast(cursor: bool) -> Result<ScreencastSession, StreamError> {
    let rt = runtime()?;
    rt.block_on(async move {
        let proxy = Screencast::new().await.map_err(map_err)?;
        let available_cursor = proxy.available_cursor_modes().await.map_err(map_err)?;
        let cursor_mode = if cursor && available_cursor.contains(CursorMode::Embedded) {
            CursorMode::Embedded
        } else {
            CursorMode::Hidden
        };
        let available_sources = proxy.available_source_types().await.map_err(map_err)?;
        let wanted = if available_sources.contains(SourceType::Window) {
            SourceType::Monitor | SourceType::Window
        } else {
            SourceType::Monitor.into()
        };

        let session = proxy
            .create_session(CreateSessionOptions::default())
            .await
            .map_err(map_err)?;
        let mut options = SelectSourcesOptions::default()
            .set_cursor_mode(cursor_mode)
            .set_sources(wanted)
            .set_multiple(false)
            .set_persist_mode(PersistMode::ExplicitlyRevoked);
        let saved = load_token();
        if let Some(token) = saved.as_deref() {
            options = options.set_restore_token(token);
        }
        proxy
            .select_sources(&session, options)
            .await
            .map_err(map_err)?;

        // `start` is what shows the dialog and only resolves once a human has
        // answered it, so the timeout belongs here rather than on the response.
        let request = tokio::time::timeout(
            PICKER_TIMEOUT,
            proxy.start(&session, None, StartCastOptions::default()),
        )
        .await
        .map_err(|_| StreamError::Capture("nobody answered the screen-sharing dialog".into()))?
        .map_err(map_err)?;
        let streams = request.response().map_err(map_err)?;
        save_token(streams.restore_token());

        let stream = streams.streams().first().ok_or(StreamError::Permission)?;
        let (node_id, size) = (stream.pipe_wire_node_id(), stream.size());
        let fd = proxy
            .open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default())
            .await
            .map_err(map_err)?;
        Ok(ScreencastSession { node_id, size, fd })
    })
}
