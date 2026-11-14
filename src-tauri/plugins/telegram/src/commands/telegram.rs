use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use base64::Engine;
use libreget_plugin_sdk::PluginHost;

use crate::platforms::telegram::api::{self, TelegramChat, TelegramMediaItem};
use crate::platforms::telegram::auth::{self, QrPollStatus, VerifyError};
use crate::platforms::telegram::thumbnail_cache;
use crate::settings_helper;
use crate::state::TelegramPluginState;

fn emit_event<T: Serialize>(host: &Option<Arc<dyn PluginHost>>, name: &str, payload: &T) {
    if let Some(h) = host {
        if let Ok(val) = serde_json::to_value(payload) {
            let _ = h.emit_event(name, val);
        }
    }
}

#[derive(Clone, Serialize)]
struct GenericDownloadProgress {
    id: u64,
    title: String,
    platform: String,
    percent: f64,
}

#[derive(Clone, Serialize)]
struct GenericDownloadComplete {
    id: u64,
    title: String,
    platform: String,
    success: bool,
    error: Option<String>,
    file_path: Option<String>,
    file_size_bytes: Option<u64>,
    file_count: Option<u32>,
}

#[derive(Clone, Serialize)]
pub struct TelegramDownloadStarted {
    pub id: u64,
    pub file_name: String,
}

pub async fn telegram_check_session(state: &TelegramPluginState) -> Result<String, String> {
    auth::check_session(&state.telegram_session)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
pub struct QrStartResponse {
    pub svg: String,
    pub expires: i32,
}

pub async fn telegram_qr_start(state: &TelegramPluginState) -> Result<QrStartResponse, String> {
    let result = auth::qr_login_start(&state.telegram_session)
        .await
        .map_err(|e| e.to_string())?;
    Ok(QrStartResponse {
        svg: result.qr_svg,
        expires: result.expires,
    })
}

pub async fn telegram_qr_poll(state: &TelegramPluginState) -> Result<String, String> {
    match auth::qr_login_poll(&state.telegram_session).await {
        Ok(QrPollStatus::Waiting) => Ok("waiting".to_string()),
        Ok(QrPollStatus::Success { phone }) => Ok(format!("success:{}", phone)),
        Ok(QrPollStatus::PasswordRequired { hint }) => {
            if hint.is_empty() {
                Ok("password_required".to_string())
            } else {
                Ok(format!("password_required:{}", hint))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn telegram_send_code(state: &TelegramPluginState, phone: String) -> Result<(), String> {
    auth::send_code(&state.telegram_session, &phone)
        .await
        .map_err(|e| e.to_string())
}

pub async fn telegram_verify_code(
    state: &TelegramPluginState,
    code: String,
) -> Result<String, String> {
    match auth::verify_code(&state.telegram_session, &code).await {
        Ok(phone) => Ok(phone),
        Err(VerifyError::InvalidCode) => Err("invalid_code".to_string()),
        Err(VerifyError::PasswordRequired { hint }) => Err(format!("password_required:{}", hint)),
        Err(VerifyError::NoSession) => Err("no_session".to_string()),
        Err(VerifyError::Other(msg)) => Err(msg),
    }
}

pub async fn telegram_verify_2fa(
    state: &TelegramPluginState,
    password: String,
) -> Result<String, String> {
    auth::verify_password(&state.telegram_session, &password)
        .await
        .map_err(|e| e.to_string())
}

pub async fn telegram_logout(state: &TelegramPluginState) -> Result<(), String> {
    thumbnail_cache::clear_cache().await;
    auth::logout(&state.telegram_session)
        .await
        .map_err(|e| e.to_string())
}

pub async fn telegram_list_chats(state: &TelegramPluginState) -> Result<Vec<TelegramChat>, String> {
    api::list_chats(&state.telegram_session)
        .await
        .map_err(|e| e.to_string())
}

pub async fn telegram_list_media(
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    media_type: Option<String>,
    offset: i32,
    limit: u32,
) -> Result<Vec<TelegramMediaItem>, String> {
    let fix_extensions = settings_helper::load_settings()
        .telegram
        .fix_file_extensions;
    api::list_media(
        &state.telegram_session,
        chat_id,
        &chat_type,
        media_type.as_deref(),
        offset,
        limit,
        fix_extensions,
    )
    .await
    .map_err(|e| e.to_string())
}

pub fn sanitize_folder_name(name: &str) -> String {
    let forbidden = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let cleaned: String = name
        .chars()
        .map(|c| {
            if forbidden.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        return "Telegram Media".to_string();
    }

    let upper = trimmed.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    if is_reserved {
        format!("{}_channel", trimmed)
    } else if trimmed.chars().count() > 100 {
        trimmed.chars().take(100).collect()
    } else {
        trimmed.to_string()
    }
}

pub fn resolve_telegram_download_dir(
    base_dir: &str,
    chat_title: Option<&str>,
) -> std::path::PathBuf {
    let base = std::path::PathBuf::from(base_dir);
    let is_already_folder = base
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case("LibreGram"))
        .unwrap_or(false);

    let libregram_root = if is_already_folder {
        base
    } else {
        base.join("LibreGram")
    };

    match chat_title {
        Some(title) if !title.trim().is_empty() => {
            let clean = sanitize_folder_name(title);
            libregram_root.join(clean)
        }
        _ => libregram_root,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn telegram_download_media(
    host: Option<Arc<dyn PluginHost>>,
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    chat_title: Option<String>,
    message_id: i32,
    file_name: String,
    output_dir: String,
) -> Result<TelegramDownloadStarted, String> {
    tracing::info!(
        "[tg-cmd] telegram_download_media: chat_id={}, chat_type={}, chat_title={:?}, message_id={}, file_name={}",
        chat_id,
        chat_type,
        chat_title,
        message_id,
        file_name
    );

    let download_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let cancel_token = CancellationToken::new();

    {
        let mut active = state.active_generic_downloads.lock().await;
        let key = format!("tg:{}:{}", chat_id, message_id);
        if active.values().any(|(u, _)| u == &key) {
            return Err("Download already in progress for this media".to_string());
        }
        active.insert(download_id, (key, cancel_token.clone()));
    }

    let session = state.telegram_session.clone();
    let active_downloads = state.active_generic_downloads.clone();
    let persistent = state.persistent.clone();
    let file_name_clone = file_name.clone();

    tokio::spawn(async move {
        let target_dir = resolve_telegram_download_dir(&output_dir, chat_title.as_deref());
        if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
            tracing::warn!(
                "[tg-cmd] Failed to create target directory {:?}: {}",
                target_dir,
                e
            );
        }
        let output_path = target_dir.join(&file_name_clone);

        emit_event(
            &host,
            "generic-download-progress",
            &GenericDownloadProgress {
                id: download_id,
                title: file_name_clone.clone(),
                platform: "telegram".to_string(),
                percent: 0.0,
            },
        );

        let (tx, mut rx) = mpsc::channel::<libreget_core::models::progress::ProgressUpdate>(32);

        let host_progress = host.clone();
        let file_name_progress = file_name_clone.clone();
        let progress_forwarder = tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                emit_event(
                    &host_progress,
                    "generic-download-progress",
                    &GenericDownloadProgress {
                        id: download_id,
                        title: file_name_progress.clone(),
                        platform: "telegram".to_string(),
                        percent: update.percent,
                    },
                );
            }
        });

        let result = api::download_media_with_retry(
            &session,
            chat_id,
            &chat_type,
            message_id,
            &output_path,
            tx,
            &cancel_token,
        )
        .await;

        let _ = progress_forwarder.await;

        {
            active_downloads.lock().await.remove(&download_id);
        }

        match result {
            Ok(size) => {
                {
                    let mut p = persistent.lock().await;
                    p.bandwidth.record_bytes(size);
                    p.save();
                }
                emit_event(
                    &host,
                    "generic-download-complete",
                    &GenericDownloadComplete {
                        id: download_id,
                        title: file_name_clone,
                        platform: "telegram".to_string(),
                        success: true,
                        error: None,
                        file_path: Some(output_path.to_string_lossy().to_string()),
                        file_size_bytes: Some(size),
                        file_count: Some(1),
                    },
                );
            }
            Err(e) => {
                emit_event(
                    &host,
                    "generic-download-complete",
                    &GenericDownloadComplete {
                        id: download_id,
                        title: file_name_clone,
                        platform: "telegram".to_string(),
                        success: false,
                        error: Some(e.to_string()),
                        file_path: None,
                        file_size_bytes: None,
                        file_count: None,
                    },
                );
            }
        }
    });

    Ok(TelegramDownloadStarted {
        id: download_id,
        file_name,
    })
}

#[derive(Clone, Deserialize)]
pub struct BatchItem {
    pub message_id: i32,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Clone, Serialize)]
struct BatchFileStatus {
    batch_id: u64,
    message_id: i32,
    status: String, // "waiting" | "downloading" | "done" | "error" | "skipped"
    percent: f64,
    error: Option<String>,
}

pub async fn telegram_download_batch(
    host: Option<Arc<dyn PluginHost>>,
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    chat_title: String,
    items: Vec<BatchItem>,
    output_dir: String,
) -> Result<u64, String> {
    tracing::info!(
        "[tg-cmd] telegram_download_batch: chat_id={}, chat_type={}, items={}, output_dir={}",
        chat_id,
        chat_type,
        items.len(),
        output_dir
    );

    let batch_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let cancel_token = CancellationToken::new();

    {
        let mut active = state.active_generic_downloads.lock().await;
        let key = format!("tg-batch:{}", batch_id);
        active.insert(batch_id, (key, cancel_token.clone()));
    }

    let session = state.telegram_session.clone();
    let active_downloads = state.active_generic_downloads.clone();
    let persistent = state.persistent.clone();
    let total_files = items.len() as u32;

    let tg_settings = settings_helper::load_settings().telegram;
    let perf_threads = {
        let p = state.persistent.lock().await;
        p.perf.max_threads
    };
    let concurrent = tg_settings
        .concurrent_downloads
        .max(perf_threads.min(10))
        .clamp(1, 10) as usize;

    // Emit initial 0% batch progress
    emit_event(
        &host,
        "generic-download-progress",
        &GenericDownloadProgress {
            id: batch_id,
            title: chat_title.clone(),
            platform: "telegram".to_string(),
            percent: 0.0,
        },
    );

    let target_dir = resolve_telegram_download_dir(&output_dir, Some(&chat_title));
    if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
        tracing::warn!(
            "[tg-cmd] Failed to create target directory {:?}: {}",
            target_dir,
            e
        );
    }
    let target_dir_str = target_dir.to_string_lossy().to_string();

    tokio::spawn(async move {
        let semaphore = Arc::new(Semaphore::new(concurrent));
        let completed = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let failed = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let skipped = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let mut handles = Vec::new();

        for item in items {
            let sem = semaphore.clone();
            let session = session.clone();
            let persistent = persistent.clone();
            let host = host.clone();
            let chat_type = chat_type.clone();
            let chat_title = chat_title.clone();
            let target_dir_str = target_dir_str.clone();
            let cancel = cancel_token.clone();
            let completed = completed.clone();
            let failed = failed.clone();
            let skipped = skipped.clone();

            let handle = tokio::spawn(async move {
                // Check cancellation before acquiring permit
                if cancel.is_cancelled() {
                    return;
                }

                let output_path = std::path::PathBuf::from(&target_dir_str).join(&item.file_name);

                // Skip existing files
                if let Ok(true) = tokio::fs::try_exists(&output_path).await {
                    if let Ok(meta) = tokio::fs::metadata(&output_path).await {
                        if meta.len() > 0 {
                            skipped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let done = completed.load(std::sync::atomic::Ordering::Relaxed)
                                + failed.load(std::sync::atomic::Ordering::Relaxed)
                                + skipped.load(std::sync::atomic::Ordering::Relaxed);
                            let percent = (done as f64 / total_files as f64) * 100.0;

                            emit_event(
                                &host,
                                "telegram-batch-file-status",
                                &BatchFileStatus {
                                    batch_id,
                                    message_id: item.message_id,
                                    status: "skipped".to_string(),
                                    percent: 100.0,
                                    error: None,
                                },
                            );
                            emit_event(
                                &host,
                                "generic-download-progress",
                                &GenericDownloadProgress {
                                    id: batch_id,
                                    title: chat_title,
                                    platform: "telegram".to_string(),
                                    percent,
                                },
                            );
                            return;
                        }
                    }
                }

                // Acquire semaphore permit
                let _permit = tokio::select! {
                    p = sem.acquire() => match p {
                        Ok(p) => p,
                        Err(_) => return,
                    },
                    _ = cancel.cancelled() => return,
                };

                if cancel.is_cancelled() {
                    return;
                }

                // Emit downloading status
                emit_event(
                    &host,
                    "telegram-batch-file-status",
                    &BatchFileStatus {
                        batch_id,
                        message_id: item.message_id,
                        status: "downloading".to_string(),
                        percent: 0.0,
                        error: None,
                    },
                );

                let (tx, mut rx) =
                    mpsc::channel::<libreget_core::models::progress::ProgressUpdate>(32);

                let host_progress = host.clone();
                let progress_forwarder = tokio::spawn(async move {
                    while let Some(update) = rx.recv().await {
                        emit_event(
                            &host_progress,
                            "telegram-batch-file-status",
                            &BatchFileStatus {
                                batch_id,
                                message_id: item.message_id,
                                status: "downloading".to_string(),
                                percent: update.percent,
                                error: None,
                            },
                        );
                    }
                });

                let result = api::download_media_with_retry(
                    &session,
                    chat_id,
                    &chat_type,
                    item.message_id,
                    &output_path,
                    tx,
                    &cancel,
                )
                .await;

                let _ = progress_forwarder.await;

                match result {
                    Ok(size) => {
                        {
                            let mut p = persistent.lock().await;
                            p.bandwidth.record_bytes(size);
                            p.save();
                        }
                        completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        emit_event(
                            &host,
                            "telegram-batch-file-status",
                            &BatchFileStatus {
                                batch_id,
                                message_id: item.message_id,
                                status: "done".to_string(),
                                percent: 100.0,
                                error: None,
                            },
                        );
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("cancelled") {
                            return;
                        }
                        failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        emit_event(
                            &host,
                            "telegram-batch-file-status",
                            &BatchFileStatus {
                                batch_id,
                                message_id: item.message_id,
                                status: "error".to_string(),
                                percent: 0.0,
                                error: Some(err_str),
                            },
                        );
                    }
                }

                let done = completed.load(std::sync::atomic::Ordering::Relaxed)
                    + failed.load(std::sync::atomic::Ordering::Relaxed)
                    + skipped.load(std::sync::atomic::Ordering::Relaxed);
                let percent = (done as f64 / total_files as f64) * 100.0;

                emit_event(
                    &host,
                    "generic-download-progress",
                    &GenericDownloadProgress {
                        id: batch_id,
                        title: chat_title,
                        platform: "telegram".to_string(),
                        percent,
                    },
                );
            });

            handles.push(handle);
        }

        // Wait for all file downloads to complete
        for h in handles {
            let _ = h.await;
        }

        // Clean up active download entry
        {
            active_downloads.lock().await.remove(&batch_id);
        }

        let done_count = completed.load(std::sync::atomic::Ordering::Relaxed);
        let fail_count = failed.load(std::sync::atomic::Ordering::Relaxed);
        let skip_count = skipped.load(std::sync::atomic::Ordering::Relaxed);
        let success = fail_count == 0 && !cancel_token.is_cancelled();

        emit_event(
            &host,
            "generic-download-complete",
            &GenericDownloadComplete {
                id: batch_id,
                title: chat_title,
                platform: "telegram".to_string(),
                success,
                error: if cancel_token.is_cancelled() {
                    Some("Batch cancelled".to_string())
                } else if fail_count > 0 {
                    Some(format!("{} files failed", fail_count))
                } else {
                    None
                },
                file_path: Some(target_dir_str),
                file_size_bytes: None,
                file_count: Some(done_count + skip_count),
            },
        );
    });

    Ok(batch_id)
}

pub async fn telegram_cancel_batch(
    state: &TelegramPluginState,
    batch_id: u64,
) -> Result<(), String> {
    let active = state.active_generic_downloads.lock().await;
    if let Some((key, token)) = active.get(&batch_id) {
        if key.starts_with("tg-batch:") {
            token.cancel();
            return Ok(());
        }
    }
    Err("Batch not found".to_string())
}

pub async fn telegram_get_thumbnail(
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    message_id: i32,
) -> Result<String, String> {
    let data =
        thumbnail_cache::get_thumbnail(&state.telegram_session, chat_id, &chat_type, message_id)
            .await
            .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

pub async fn telegram_search_media(
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    query: String,
    media_type: Option<String>,
    limit: u32,
) -> Result<Vec<TelegramMediaItem>, String> {
    let fix_extensions = settings_helper::load_settings()
        .telegram
        .fix_file_extensions;
    api::search_media(
        &state.telegram_session,
        chat_id,
        &chat_type,
        &query,
        media_type.as_deref(),
        limit,
        fix_extensions,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn telegram_get_chat_photo(
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
) -> Result<String, String> {
    let data = thumbnail_cache::get_chat_photo(&state.telegram_session, chat_id, &chat_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

pub async fn telegram_clear_thumbnail_cache() -> Result<(), String> {
    thumbnail_cache::clear_cache().await;
    Ok(())
}

pub async fn telegram_upload_media(
    host: Option<Arc<dyn PluginHost>>,
    state: &TelegramPluginState,
    chat_id: i64,
    chat_type: String,
    file_path: String,
    caption: Option<String>,
) -> Result<TelegramDownloadStarted, String> {
    let upload_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let path_buf = std::path::PathBuf::from(&file_path);
    let file_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.bin")
        .to_string();

    let cancel_token = CancellationToken::new();
    {
        let mut active = state.active_generic_downloads.lock().await;
        active.insert(
            upload_id,
            (format!("tg-up:{}", upload_id), cancel_token.clone()),
        );
    }

    let session = state.telegram_session.clone();
    let active_downloads = state.active_generic_downloads.clone();
    let persistent = state.persistent.clone();
    let file_name_clone = file_name.clone();
    let caption_str = caption.unwrap_or_default();

    tokio::spawn(async move {
        let access_hash = {
            let guard = session.lock().await;
            guard.peer_hashes.get(&chat_id).copied().unwrap_or(0)
        };
        let peer = api::make_input_peer(chat_id, &chat_type, access_hash);

        let (tx, mut rx) = mpsc::channel::<(u64, u64)>(32);
        let host_progress = host.clone();
        let progress_forwarder = tokio::spawn(async move {
            while let Some((uploaded, total)) = rx.recv().await {
                let percent = if total > 0 {
                    (uploaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                emit_event(
                    &host_progress,
                    "generic-upload-progress",
                    &serde_json::json!({
                        "id": upload_id,
                        "percent": percent,
                        "uploaded_bytes": uploaded,
                        "total_bytes": total,
                    }),
                );
            }
        });

        let res =
            api::upload_file_to_peer(&session, peer, &path_buf, &caption_str, tx, &cancel_token)
                .await;

        let _ = progress_forwarder.await;
        active_downloads.lock().await.remove(&upload_id);

        match res {
            Ok(size) => {
                {
                    let mut p = persistent.lock().await;
                    p.bandwidth.record_bytes(size);
                    p.save();
                }
                emit_event(
                    &host,
                    "generic-upload-complete",
                    &serde_json::json!({
                        "id": upload_id,
                        "success": true,
                        "file_name": file_name_clone,
                        "file_size_bytes": size,
                    }),
                );
            }
            Err(e) => {
                emit_event(
                    &host,
                    "generic-upload-complete",
                    &serde_json::json!({
                        "id": upload_id,
                        "success": false,
                        "error": e.to_string(),
                    }),
                );
            }
        }
    });

    Ok(TelegramDownloadStarted {
        id: upload_id,
        file_name,
    })
}

pub async fn telegram_user_upload_file(
    host: Option<Arc<dyn PluginHost>>,
    state: &TelegramPluginState,
    file_path: String,
    destination: String,
    caption: String,
) -> Result<serde_json::Value, String> {
    let path_buf = std::path::PathBuf::from(&file_path);
    let peer = api::resolve_destination_peer(&state.telegram_session, &destination)
        .await
        .map_err(|e| e.to_string())?;

    let cancel_token = CancellationToken::new();
    {
        state
            .active_ops
            .lock()
            .await
            .insert("upload".to_string(), cancel_token.clone());
    }

    let (tx, mut rx) = mpsc::channel::<(u64, u64)>(32);
    let host_progress = host.clone();
    let progress_forwarder = tokio::spawn(async move {
        while let Some((uploaded, total)) = rx.recv().await {
            let percent = if total > 0 {
                (uploaded as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            emit_event(
                &host_progress,
                "telegram-upload-progress",
                &serde_json::json!({
                    "percent": percent,
                    "uploaded_bytes": uploaded,
                    "total_bytes": total,
                }),
            );
        }
    });

    let res = api::upload_file_to_peer(
        &state.telegram_session,
        peer,
        &path_buf,
        &caption,
        tx,
        &cancel_token,
    )
    .await;

    let _ = progress_forwarder.await;
    state.active_ops.lock().await.remove("upload");

    match res {
        Ok(size) => {
            {
                let mut p = state.persistent.lock().await;
                p.bandwidth.record_bytes(size);
                p.save();
            }
            Ok(serde_json::json!({
                "success": true,
                "message": "Uploaded via Telegram MTProto"
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

pub async fn telegram_bot_upload_file(
    host: Option<Arc<dyn PluginHost>>,
    file_path: String,
    bot_token: String,
    chat_id: String,
    caption: String,
) -> Result<serde_json::Value, String> {
    let path_buf = std::path::PathBuf::from(&file_path);
    let bytes = tokio::fs::read(&path_buf)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let total = bytes.len() as u64;
    let file_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.bin")
        .to_string();

    emit_event(
        &host,
        "telegram-upload-progress",
        &serde_json::json!({
            "percent": 15.0,
            "uploaded_bytes": 0,
            "total_bytes": total,
        }),
    );

    let part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
    let form = reqwest::multipart::Form::new()
        .text("chat_id", chat_id)
        .text("caption", caption)
        .part("document", part);

    let url = format!(
        "https://api.telegram.org/bot{}/sendDocument",
        bot_token.trim()
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Bot upload request failed: {}", e))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();

    emit_event(
        &host,
        "telegram-upload-progress",
        &serde_json::json!({
            "percent": 100.0,
            "uploaded_bytes": total,
            "total_bytes": total,
        }),
    );

    if status.is_success() && body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(serde_json::json!({
            "success": true,
            "message": "Uploaded via Bot API"
        }))
    } else {
        let desc = body
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Bot API upload failed");
        Ok(serde_json::json!({
            "success": false,
            "message": desc
        }))
    }
}

pub fn redact_phone(phone: &str) -> Option<String> {
    let trimmed = phone.trim();
    if trimmed.is_empty() {
        return None;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 4 {
        return Some(trimmed.to_string());
    }
    let prefix: String = chars.iter().take(3).collect();
    let suffix: String = chars[chars.len() - 2..].iter().collect();
    Some(format!("{}***{}", prefix, suffix))
}

#[allow(clippy::too_many_arguments)]
pub async fn telegram_clone_start(
    host: Option<Arc<dyn PluginHost>>,
    state: &TelegramPluginState,
    source_id: i64,
    source_type: String,
    source_title: Option<String>,
    dest_id: Option<i64>,
    dest_type: Option<String>,
    dest_title: Option<String>,
    options: crate::state::TelegramCloneOptions,
) -> Result<serde_json::Value, String> {
    let src_title = source_title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Chat {}", source_id));

    let (resolved_dest_id, resolved_dest_type, resolved_dest_title) = match dest_id {
        Some(did) if did != 0 => (
            did,
            dest_type.unwrap_or_else(|| "channel".to_string()),
            dest_title.unwrap_or_else(|| format!("Chat {}", did)),
        ),
        _ => {
            let new_title = dest_title
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("{} (Clone)", src_title));
            let created = api::create_channel_or_folder(
                &state.telegram_session,
                &new_title,
                "Cloned with LibreGet",
                false,
            )
            .await
            .map_err(|e| e.to_string())?;
            (created.id, created.chat_type, created.title)
        }
    };

    let now = crate::state::unix_now_secs();
    let session_id = format!(
        "clone_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let clone_session = crate::state::TelegramCloneSession {
        id: session_id.clone(),
        source_chat_id: source_id,
        source_chat_type: source_type.clone(),
        source_title: src_title,
        dest_chat_id: resolved_dest_id,
        dest_chat_type: resolved_dest_type.clone(),
        dest_title: resolved_dest_title.clone(),
        last_message_id: 0,
        total_collected: 0,
        cloned_count: 0,
        failed_count: 0,
        status: "running".to_string(),
        last_error: None,
        options: options.clone(),
        created_at: now,
        updated_at: now,
    };

    {
        let mut p = state.persistent.lock().await;
        p.clones.push(clone_session);
        p.save();
    }

    let cancel_token = CancellationToken::new();
    {
        state
            .active_ops
            .lock()
            .await
            .insert(session_id.clone(), cancel_token.clone());
    }

    let tg_session = state.telegram_session.clone();
    let persistent = state.persistent.clone();
    let active_ops = state.active_ops.clone();
    let sid = session_id.clone();
    let resolved_dest_type_for_task = resolved_dest_type.clone();

    tokio::spawn(async move {
        let max_items = options.limit.unwrap_or(500).clamp(1, 5000) as u32;
        let media_items = api::list_media(
            &tg_session,
            source_id,
            &source_type,
            None,
            0,
            max_items,
            true,
        )
        .await;

        let mut msg_ids: Vec<i32> = match media_items {
            Ok(items) => items.into_iter().map(|i| i.message_id).collect(),
            Err(e) => {
                let mut p = persistent.lock().await;
                if let Some(c) = p.clones.iter_mut().find(|c| c.id == sid) {
                    c.status = "failed".to_string();
                    c.last_error = Some(e.to_string());
                    c.updated_at = crate::state::unix_now_secs();
                }
                p.save();
                active_ops.lock().await.remove(&sid);
                return;
            }
        };

        msg_ids.sort_unstable();
        msg_ids.dedup();
        let total = msg_ids.len();

        {
            let mut p = persistent.lock().await;
            if let Some(c) = p.clones.iter_mut().find(|c| c.id == sid) {
                c.total_collected = total;
                c.updated_at = crate::state::unix_now_secs();
            }
            p.save();
        }

        emit_event(
            &host,
            "telegram:clone:progress",
            &serde_json::json!({
                "session_id": sid,
                "phase": "cloning",
                "stage": "cloning",
                "cloned": 0,
                "current": 0,
                "failed": 0,
                "total": total,
                "last_message_id": 0,
            }),
        );

        let batch_size = options.batch_size.clamp(1, 100);
        let delay_ms = options.delay_ms.clamp(200, 60_000);
        let mut cloned = 0usize;
        let mut failed = 0usize;
        let mut last_id = 0i32;

        for chunk in msg_ids.chunks(batch_size) {
            if cancel_token.is_cancelled() {
                break;
            }
            last_id = *chunk.last().unwrap_or(&last_id);
            match api::forward_messages(
                &tg_session,
                source_id,
                &source_type,
                resolved_dest_id,
                &resolved_dest_type_for_task,
                chunk.to_vec(),
                options.drop_author,
                options.drop_captions,
            )
            .await
            {
                Ok(()) => {
                    cloned += chunk.len();
                }
                Err(e) => {
                    failed += chunk.len();
                    let mut p = persistent.lock().await;
                    if let Some(c) = p.clones.iter_mut().find(|c| c.id == sid) {
                        c.last_error = Some(e.to_string());
                    }
                }
            }

            {
                let mut p = persistent.lock().await;
                if let Some(c) = p.clones.iter_mut().find(|c| c.id == sid) {
                    c.cloned_count = cloned;
                    c.failed_count = failed;
                    c.last_message_id = last_id;
                    c.updated_at = crate::state::unix_now_secs();
                }
                p.save();
            }

            emit_event(
                &host,
                "telegram:clone:progress",
                &serde_json::json!({
                    "session_id": sid,
                    "phase": "cloning",
                    "stage": "cloning",
                    "cloned": cloned,
                    "current": cloned,
                    "failed": failed,
                    "total": total,
                    "last_message_id": last_id,
                }),
            );

            tokio::select! {
                _ = tokio::time::sleep( std::time::Duration::from_millis(delay_ms) ) => {},
                _ = cancel_token.cancelled() => break,
            }
        }

        {
            let mut p = persistent.lock().await;
            if let Some(c) = p.clones.iter_mut().find(|c| c.id == sid) {
                if c.status == "running" {
                    c.status = if cancel_token.is_cancelled() {
                        "paused".to_string()
                    } else {
                        "completed".to_string()
                    };
                }
                c.updated_at = crate::state::unix_now_secs();
            }
            p.save();
        }

        let final_stage = if cancel_token.is_cancelled() {
            "paused"
        } else {
            "completed"
        };
        active_ops.lock().await.remove(&sid);
        emit_event(
            &host,
            "telegram:clone:progress",
            &serde_json::json!({
                "session_id": sid,
                "phase": "done",
                "stage": final_stage,
                "cloned": cloned,
                "current": cloned,
                "failed": failed,
                "total": total,
                "last_message_id": last_id,
            }),
        );
    });

    Ok(serde_json::json!({
        "session_id": session_id,
        "dest_chat_id": resolved_dest_id,
        "dest_chat_type": resolved_dest_type,
        "dest_title": resolved_dest_title,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sanitize_folder_name_forbidden_characters() {
        assert_eq!(
            sanitize_folder_name("My:Channel/Name?*"),
            "My_Channel_Name__"
        );
        assert_eq!(sanitize_folder_name("<Test>|Pipe\""), "_Test__Pipe_");
    }

    #[test]
    fn test_sanitize_folder_name_reserved_windows_names() {
        assert_eq!(sanitize_folder_name("CON"), "CON_channel");
        assert_eq!(sanitize_folder_name("prn"), "prn_channel");
        assert_eq!(sanitize_folder_name("aux"), "aux_channel");
        assert_eq!(sanitize_folder_name("NUL"), "NUL_channel");
        assert_eq!(sanitize_folder_name("COM1"), "COM1_channel");
        assert_eq!(sanitize_folder_name("lpt3"), "lpt3_channel");
    }

    #[test]
    fn test_sanitize_folder_name_empty_or_whitespace() {
        assert_eq!(sanitize_folder_name(""), "Telegram Media");
        assert_eq!(sanitize_folder_name("   "), "Telegram Media");
        assert_eq!(sanitize_folder_name("..."), "Telegram Media");
    }

    #[test]
    fn test_resolve_telegram_download_dir_creates_libregram_and_channel_folder() {
        let base = "C:/Users/mint/Downloads";
        let dir = resolve_telegram_download_dir(base, Some("Evelina Ava"));
        assert_eq!(
            dir,
            PathBuf::from("C:/Users/mint/Downloads")
                .join("LibreGram")
                .join("Evelina Ava")
        );

        let dir2 = resolve_telegram_download_dir(base, Some("Ivana"));
        assert_eq!(
            dir2,
            PathBuf::from("C:/Users/mint/Downloads")
                .join("LibreGram")
                .join("Ivana")
        );
    }

    #[test]
    fn test_resolve_telegram_download_dir_when_base_already_libregram() {
        let base = "C:/Users/mint/Downloads/LibreGram";
        let dir = resolve_telegram_download_dir(base, Some("Evelina Ava"));
        assert_eq!(
            dir,
            PathBuf::from("C:/Users/mint/Downloads/LibreGram").join("Evelina Ava")
        );
    }

    #[test]
    fn test_resolve_telegram_download_dir_with_none_or_empty_title() {
        let base = "C:/Users/mint/Downloads";
        let dir = resolve_telegram_download_dir(base, None);
        assert_eq!(
            dir,
            PathBuf::from("C:/Users/mint/Downloads").join("LibreGram")
        );

        let dir2 = resolve_telegram_download_dir(base, Some("   "));
        assert_eq!(
            dir2,
            PathBuf::from("C:/Users/mint/Downloads").join("LibreGram")
        );
    }
}
