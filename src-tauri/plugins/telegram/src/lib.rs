pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};
use state::TelegramPluginState;
use std::sync::Arc;

pub struct TelegramPlugin {
    host: Option<Arc<dyn PluginHost>>,
    state: Arc<TelegramPluginState>,
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            state: Arc::new(TelegramPluginState::default()),
        }
    }
}

pub fn manifest() -> omniget_plugin_sdk::PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid telegram manifest")
}

impl OmnigetPlugin for TelegramPlugin {
    fn id(&self) -> &str {
        "telegram"
    }

    fn name(&self) -> &str {
        "Telegram Downloader"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    > {
        let host = self.host.clone();
        let state = self.state.clone();
        Box::pin(async move {
            match command.as_str() {
                "telegram_check_session" => {
                    let res = commands::telegram::telegram_check_session(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_qr_start" => {
                    let res = commands::telegram::telegram_qr_start(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_qr_poll" => {
                    let res = commands::telegram::telegram_qr_poll(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_send_code" => {
                    let phone: String = serde_json::from_value(
                        args.get("phone").cloned().ok_or("missing 'phone'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    commands::telegram::telegram_send_code(&state, phone).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_verify_code" => {
                    let code: String =
                        serde_json::from_value(args.get("code").cloned().ok_or("missing 'code'")?)
                            .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_verify_code(&state, code).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_verify_2fa" => {
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or("missing 'password'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_verify_2fa(&state, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_logout" => {
                    commands::telegram::telegram_logout(&state).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_list_chats" => {
                    let res = commands::telegram::telegram_list_chats(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_list_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args
                        .get("media_type")
                        .or(args.get("mediaType"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let offset: i32 = args
                        .get("offset")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let res = commands::telegram::telegram_list_media(
                        &state, chat_id, chat_type, media_type, offset, limit,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_download_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let message_id: i32 = serde_json::from_value(
                        args.get("message_id")
                            .or(args.get("messageId"))
                            .cloned()
                            .ok_or("missing 'message_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let file_name: String = serde_json::from_value(
                        args.get("file_name")
                            .or(args.get("fileName"))
                            .cloned()
                            .ok_or("missing 'file_name'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let output_dir: String = args
                        .get("output_dir")
                        .or(args.get("outputDir"))
                        .or(args.get("save_path"))
                        .or(args.get("savePath"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| {
                            host.as_ref()
                                .map(|h| h.default_output_dir().to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    let chat_title: Option<String> = args
                        .get("chat_title")
                        .or(args.get("chatTitle"))
                        .or(args.get("channel_name"))
                        .or(args.get("channelName"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let res = commands::telegram::telegram_download_media(
                        host.clone(),
                        &state,
                        chat_id,
                        chat_type,
                        chat_title,
                        message_id,
                        file_name,
                        output_dir,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_download_batch" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_title: String = serde_json::from_value(
                        args.get("chat_title")
                            .or(args.get("chatTitle"))
                            .cloned()
                            .unwrap_or(serde_json::json!("Telegram Download")),
                    )
                    .unwrap_or_else(|_| "Telegram Download".to_string());
                    let items: Vec<commands::telegram::BatchItem> = serde_json::from_value(
                        args.get("items").cloned().ok_or("missing 'items'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let output_dir: String = args
                        .get("output_dir")
                        .or(args.get("outputDir"))
                        .or(args.get("save_dir"))
                        .or(args.get("saveDir"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| {
                            host.as_ref()
                                .map(|h| h.default_output_dir().to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    let res = commands::telegram::telegram_download_batch(
                        host.clone(),
                        &state,
                        chat_id,
                        chat_type,
                        chat_title,
                        items,
                        output_dir,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_cancel_batch" => {
                    let batch_id: u64 = serde_json::from_value(
                        args.get("batch_id")
                            .or(args.get("batchId"))
                            .cloned()
                            .ok_or("missing 'batch_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    commands::telegram::telegram_cancel_batch(&state, batch_id).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_get_thumbnail" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .unwrap_or(serde_json::json!("channel")),
                    )
                    .unwrap_or_else(|_| "channel".to_string());
                    let message_id: i32 = serde_json::from_value(
                        args.get("message_id")
                            .or(args.get("messageId"))
                            .cloned()
                            .ok_or("missing 'message_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_get_thumbnail(
                        &state, chat_id, chat_type, message_id,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_search_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let query: String = serde_json::from_value(
                        args.get("query").cloned().ok_or("missing 'query'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args
                        .get("media_type")
                        .or(args.get("mediaType"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let res = commands::telegram::telegram_search_media(
                        &state, chat_id, chat_type, query, media_type, limit,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_get_chat_photo" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .unwrap_or(serde_json::json!("channel")),
                    )
                    .unwrap_or_else(|_| "channel".to_string());
                    let res =
                        commands::telegram::telegram_get_chat_photo(&state, chat_id, chat_type)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_clear_thumbnail_cache" => {
                    commands::telegram::telegram_clear_thumbnail_cache().await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_perf_get" => Ok(serde_json::json!({
                    "max_threads": 8,
                    "max_part_size_kb": 512,
                    "buckets": []
                })),
                "telegram_perf_set" => {
                    let max_threads: u32 = args
                        .get("maxThreads")
                        .or(args.get("max_threads"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(8);
                    Ok(serde_json::json!({ "max_threads": max_threads }))
                }
                "telegram_bandwidth_stats" => Ok(serde_json::json!({
                    "used_today_bytes": 0,
                    "total_used_bytes": 0,
                    "quota_bytes": 0,
                    "date": "",
                    "percentage": 0.0
                })),
                "telegram_bandwidth_set_quota" => {
                    let gb: u64 = args
                        .get("gb")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    Ok(serde_json::json!({
                        "quota_bytes": gb * 1024 * 1024 * 1024,
                        "used_today_bytes": 0
                    }))
                }
                "telegram_bandwidth_reset" => Ok(serde_json::json!({
                    "used_today_bytes": 0,
                    "date": ""
                })),
                "telegram_sync_state" => Ok(serde_json::json!({
                    "enabled": false,
                    "interval_min": 30,
                    "last_success_at": 0,
                    "last_duration_ms": 0,
                    "last_updated_count": 0,
                    "is_syncing": false
                })),
                "telegram_sync_now" => {
                    let res = commands::telegram::telegram_list_chats(&state).await;
                    match res {
                        Ok(chats) => {
                            let count = chats.len();
                            Ok(serde_json::json!({ "updated": count }))
                        }
                        Err(_) => Ok(serde_json::json!({ "updated": 0 })),
                    }
                }
                "telegram_sync_settings_set" => {
                    let enabled: bool = args
                        .get("enabled")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(false);
                    let interval_min: u32 = args
                        .get("intervalMin")
                        .or(args.get("interval_min"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(30);
                    Ok(serde_json::json!({
                        "enabled": enabled,
                        "interval_min": interval_min,
                        "last_success_at": 0,
                        "last_duration_ms": 0,
                        "last_updated_count": 0,
                        "is_syncing": false
                    }))
                }
                "telegram_full_channel_info" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({
                        "chat_id": chat_id,
                        "title": "",
                        "about": "",
                        "username": null,
                        "participants_count": null
                    }))
                }
                "telegram_list_participants" => Ok(serde_json::json!({
                    "users": [],
                    "count": 0
                })),
                "telegram_accounts_list" => Ok(serde_json::json!([])),
                "telegram_accounts_save_current" => {
                    let label: String = args
                        .get("label")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "Account".to_string());
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    Ok(serde_json::json!({
                        "id": format!("acc_{}", now),
                        "label": label,
                        "phone_redacted": null,
                        "user_id": null,
                        "created_at": now,
                        "updated_at": now
                    }))
                }
                "telegram_accounts_restore" => {
                    Ok(serde_json::json!({ "ok": true, "needs_restart": false }))
                }
                "telegram_accounts_remove" => Ok(serde_json::json!({ "ok": true })),
                "telegram_accounts_rename" => {
                    let id: String = args
                        .get("id")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let label: String = args
                        .get("label")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    Ok(serde_json::json!({
                        "id": id,
                        "label": label,
                        "phone_redacted": null,
                        "user_id": null,
                        "created_at": now,
                        "updated_at": now
                    }))
                }
                "telegram_accounts_backup_now" => Ok(serde_json::json!({ "path": "", "name": "" })),
                "telegram_accounts_list_backups" => Ok(serde_json::json!([])),
                "telegram_clone_list" => Ok(serde_json::json!([])),
                "telegram_clone_start" => {
                    let session_id = format!(
                        "clone_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );
                    Ok(serde_json::json!({
                        "session_id": session_id,
                        "dest_chat_id": 0,
                        "dest_chat_type": "channel",
                        "dest_title": "Clone"
                    }))
                }
                "telegram_clone_status" => {
                    let session_id: String = args
                        .get("sessionId")
                        .or(args.get("session_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    Ok(serde_json::json!({
                        "id": session_id,
                        "source_chat_id": 0,
                        "source_chat_type": "channel",
                        "source_title": "",
                        "dest_chat_id": 0,
                        "dest_chat_type": "channel",
                        "dest_title": "",
                        "last_message_id": 0,
                        "total_collected": 0,
                        "cloned_count": 0,
                        "failed_count": 0,
                        "status": "completed",
                        "options": {
                            "delay_ms": 2000,
                            "batch_size": 50,
                            "limit": null,
                            "drop_author": false,
                            "drop_captions": false
                        },
                        "created_at": 0,
                        "updated_at": 0
                    }))
                }
                "telegram_clone_pause" => Ok(serde_json::json!({ "cancelled": true })),
                "telegram_clone_cancel" => Ok(serde_json::json!({ "ok": true })),
                "telegram_clone_delete" => Ok(serde_json::json!({ "ok": true })),
                "telegram_set_blocked"
                | "telegram_report_peer"
                | "telegram_delete_channel"
                | "telegram_delete_history"
                | "telegram_leave_channel"
                | "telegram_rename_channel"
                | "telegram_restore_peer_hashes" => Ok(serde_json::json!({ "ok": true })),
                "telegram_create_folder" => {
                    let name: String = args
                        .get("name")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    Ok(serde_json::json!({
                        "id": 0,
                        "title": name,
                        "chat_type": "folder"
                    }))
                }
                "telegram_diag_list_media" | "telegram_expand_album" => Ok(serde_json::json!([])),
                "telegram_takeout_start" => Ok(serde_json::json!({ "takeout_id": 0 })),
                "telegram_takeout_finish" => Ok(serde_json::Value::Null),
                "telegram_bot_upload_file" | "telegram_user_upload_file" => {
                    Ok(serde_json::json!({ "success": false }))
                }
                "telegram_cancel_op" => Ok(serde_json::json!({ "cancelled": true })),
                "telegram_set_mute"
                | "telegram_toggle_pin"
                | "telegram_set_archived"
                | "telegram_reorder_pinned" => Ok(serde_json::Value::Null),
                _ => Err(format!("Unknown command: {}", command)),
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "telegram_check_session".into(),
            "telegram_qr_start".into(),
            "telegram_qr_poll".into(),
            "telegram_send_code".into(),
            "telegram_verify_code".into(),
            "telegram_verify_2fa".into(),
            "telegram_logout".into(),
            "telegram_list_chats".into(),
            "telegram_list_media".into(),
            "telegram_download_media".into(),
            "telegram_download_batch".into(),
            "telegram_cancel_batch".into(),
            "telegram_get_thumbnail".into(),
            "telegram_search_media".into(),
            "telegram_get_chat_photo".into(),
            "telegram_clear_thumbnail_cache".into(),
            "telegram_perf_get".into(),
            "telegram_perf_set".into(),
            "telegram_bandwidth_stats".into(),
            "telegram_bandwidth_set_quota".into(),
            "telegram_bandwidth_reset".into(),
            "telegram_sync_state".into(),
            "telegram_sync_now".into(),
            "telegram_sync_settings_set".into(),
            "telegram_full_channel_info".into(),
            "telegram_list_participants".into(),
            "telegram_accounts_list".into(),
            "telegram_accounts_save_current".into(),
            "telegram_accounts_restore".into(),
            "telegram_accounts_remove".into(),
            "telegram_accounts_rename".into(),
            "telegram_accounts_backup_now".into(),
            "telegram_accounts_list_backups".into(),
            "telegram_clone_list".into(),
            "telegram_clone_start".into(),
            "telegram_clone_status".into(),
            "telegram_clone_pause".into(),
            "telegram_clone_cancel".into(),
            "telegram_clone_delete".into(),
            "telegram_set_blocked".into(),
            "telegram_report_peer".into(),
            "telegram_delete_channel".into(),
            "telegram_delete_history".into(),
            "telegram_leave_channel".into(),
            "telegram_rename_channel".into(),
            "telegram_restore_peer_hashes".into(),
            "telegram_create_folder".into(),
            "telegram_diag_list_media".into(),
            "telegram_expand_album".into(),
            "telegram_takeout_start".into(),
            "telegram_takeout_finish".into(),
            "telegram_bot_upload_file".into(),
            "telegram_user_upload_file".into(),
            "telegram_cancel_op".into(),
            "telegram_set_mute".into(),
            "telegram_toggle_pin".into(),
            "telegram_set_archived".into(),
            "telegram_reorder_pinned".into(),
        ]
    }
}
