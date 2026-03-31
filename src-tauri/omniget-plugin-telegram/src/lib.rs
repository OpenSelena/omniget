pub mod platforms;

use std::collections::HashMap;
use std::sync::Arc;
use serde::Serialize;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use base64::Engine;
use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};
use platforms::telegram::api::{self, TelegramChat, TelegramMediaItem};
use platforms::telegram::auth::{self, TelegramSessionHandle, TelegramState, QrPollStatus, VerifyError};
use platforms::telegram::thumbnail_cache;

pub struct TelegramPlugin {
    host: Option<Arc<dyn PluginHost>>,
    telegram_session: TelegramSessionHandle,
    active_downloads: Arc<tokio::sync::Mutex<HashMap<u64, (String, CancellationToken)>>>,
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            telegram_session: Arc::new(tokio::sync::Mutex::new(TelegramState::new())),
            active_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    fn get_setting(&self, key: &str) -> serde_json::Value {
        self.host.as_ref()
            .map(|h| h.get_settings("telegram"))
            .and_then(|s| s.get(key).cloned())
            .unwrap_or(serde_json::Value::Null)
    }

    fn fix_file_extensions(&self) -> bool {
        self.get_setting("fix_file_extensions").as_bool().unwrap_or(false)
    }

    fn concurrent_downloads(&self) -> usize {
        self.get_setting("concurrent_downloads").as_u64().unwrap_or(3) as usize
    }
}

#[derive(Clone, Serialize)]
struct GenericDownloadProgress { id: u64, title: String, platform: String, percent: f64 }

#[derive(Clone, Serialize)]
struct GenericDownloadComplete { id: u64, title: String, platform: String, success: bool, error: Option<String>, file_path: Option<String>, file_size_bytes: Option<u64>, file_count: Option<u32> }

#[derive(Clone, Serialize)]
struct BatchFileStatus { batch_id: u64, message_id: i32, status: String, percent: f64, error: Option<String> }

#[derive(Clone, serde::Deserialize)]
struct BatchItem { message_id: i32, file_name: String, file_size: u64 }

impl OmnigetPlugin for TelegramPlugin {
    fn id(&self) -> &str { "telegram" }
    fn name(&self) -> &str { "Telegram Downloader" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>> {
        let host = self.host.clone();
        let session = self.telegram_session.clone();
        let active = self.active_downloads.clone();
        let fix_ext = self.fix_file_extensions();
        let concurrent = self.concurrent_downloads().clamp(1, 10);

        Box::pin(async move {
            match command.as_str() {
                "telegram_check_session" => {
                    let r = auth::check_session(&session).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::to_value(r).map_err(|e| e.to_string())?)
                }

                "telegram_qr_start" => {
                    let r = auth::qr_login_start(&session).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({ "svg": r.qr_svg, "expires": r.expires }))
                }

                "telegram_qr_poll" => {
                    match auth::qr_login_poll(&session).await {
                        Ok(QrPollStatus::Waiting) => Ok(serde_json::json!("waiting")),
                        Ok(QrPollStatus::Success { phone }) => Ok(serde_json::json!(format!("success:{}", phone))),
                        Ok(QrPollStatus::PasswordRequired { hint }) => {
                            if hint.is_empty() { Ok(serde_json::json!("password_required")) }
                            else { Ok(serde_json::json!(format!("password_required:{}", hint))) }
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }

                "telegram_send_code" => {
                    let phone: String = serde_json::from_value(args.get("phone").cloned().ok_or("missing 'phone'")?).map_err(|e| e.to_string())?;
                    auth::send_code(&session, &phone).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(null))
                }

                "telegram_verify_code" => {
                    let code: String = serde_json::from_value(args.get("code").cloned().ok_or("missing 'code'")?).map_err(|e| e.to_string())?;
                    match auth::verify_code(&session, &code).await {
                        Ok(phone) => Ok(serde_json::json!(phone)),
                        Err(VerifyError::InvalidCode) => Err("invalid_code".into()),
                        Err(VerifyError::PasswordRequired { hint }) => Err(format!("password_required:{}", hint)),
                        Err(VerifyError::NoSession) => Err("no_session".into()),
                        Err(VerifyError::Other(msg)) => Err(msg),
                    }
                }

                "telegram_verify_2fa" => {
                    let password: String = serde_json::from_value(args.get("password").cloned().ok_or("missing 'password'")?).map_err(|e| e.to_string())?;
                    let r = auth::verify_password(&session, &password).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(r))
                }

                "telegram_logout" => {
                    thumbnail_cache::clear_cache().await;
                    auth::logout(&session).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(null))
                }

                "telegram_list_chats" => {
                    let r = api::list_chats(&session).await.map_err(|e| e.to_string())?;
                    serde_json::to_value(r).map_err(|e| e.to_string())
                }

                "telegram_list_media" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args.get("media_type").or(args.get("mediaType")).cloned().and_then(|v| serde_json::from_value(v).ok());
                    let offset: i32 = serde_json::from_value(args.get("offset").cloned().unwrap_or(serde_json::json!(0))).map_err(|e| e.to_string())?;
                    let limit: u32 = serde_json::from_value(args.get("limit").cloned().unwrap_or(serde_json::json!(50))).map_err(|e| e.to_string())?;
                    let r = api::list_media(&session, chat_id, &chat_type, media_type.as_deref(), offset, limit, fix_ext).await.map_err(|e| e.to_string())?;
                    serde_json::to_value(r).map_err(|e| e.to_string())
                }

                "telegram_download_media" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let message_id: i32 = serde_json::from_value(args.get("message_id").or(args.get("messageId")).cloned().ok_or("missing 'messageId'")?).map_err(|e| e.to_string())?;
                    let file_name: String = serde_json::from_value(args.get("file_name").or(args.get("fileName")).cloned().ok_or("missing 'fileName'")?).map_err(|e| e.to_string())?;
                    let output_dir: String = serde_json::from_value(args.get("output_dir").or(args.get("outputDir")).cloned().ok_or("missing 'outputDir'")?).map_err(|e| e.to_string())?;

                    let download_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                    let cancel_token = CancellationToken::new();
                    {
                        let mut a = active.lock().await;
                        let key = format!("tg:{}:{}", chat_id, message_id);
                        if a.values().any(|(u, _)| u == &key) { return Err("Download already in progress".into()); }
                        a.insert(download_id, (key, cancel_token.clone()));
                    }

                    let host = host.ok_or("Plugin not initialized")?;
                    let active = active.clone();
                    let file_name_clone = file_name.clone();

                    tokio::spawn(async move {
                        let output_path = std::path::PathBuf::from(&output_dir).join(&file_name_clone);
                        let _ = host.emit_event("generic-download-progress", serde_json::to_value(&GenericDownloadProgress { id: download_id, title: file_name_clone.clone(), platform: "telegram".into(), percent: 0.0 }).unwrap_or_default());
                        let (tx, mut rx) = mpsc::channel::<f64>(32);
                        let host_p = host.clone();
                        let fn_p = file_name_clone.clone();
                        let pf = tokio::spawn(async move { while let Some(p) = rx.recv().await { let _ = host_p.emit_event("generic-download-progress", serde_json::to_value(&GenericDownloadProgress { id: download_id, title: fn_p.clone(), platform: "telegram".into(), percent: p }).unwrap_or_default()); } });
                        let result = api::download_media_with_retry(&session, chat_id, &chat_type, message_id, &output_path, tx, &cancel_token).await;
                        let _ = pf.await;
                        active.lock().await.remove(&download_id);
                        match result {
                            Ok(size) => { let _ = host.emit_event("generic-download-complete", serde_json::to_value(&GenericDownloadComplete { id: download_id, title: file_name_clone, platform: "telegram".into(), success: true, error: None, file_path: Some(output_path.to_string_lossy().into()), file_size_bytes: Some(size), file_count: Some(1) }).unwrap_or_default()); }
                            Err(e) => { let _ = host.emit_event("generic-download-complete", serde_json::to_value(&GenericDownloadComplete { id: download_id, title: file_name_clone, platform: "telegram".into(), success: false, error: Some(e.to_string()), file_path: None, file_size_bytes: None, file_count: None }).unwrap_or_default()); }
                        }
                    });
                    Ok(serde_json::json!({ "id": download_id, "file_name": file_name }))
                }

                "telegram_download_batch" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let chat_title: String = serde_json::from_value(args.get("chat_title").or(args.get("chatTitle")).cloned().ok_or("missing 'chatTitle'")?).map_err(|e| e.to_string())?;
                    let items: Vec<BatchItem> = serde_json::from_value(args.get("items").cloned().ok_or("missing 'items'")?).map_err(|e| e.to_string())?;
                    let output_dir: String = serde_json::from_value(args.get("output_dir").or(args.get("outputDir")).cloned().ok_or("missing 'outputDir'")?).map_err(|e| e.to_string())?;

                    let batch_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                    let cancel_token = CancellationToken::new();
                    { let mut a = active.lock().await; a.insert(batch_id, (format!("tg-batch:{}", batch_id), cancel_token.clone())); }

                    let host = host.ok_or("Plugin not initialized")?;
                    let active = active.clone();
                    let total_files = items.len() as u32;

                    let _ = host.emit_event("generic-download-progress", serde_json::to_value(&GenericDownloadProgress { id: batch_id, title: chat_title.clone(), platform: "telegram".into(), percent: 0.0 }).unwrap_or_default());

                    tokio::spawn(async move {
                        let sem = Arc::new(Semaphore::new(concurrent));
                        let completed = Arc::new(std::sync::atomic::AtomicU32::new(0));
                        let failed = Arc::new(std::sync::atomic::AtomicU32::new(0));
                        let skipped = Arc::new(std::sync::atomic::AtomicU32::new(0));
                        let mut handles = Vec::new();

                        for item in items {
                            let sem = sem.clone(); let session = session.clone(); let host = host.clone();
                            let chat_type = chat_type.clone(); let chat_title = chat_title.clone();
                            let output_dir = output_dir.clone(); let cancel = cancel_token.clone();
                            let completed = completed.clone(); let failed = failed.clone(); let skipped = skipped.clone();

                            handles.push(tokio::spawn(async move {
                                if cancel.is_cancelled() { return; }
                                let output_path = std::path::PathBuf::from(&output_dir).join(&item.file_name);

                                if let Ok(true) = tokio::fs::try_exists(&output_path).await {
                                    if let Ok(meta) = tokio::fs::metadata(&output_path).await {
                                        if meta.len() > 0 {
                                            skipped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                            let done = completed.load(std::sync::atomic::Ordering::Relaxed) + failed.load(std::sync::atomic::Ordering::Relaxed) + skipped.load(std::sync::atomic::Ordering::Relaxed);
                                            let _ = host.emit_event("telegram-batch-file-status", serde_json::to_value(&BatchFileStatus { batch_id, message_id: item.message_id, status: "skipped".into(), percent: 100.0, error: None }).unwrap_or_default());
                                            let _ = host.emit_event("generic-download-progress", serde_json::to_value(&GenericDownloadProgress { id: batch_id, title: chat_title, platform: "telegram".into(), percent: (done as f64 / total_files as f64) * 100.0 }).unwrap_or_default());
                                            return;
                                        }
                                    }
                                }

                                let _permit = tokio::select! { p = sem.acquire() => match p { Ok(p) => p, Err(_) => return }, _ = cancel.cancelled() => return };
                                if cancel.is_cancelled() { return; }

                                let _ = host.emit_event("telegram-batch-file-status", serde_json::to_value(&BatchFileStatus { batch_id, message_id: item.message_id, status: "downloading".into(), percent: 0.0, error: None }).unwrap_or_default());
                                let (tx, mut rx) = mpsc::channel::<f64>(32);
                                let host_p = host.clone();
                                let mid = item.message_id;
                                let pf = tokio::spawn(async move { while let Some(p) = rx.recv().await { let _ = host_p.emit_event("telegram-batch-file-status", serde_json::to_value(&BatchFileStatus { batch_id, message_id: mid, status: "downloading".into(), percent: p, error: None }).unwrap_or_default()); } });

                                let result = api::download_media_with_retry(&session, chat_id, &chat_type, item.message_id, &output_path, tx, &cancel).await;
                                let _ = pf.await;

                                match result {
                                    Ok(_) => { completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed); let _ = host.emit_event("telegram-batch-file-status", serde_json::to_value(&BatchFileStatus { batch_id, message_id: item.message_id, status: "done".into(), percent: 100.0, error: None }).unwrap_or_default()); }
                                    Err(e) => { if e.to_string().contains("cancelled") { return; } failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed); let _ = host.emit_event("telegram-batch-file-status", serde_json::to_value(&BatchFileStatus { batch_id, message_id: item.message_id, status: "error".into(), percent: 0.0, error: Some(e.to_string()) }).unwrap_or_default()); }
                                }

                                let done = completed.load(std::sync::atomic::Ordering::Relaxed) + failed.load(std::sync::atomic::Ordering::Relaxed) + skipped.load(std::sync::atomic::Ordering::Relaxed);
                                let _ = host.emit_event("generic-download-progress", serde_json::to_value(&GenericDownloadProgress { id: batch_id, title: chat_title, platform: "telegram".into(), percent: (done as f64 / total_files as f64) * 100.0 }).unwrap_or_default());
                            }));
                        }

                        for h in handles { let _ = h.await; }
                        active.lock().await.remove(&batch_id);
                        let done_count = completed.load(std::sync::atomic::Ordering::Relaxed);
                        let fail_count = failed.load(std::sync::atomic::Ordering::Relaxed);
                        let skip_count = skipped.load(std::sync::atomic::Ordering::Relaxed);
                        let _ = host.emit_event("generic-download-complete", serde_json::to_value(&GenericDownloadComplete {
                            id: batch_id, title: chat_title, platform: "telegram".into(),
                            success: fail_count == 0 && !cancel_token.is_cancelled(),
                            error: if cancel_token.is_cancelled() { Some("Batch cancelled".into()) } else if fail_count > 0 { Some(format!("{} files failed", fail_count)) } else { None },
                            file_path: Some(output_dir), file_size_bytes: None, file_count: Some(done_count + skip_count),
                        }).unwrap_or_default());
                    });
                    Ok(serde_json::json!(batch_id))
                }

                "telegram_cancel_batch" => {
                    let batch_id: u64 = serde_json::from_value(args.get("batch_id").or(args.get("batchId")).cloned().ok_or("missing 'batchId'")?).map_err(|e| e.to_string())?;
                    let a = active.lock().await;
                    if let Some((key, token)) = a.get(&batch_id) {
                        if key.starts_with("tg-batch:") { token.cancel(); return Ok(serde_json::json!(null)); }
                    }
                    Err("Batch not found".into())
                }

                "telegram_get_thumbnail" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let message_id: i32 = serde_json::from_value(args.get("message_id").or(args.get("messageId")).cloned().ok_or("missing 'messageId'")?).map_err(|e| e.to_string())?;
                    let data = thumbnail_cache::get_thumbnail(&session, chat_id, &chat_type, message_id).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&data)))
                }

                "telegram_search_media" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let query: String = serde_json::from_value(args.get("query").cloned().ok_or("missing 'query'")?).map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args.get("media_type").or(args.get("mediaType")).cloned().and_then(|v| serde_json::from_value(v).ok());
                    let limit: u32 = serde_json::from_value(args.get("limit").cloned().unwrap_or(serde_json::json!(50))).map_err(|e| e.to_string())?;
                    let r = api::search_media(&session, chat_id, &chat_type, &query, media_type.as_deref(), limit, fix_ext).await.map_err(|e| e.to_string())?;
                    serde_json::to_value(r).map_err(|e| e.to_string())
                }

                "telegram_get_chat_photo" => {
                    let chat_id: i64 = serde_json::from_value(args.get("chat_id").or(args.get("chatId")).cloned().ok_or("missing 'chatId'")?).map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(args.get("chat_type").or(args.get("chatType")).cloned().ok_or("missing 'chatType'")?).map_err(|e| e.to_string())?;
                    let data = thumbnail_cache::get_chat_photo(&session, chat_id, &chat_type).await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&data)))
                }

                "telegram_clear_thumbnail_cache" => {
                    thumbnail_cache::clear_cache().await;
                    Ok(serde_json::json!(null))
                }

                _ => Err(format!("Unknown command: {}", command)),
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "telegram_check_session".into(), "telegram_qr_start".into(), "telegram_qr_poll".into(),
            "telegram_send_code".into(), "telegram_verify_code".into(), "telegram_verify_2fa".into(),
            "telegram_logout".into(), "telegram_list_chats".into(), "telegram_list_media".into(),
            "telegram_download_media".into(), "telegram_download_batch".into(), "telegram_cancel_batch".into(),
            "telegram_get_thumbnail".into(), "telegram_search_media".into(), "telegram_get_chat_photo".into(),
            "telegram_clear_thumbnail_cache".into(),
        ]
    }
}

omniget_plugin_sdk::export_plugin!(TelegramPlugin::new());
