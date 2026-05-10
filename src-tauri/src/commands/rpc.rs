use serde_json::Value;

use crate::core::rpc;
use crate::storage::config;

#[tauri::command]
pub async fn rpc_test_connection(app: tauri::AppHandle) -> Result<Value, String> {
    let settings = config::load_settings(&app);
    rpc::test_connection(settings.rpc.clone()).await
}

#[tauri::command]
pub async fn rpc_set_source(
    app: tauri::AppHandle,
    source: String,
    details: String,
    state: String,
    duration: i64,
    position: i64,
    paused: bool,
    large_image_key: Option<String>,
) -> Result<Value, String> {
    let settings = config::load_settings(&app);
    rpc::set_presence_source(
        settings.rpc.clone(),
        source,
        details,
        state,
        duration,
        position,
        paused,
        large_image_key,
    )
    .await
}

#[tauri::command]
pub async fn rpc_clear_source(
    app: tauri::AppHandle,
    source: String,
) -> Result<Value, String> {
    let settings = config::load_settings(&app);
    rpc::clear_source(settings.rpc.clone(), source).await
}

#[tauri::command]
pub async fn rpc_set_idle_stats(
    app: tauri::AppHandle,
    downloads_count: u64,
    total_bytes: u64,
) -> Result<Value, String> {
    let settings = config::load_settings(&app);
    rpc::set_idle_stats(settings.rpc.clone(), downloads_count, total_bytes).await
}
