pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

use libreget_plugin_sdk::{LibregetPlugin, PluginHost};
use state::TelegramPluginState;
use std::sync::Arc;

pub struct TelegramPlugin {
    host: Option<Arc<dyn PluginHost>>,
    state: Arc<TelegramPluginState>,
}

impl Default for TelegramPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            state: Arc::new(TelegramPluginState::default()),
        }
    }
}

pub fn manifest() -> libreget_plugin_sdk::PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid telegram manifest")
}

impl LibregetPlugin for TelegramPlugin {
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
                "telegram_get_self" => {
                    let res = platforms::telegram::api::get_self(&state.telegram_session)
                        .await
                        .map_err(|e| e.to_string())?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_list_chats_page" => {
                    let limit: usize = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(100);
                    let offset_id: i32 = args
                        .get("offsetId")
                        .or(args.get("offset_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let chats = commands::telegram::telegram_list_chats(&state).await?;
                    let start = (offset_id.max(0) as usize).min(chats.len());
                    let end = (start + limit.max(1)).min(chats.len());
                    let page = chats[start..end].to_vec();
                    let next_offset = if end < chats.len() {
                        Some(serde_json::json!({
                            "date": 0,
                            "id": end as i32,
                            "peer_id": page.last().map(|c| c.id).unwrap_or(0),
                            "peer_type": page.last().map(|c| c.chat_type.clone()).unwrap_or_else(|| "channel".to_string()),
                            "peer_hash": page.last().and_then(|c| c.peer_hash).unwrap_or(0),
                        }))
                    } else {
                        None
                    };
                    Ok(serde_json::json!({
                        "chats": page,
                        "next_offset": next_offset,
                    }))
                }
                "telegram_search_global_hits" => {
                    let query: String = args
                        .get("query")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let fix_extensions = settings_helper::load_settings()
                        .telegram
                        .fix_file_extensions;
                    let hits = platforms::telegram::api::search_global_hits(
                        &state.telegram_session,
                        &query,
                        limit,
                        fix_extensions,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(hits).map_err(|e| e.to_string())
                }
                "telegram_search_global" => {
                    let query: String = args
                        .get("query")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let fix_extensions = settings_helper::load_settings()
                        .telegram
                        .fix_file_extensions;
                    let hits = platforms::telegram::api::search_global_hits(
                        &state.telegram_session,
                        &query,
                        limit,
                        fix_extensions,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    let items: Vec<platforms::telegram::api::TelegramMediaItem> = hits
                        .into_iter()
                        .map(|h| platforms::telegram::api::TelegramMediaItem {
                            message_id: h.message_id,
                            file_name: h.file_name,
                            file_size: h.file_size,
                            media_type: h.media_type,
                            date: h.date,
                            webpage: h.webpage,
                            caption: h.caption,
                            grouped_id: h.grouped_id,
                        })
                        .collect();
                    serde_json::to_value(items).map_err(|e| e.to_string())
                }
                "telegram_delete_messages" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let message_ids: Vec<i32> = args
                        .get("messageIds")
                        .or(args.get("message_ids"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let revoke: bool = args
                        .get("revoke")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    let deleted = platforms::telegram::api::delete_messages(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        message_ids,
                        revoke,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "deleted": deleted }))
                }
                "telegram_forward_messages" => {
                    let from_chat_id: i64 = serde_json::from_value(
                        args.get("fromChatId")
                            .or(args.get("from_chat_id"))
                            .cloned()
                            .ok_or("missing 'fromChatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let from_chat_type: String = args
                        .get("fromChatType")
                        .or(args.get("from_chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let to_chat_id: i64 = serde_json::from_value(
                        args.get("toChatId")
                            .or(args.get("to_chat_id"))
                            .cloned()
                            .ok_or("missing 'toChatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let to_chat_type: String = args
                        .get("toChatType")
                        .or(args.get("to_chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let message_ids: Vec<i32> = args
                        .get("messageIds")
                        .or(args.get("message_ids"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let drop_author: bool = args
                        .get("dropAuthor")
                        .or(args.get("drop_author"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(false);
                    let drop_captions: bool = args
                        .get("dropCaptions")
                        .or(args.get("drop_captions"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(false);
                    platforms::telegram::api::forward_messages(
                        &state.telegram_session,
                        from_chat_id,
                        &from_chat_type,
                        to_chat_id,
                        &to_chat_type,
                        message_ids,
                        drop_author,
                        drop_captions,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_edit_caption" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let message_id: i32 = serde_json::from_value(
                        args.get("messageId")
                            .or(args.get("message_id"))
                            .cloned()
                            .ok_or("missing 'messageId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let caption: String = args
                        .get("caption")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    platforms::telegram::api::edit_caption(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        message_id,
                        &caption,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_stream_info" => {
                    let info = platforms::telegram::api::ensure_stream_server(
                        state.telegram_session.clone(),
                        state.stream_server.clone(),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(info).map_err(|e| e.to_string())
                }
                "telegram_upload_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let file_path: String = serde_json::from_value(
                        args.get("filePath")
                            .or(args.get("file_path"))
                            .cloned()
                            .ok_or("missing 'filePath'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let caption: Option<String> = args
                        .get("caption")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let res = commands::telegram::telegram_upload_media(
                        host, &state, chat_id, chat_type, file_path, caption,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_perf_get" => {
                    let p = state.persistent.lock().await;
                    Ok(serde_json::json!({
                        "max_threads": p.perf.max_threads,
                        "max_part_size_kb": 512,
                        "buckets": p.perf.buckets,
                    }))
                }
                "telegram_perf_set" => {
                    let max_threads: u32 = args
                        .get("maxThreads")
                        .or(args.get("max_threads"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(8)
                        .clamp(1, 32);
                    let mut p = state.persistent.lock().await;
                    p.perf.max_threads = max_threads;
                    p.save();
                    Ok(serde_json::json!({ "max_threads": max_threads }))
                }
                "telegram_bandwidth_stats" => {
                    let mut p = state.persistent.lock().await;
                    p.bandwidth.normalize_day();
                    let used = p.bandwidth.used_today_bytes;
                    let total = p.bandwidth.total_used_bytes;
                    let quota = p.bandwidth.quota_bytes;
                    let date = p.bandwidth.date.clone();
                    let pct = if quota > 0 {
                        (used as f64 / quota as f64) * 100.0
                    } else {
                        0.0
                    };
                    Ok(serde_json::json!({
                        "used_today_bytes": used,
                        "total_used_bytes": total,
                        "quota_bytes": quota,
                        "date": date,
                        "percentage": pct,
                    }))
                }
                "telegram_bandwidth_set_quota" => {
                    let gb: u64 = args
                        .get("gb")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let quota_bytes = gb.saturating_mul(1024 * 1024 * 1024);
                    let mut p = state.persistent.lock().await;
                    p.bandwidth.normalize_day();
                    p.bandwidth.quota_bytes = quota_bytes;
                    let used = p.bandwidth.used_today_bytes;
                    p.save();
                    Ok(serde_json::json!({
                        "quota_bytes": quota_bytes,
                        "used_today_bytes": used,
                    }))
                }
                "telegram_bandwidth_reset" => {
                    let mut p = state.persistent.lock().await;
                    p.bandwidth.used_today_bytes = 0;
                    p.bandwidth.date = state::current_ymd();
                    let date = p.bandwidth.date.clone();
                    p.save();
                    Ok(serde_json::json!({
                        "used_today_bytes": 0,
                        "date": date,
                    }))
                }
                "telegram_sync_state" => {
                    let p = state.persistent.lock().await;
                    Ok(serde_json::json!({
                        "enabled": p.sync.enabled,
                        "interval_min": p.sync.interval_min,
                        "last_success_at": p.sync.last_success_at,
                        "last_duration_ms": p.sync.last_duration_ms,
                        "last_updated_count": p.sync.last_updated_count,
                        "is_syncing": p.sync.is_syncing,
                    }))
                }
                "telegram_sync_now" => {
                    {
                        let mut p = state.persistent.lock().await;
                        p.sync.is_syncing = true;
                    }
                    let t0 = std::time::Instant::now();
                    let res = commands::telegram::telegram_list_chats(&state).await;
                    let elapsed_ms = t0.elapsed().as_millis() as u64;
                    let mut p = state.persistent.lock().await;
                    p.sync.is_syncing = false;
                    match res {
                        Ok(chats) => {
                            let count = chats.len();
                            p.sync.last_success_at = state::unix_now_secs();
                            p.sync.last_duration_ms = elapsed_ms;
                            p.sync.last_updated_count = count;
                            p.save();
                            Ok(serde_json::json!({ "updated": count }))
                        }
                        Err(e) => Err(e),
                    }
                }
                "telegram_sync_settings_set" => {
                    let mut p = state.persistent.lock().await;
                    if let Some(enabled) = args
                        .get("enabled")
                        .and_then(|v| serde_json::from_value::<bool>(v.clone()).ok())
                    {
                        p.sync.enabled = enabled;
                    }
                    if let Some(interval) = args
                        .get("intervalMin")
                        .or(args.get("interval_min"))
                        .and_then(|v| serde_json::from_value::<u32>(v.clone()).ok())
                    {
                        p.sync.interval_min = interval.clamp(1, 1440);
                    }
                    p.save();
                    Ok(serde_json::json!({
                        "enabled": p.sync.enabled,
                        "interval_min": p.sync.interval_min,
                        "last_success_at": p.sync.last_success_at,
                        "last_duration_ms": p.sync.last_duration_ms,
                        "last_updated_count": p.sync.last_updated_count,
                        "is_syncing": p.sync.is_syncing,
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
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let info = platforms::telegram::api::get_full_channel_info(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(info).map_err(|e| e.to_string())
                }
                "telegram_list_participants" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let filter: String = args
                        .get("filter")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "recent".to_string());
                    let offset: i32 = args
                        .get("offset")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let limit: i32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let search: Option<String> = args
                        .get("search")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let page = platforms::telegram::api::list_participants(
                        &state.telegram_session,
                        chat_id,
                        &filter,
                        offset,
                        limit,
                        search.as_deref(),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(page).map_err(|e| e.to_string())
                }
                "telegram_accounts_list" => {
                    let p = state.persistent.lock().await;
                    serde_json::to_value(&p.accounts).map_err(|e| e.to_string())
                }
                "telegram_accounts_save_current" => {
                    let label: String = args
                        .get("label")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "Account".to_string());
                    let now = state::unix_now_secs();
                    let id = format!(
                        "acc_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );

                    let me = platforms::telegram::api::get_self(&state.telegram_session)
                        .await
                        .ok();
                    let phone_raw = {
                        let guard = state.telegram_session.lock().await;
                        guard.phone.clone()
                    };
                    let phone_redacted = me
                        .as_ref()
                        .and_then(|m| m.phone.as_deref())
                        .and_then(commands::telegram::redact_phone)
                        .or_else(|| commands::telegram::redact_phone(&phone_raw));

                    if let Ok(src_session) = platforms::telegram::auth::session_file_path() {
                        if src_session.exists() {
                            let acc_dir = state::accounts_dir();
                            let _ = std::fs::create_dir_all(&acc_dir);
                            let dst = acc_dir.join(format!("{}.session", id));
                            let _ = std::fs::copy(&src_session, &dst);
                        }
                    }

                    let entry = state::TelegramAccountEntry {
                        id,
                        label,
                        phone_redacted,
                        user_id: me.map(|m| m.user_id),
                        created_at: now,
                        updated_at: now,
                    };

                    let mut p = state.persistent.lock().await;
                    p.accounts.push(entry.clone());
                    p.save();
                    serde_json::to_value(entry).map_err(|e| e.to_string())
                }
                "telegram_accounts_restore" => {
                    let id: String =
                        serde_json::from_value(args.get("id").cloned().ok_or("missing 'id'")?)
                            .map_err(|e| e.to_string())?;
                    platforms::telegram::SubtleValidator::validate_safe_id(&id)?;
                    let saved = state::accounts_dir().join(format!("{}.session", id));
                    if !saved.exists() {
                        return Err(format!("Saved session file not found for '{}'", id));
                    }
                    let target = platforms::telegram::auth::session_file_path()
                        .map_err(|e| e.to_string())?;
                    platforms::telegram::auth::disconnect_and_clear_client(&state.telegram_session)
                        .await;
                    std::fs::copy(&saved, &target).map_err(|e| e.to_string())?;
                    let _ = platforms::telegram::auth::check_session(&state.telegram_session).await;
                    Ok(serde_json::json!({ "ok": true, "needs_restart": false }))
                }
                "telegram_accounts_remove" => {
                    let id: String =
                        serde_json::from_value(args.get("id").cloned().ok_or("missing 'id'")?)
                            .map_err(|e| e.to_string())?;
                    platforms::telegram::SubtleValidator::validate_safe_id(&id)?;
                    let saved = state::accounts_dir().join(format!("{}.session", id));
                    let _ = std::fs::remove_file(&saved);
                    let mut p = state.persistent.lock().await;
                    p.accounts.retain(|a| a.id != id);
                    p.save();
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_accounts_rename" => {
                    let id: String =
                        serde_json::from_value(args.get("id").cloned().ok_or("missing 'id'")?)
                            .map_err(|e| e.to_string())?;
                    let label: String = serde_json::from_value(
                        args.get("label").cloned().ok_or("missing 'label'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let now = state::unix_now_secs();
                    let mut p = state.persistent.lock().await;
                    let updated = p
                        .accounts
                        .iter_mut()
                        .find(|a| a.id == id)
                        .ok_or_else(|| format!("Account '{}' not found", id))?;
                    updated.label = label;
                    updated.updated_at = now;
                    let out = updated.clone();
                    p.save();
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "telegram_accounts_backup_now" => {
                    let label: String = args
                        .get("label")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "backup".to_string());
                    let clean_label = commands::telegram::sanitize_folder_name(&label);
                    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                    let name = format!("telegram_{}_{}.session", clean_label, ts);
                    let bdir = state::backups_dir();
                    std::fs::create_dir_all(&bdir).map_err(|e| e.to_string())?;
                    let dst = bdir.join(&name);
                    let src = platforms::telegram::auth::session_file_path()
                        .map_err(|e| e.to_string())?;
                    if src.exists() {
                        std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                    } else {
                        std::fs::write(&dst, b"").map_err(|e| e.to_string())?;
                    }
                    Ok(serde_json::json!({
                        "path": dst.to_string_lossy().to_string(),
                        "name": name,
                    }))
                }
                "telegram_accounts_list_backups" => {
                    let bdir = state::backups_dir();
                    let mut list = Vec::new();
                    if let Ok(rd) = std::fs::read_dir(&bdir) {
                        for entry in rd.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                let meta = entry.metadata().ok();
                                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                                let modified = meta
                                    .and_then(|m| m.modified().ok())
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                                    .to_string();
                                list.push(serde_json::json!({
                                    "name": name,
                                    "path": path.to_string_lossy().to_string(),
                                    "size": size,
                                    "modified_at": modified,
                                }));
                            }
                        }
                    }
                    Ok(serde_json::Value::Array(list))
                }
                "telegram_clone_list" => {
                    let p = state.persistent.lock().await;
                    serde_json::to_value(&p.clones).map_err(|e| e.to_string())
                }
                "telegram_clone_start" => {
                    let source_id: i64 = serde_json::from_value(
                        args.get("sourceId")
                            .or(args.get("source_id"))
                            .cloned()
                            .ok_or("missing 'sourceId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let source_type: String = args
                        .get("sourceType")
                        .or(args.get("source_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let source_title: Option<String> = args
                        .get("sourceTitle")
                        .or(args.get("source_title"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let dest_id: Option<i64> = args
                        .get("destId")
                        .or(args.get("dest_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let dest_type: Option<String> = args
                        .get("destType")
                        .or(args.get("dest_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let dest_title: Option<String> = args
                        .get("destTitle")
                        .or(args.get("dest_title"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let options: state::TelegramCloneOptions = args
                        .get("options")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    commands::telegram::telegram_clone_start(
                        host,
                        &state,
                        source_id,
                        source_type,
                        source_title,
                        dest_id,
                        dest_type,
                        dest_title,
                        options,
                    )
                    .await
                }
                "telegram_clone_status" => {
                    let session_id: String = args
                        .get("sessionId")
                        .or(args.get("session_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let p = state.persistent.lock().await;
                    if let Some(found) = p.clones.iter().find(|c| c.id == session_id) {
                        serde_json::to_value(found).map_err(|e| e.to_string())
                    } else {
                        Err(format!("Clone session '{}' not found", session_id))
                    }
                }
                "telegram_clone_pause" => {
                    let session_id: String = args
                        .get("sessionId")
                        .or(args.get("session_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    if let Some(tok) = state.active_ops.lock().await.get(&session_id) {
                        tok.cancel();
                    }
                    let mut p = state.persistent.lock().await;
                    if let Some(c) = p.clones.iter_mut().find(|c| c.id == session_id) {
                        c.status = "paused".to_string();
                        c.updated_at = state::unix_now_secs();
                        p.save();
                    }
                    Ok(serde_json::json!({ "cancelled": true }))
                }
                "telegram_clone_cancel" => {
                    let session_id: String = args
                        .get("sessionId")
                        .or(args.get("session_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    if let Some(tok) = state.active_ops.lock().await.get(&session_id) {
                        tok.cancel();
                    }
                    let mut p = state.persistent.lock().await;
                    if let Some(c) = p.clones.iter_mut().find(|c| c.id == session_id) {
                        c.status = "cancelled".to_string();
                        c.updated_at = state::unix_now_secs();
                        p.save();
                    }
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_clone_delete" => {
                    let session_id: String = args
                        .get("sessionId")
                        .or(args.get("session_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    if let Some(tok) = state.active_ops.lock().await.remove(&session_id) {
                        tok.cancel();
                    }
                    let mut p = state.persistent.lock().await;
                    p.clones.retain(|c| c.id != session_id);
                    p.save();
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_set_blocked" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "private".to_string());
                    let blocked: bool = args
                        .get("blocked")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    platforms::telegram::api::set_blocked(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        blocked,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_report_peer" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let message_ids: Vec<i32> = args
                        .get("messageIds")
                        .or(args.get("message_ids"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let option: Vec<u8> = args
                        .get("option")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let message: String = args
                        .get("message")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    platforms::telegram::api::report_peer(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        message_ids,
                        option,
                        &message,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_delete_channel" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    platforms::telegram::api::delete_channel(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_delete_history" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let revoke: bool = args
                        .get("revoke")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(false);
                    let just_clear: bool = args
                        .get("justClear")
                        .or(args.get("just_clear"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    let batches = platforms::telegram::api::delete_history(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        revoke,
                        just_clear,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true, "batches": batches }))
                }
                "telegram_leave_channel" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    platforms::telegram::api::leave_channel(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_rename_channel" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let new_title: String = serde_json::from_value(
                        args.get("newTitle")
                            .or(args.get("new_title"))
                            .cloned()
                            .ok_or("missing 'newTitle'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    platforms::telegram::api::rename_channel(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        &new_title,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "ok": true }))
                }
                "telegram_restore_peer_hashes" => {
                    let items: Vec<(i64, i64)> = args
                        .get("items")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let count = items.len();
                    let mut guard = state.telegram_session.lock().await;
                    for (id, hash) in items {
                        if hash != 0 {
                            guard.peer_hashes.insert(id, hash);
                        }
                    }
                    Ok(serde_json::json!({ "restored": count }))
                }
                "telegram_create_folder" => {
                    let name: String = args
                        .get("name")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let clean = name.trim().trim_end_matches("[og]").trim();
                    if clean.is_empty() {
                        return Err("Folder name cannot be empty".to_string());
                    }
                    let title = format!("{} [og]", clean);
                    let chat = platforms::telegram::api::create_channel_or_folder(
                        &state.telegram_session,
                        &title,
                        "LibreGet Drive Folder",
                        false,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(chat).map_err(|e| e.to_string())
                }
                "telegram_diag_list_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    platforms::telegram::api::diag_list_media(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                    )
                    .await
                    .map_err(|e| e.to_string())
                }
                "telegram_expand_album" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let message_id: i32 = serde_json::from_value(
                        args.get("messageId")
                            .or(args.get("message_id"))
                            .cloned()
                            .ok_or("missing 'messageId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let fix_extensions = settings_helper::load_settings()
                        .telegram
                        .fix_file_extensions;
                    let items = platforms::telegram::api::expand_album(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        message_id,
                        fix_extensions,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    serde_json::to_value(items).map_err(|e| e.to_string())
                }
                "telegram_takeout_start" => {
                    let id = platforms::telegram::api::takeout_start(&state.telegram_session)
                        .await
                        .unwrap_or(0);
                    let mut p = state.persistent.lock().await;
                    p.takeout_id = Some(id);
                    p.save();
                    Ok(serde_json::json!({ "takeout_id": id }))
                }
                "telegram_takeout_finish" => {
                    let success: bool = args
                        .get("success")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    let _ =
                        platforms::telegram::api::takeout_finish(&state.telegram_session, success)
                            .await;
                    let mut p = state.persistent.lock().await;
                    p.takeout_id = None;
                    p.save();
                    Ok(serde_json::Value::Null)
                }
                "telegram_bot_upload_file" => {
                    let file_path: String = serde_json::from_value(
                        args.get("filePath")
                            .or(args.get("file_path"))
                            .cloned()
                            .ok_or("missing 'filePath'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let bot_token: String = serde_json::from_value(
                        args.get("botToken")
                            .or(args.get("bot_token"))
                            .cloned()
                            .ok_or("missing 'botToken'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_id: String = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let caption: String = args
                        .get("caption")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    commands::telegram::telegram_bot_upload_file(
                        host, file_path, bot_token, chat_id, caption,
                    )
                    .await
                }
                "telegram_user_upload_file" => {
                    let file_path: String = serde_json::from_value(
                        args.get("filePath")
                            .or(args.get("file_path"))
                            .cloned()
                            .ok_or("missing 'filePath'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let destination: String = args
                        .get("destination")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "me".to_string());
                    let caption: String = args
                        .get("caption")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    commands::telegram::telegram_user_upload_file(
                        host,
                        &state,
                        file_path,
                        destination,
                        caption,
                    )
                    .await
                }
                "telegram_cancel_op" => {
                    let op_key: String = args
                        .get("opKey")
                        .or(args.get("op_key"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let cancelled = if let Some(tok) = state.active_ops.lock().await.remove(&op_key)
                    {
                        tok.cancel();
                        true
                    } else if let Ok(num_id) = op_key.parse::<u64>() {
                        if let Some((_, tok)) =
                            state.active_generic_downloads.lock().await.get(&num_id)
                        {
                            tok.cancel();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    Ok(serde_json::json!({ "cancelled": cancelled }))
                }
                "telegram_set_mute" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let muted: bool = args
                        .get("muted")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    platforms::telegram::api::set_mute(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        muted,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_toggle_pin" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let pinned: bool = args
                        .get("pinned")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    platforms::telegram::api::toggle_pin(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        pinned,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_set_archived" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chatId")
                            .or(args.get("chat_id"))
                            .cloned()
                            .ok_or("missing 'chatId'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = args
                        .get("chatType")
                        .or(args.get("chat_type"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| "channel".to_string());
                    let archived: bool = args
                        .get("archived")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(true);
                    platforms::telegram::api::set_archived(
                        &state.telegram_session,
                        chat_id,
                        &chat_type,
                        archived,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_reorder_pinned" => {
                    let folder_id: i32 = args
                        .get("folderId")
                        .or(args.get("folder_id"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let peers: Vec<(i64, String)> = args
                        .get("peers")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    let cid = item
                                        .get("chat_id")
                                        .or(item.get("chatId"))
                                        .and_then(|v| v.as_i64())?;
                                    let ctype = item
                                        .get("chat_type")
                                        .or(item.get("chatType"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("channel")
                                        .to_string();
                                    Some((cid, ctype))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    platforms::telegram::api::reorder_pinned(
                        &state.telegram_session,
                        folder_id,
                        peers,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(serde_json::Value::Null)
                }
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
            "telegram_get_self".into(),
            "telegram_list_chats".into(),
            "telegram_list_chats_page".into(),
            "telegram_list_media".into(),
            "telegram_download_media".into(),
            "telegram_download_batch".into(),
            "telegram_cancel_batch".into(),
            "telegram_get_thumbnail".into(),
            "telegram_search_media".into(),
            "telegram_search_global".into(),
            "telegram_search_global_hits".into(),
            "telegram_get_chat_photo".into(),
            "telegram_clear_thumbnail_cache".into(),
            "telegram_delete_messages".into(),
            "telegram_forward_messages".into(),
            "telegram_edit_caption".into(),
            "telegram_stream_info".into(),
            "telegram_upload_media".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn all_declared_commands_are_routed_without_unknown_command() {
        let plugin = TelegramPlugin::new();
        let declared = plugin.commands();
        assert_eq!(declared.len(), 67);

        let formerly_missing = [
            "telegram_search_global_hits",
            "telegram_search_global",
            "telegram_get_self",
            "telegram_stream_info",
            "telegram_list_chats_page",
            "telegram_delete_messages",
            "telegram_forward_messages",
            "telegram_edit_caption",
            "telegram_upload_media",
        ];
        for cmd in formerly_missing {
            assert!(
                declared.contains(&cmd.to_string()),
                "missing command in manifest: {cmd}"
            );
        }

        for cmd in declared {
            let res = plugin
                .handle_command(cmd.clone(), serde_json::json!({}))
                .await;
            if let Err(err) = res {
                assert!(
                    !err.contains("Unknown command"),
                    "command {cmd} returned Unknown command: {err}"
                );
            }
        }
    }

    #[tokio::test]
    async fn perf_bandwidth_stream_and_peer_hashes_work() {
        let plugin = TelegramPlugin::new();

        let perf_res = plugin
            .handle_command(
                "telegram_perf_set".into(),
                serde_json::json!({ "maxThreads": 12 }),
            )
            .await
            .unwrap();
        assert_eq!(perf_res["max_threads"], 12);

        let perf_get = plugin
            .handle_command("telegram_perf_get".into(), serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(perf_get["max_threads"], 12);

        let restore = plugin
            .handle_command(
                "telegram_restore_peer_hashes".into(),
                serde_json::json!({ "items": [[123456_i64, 987654321_i64]] }),
            )
            .await
            .unwrap();
        assert_eq!(restore["restored"], 1);

        let stream_info = plugin
            .handle_command("telegram_stream_info".into(), serde_json::json!({}))
            .await
            .unwrap();
        assert!(stream_info["port"].as_u64().unwrap_or(0) > 0);
        assert!(!stream_info["token"].as_str().unwrap_or("").is_empty());
    }
}
