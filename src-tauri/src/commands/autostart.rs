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

pub fn get_autostart_enabled(app: &tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Failed to query autostart: {e}"))
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    apply_autostart(&app, enabled)?;
    let mut current = crate::storage::config::load_settings(&app);
    current.start_with_system = enabled;
    crate::storage::config::save_settings(&app, &current).map_err(|e| format!("Save: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_autostart_status(app: tauri::AppHandle) -> Result<bool, String> {
    get_autostart_enabled(&app)
}
