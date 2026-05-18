use tauri_plugin_autostart::ManagerExt;

pub fn apply_autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if std::env::var("OMNIGET_PORTABLE").is_ok() {
        return Ok(());
    }
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {e}"))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {e}"))?;
    }
    Ok(())
}
