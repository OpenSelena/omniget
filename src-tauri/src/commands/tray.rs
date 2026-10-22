//! Localization bridge for the native tray menu.
//!
//! The tray is built in Rust (`crate::tray`), where the Svelte `$t` helper is
//! not reachable, so its labels used to be hardcoded in English. The webview
//! now resolves the strings for the active locale and pushes them here on boot
//! and whenever the user switches language.

use crate::tray::{self, TrayLabels};

#[tauri::command]
pub fn sync_tray_labels(app: tauri::AppHandle, labels: TrayLabels) -> Result<(), String> {
    tray::apply_labels(&app, labels);
    Ok(())
}
