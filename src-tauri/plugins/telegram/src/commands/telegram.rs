use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use base64::Engine;
use omniget_plugin_sdk::PluginHost;

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
    let is_already_omnigram = base
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case("OmniGram"))
        .unwrap_or(false);

    let omnigram_root = if is_already_omnigram {
        base
    } else {
        base.join("OmniGram")
    };

    match chat_title {
        Some(title) if !title.trim().is_empty() => {
            let clean = sanitize_folder_name(title);
            omnigram_root.join(clean)
        }
        _ => omnigram_root,
    }
}

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

        let (tx, mut rx) = mpsc::channel::<omniget_core::models::progress::ProgressUpdate>(32);

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
    let total_files = items.len() as u32;

    // Read Telegram settings
    let tg_settings = settings_helper::load_settings().telegram;
    let concurrent = tg_settings.concurrent_downloads.clamp(1, 10) as usize;

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
                    mpsc::channel::<omniget_core::models::progress::ProgressUpdate>(32);

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
                    Ok(_) => {
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
