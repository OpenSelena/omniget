#[tauri::command]
pub async fn get_hwaccel_info() -> omniget_core::core::hwaccel::HwAccelInfo {
    omniget_core::core::hwaccel::detect_hwaccel().await
}
