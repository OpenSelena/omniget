use tauri::AppHandle;

#[tauri::command]
pub fn get_active_download_count(app: AppHandle) -> u32 {
    crate::tray::compute_total_active(&app)
}

#[tauri::command]
pub fn request_app_quit(app: AppHandle) {
    crate::tray::request_quit(&app);
}

#[tauri::command]
pub fn force_exit_app(app: AppHandle) {
    app.exit(0);
}
