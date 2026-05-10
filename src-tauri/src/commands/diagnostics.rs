#[tauri::command]
pub fn get_rate_limit_stats() -> serde_json::Value {
    crate::core::ytdlp::get_rate_limit_stats()
}

#[tauri::command]
pub async fn get_hwaccel_info() -> omniget_core::core::hwaccel::HwAccelInfo {
    omniget_core::core::hwaccel::detect_hwaccel().await
}
