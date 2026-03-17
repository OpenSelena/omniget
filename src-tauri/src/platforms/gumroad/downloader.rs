use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::core::filename;

use super::api::{self, GumroadProduct, GumroadSession};

#[derive(Debug, Clone, serde::Serialize)]
pub struct GumroadDownloadProgress {
    pub product_id: String,
    pub product_name: String,
    pub percent: f64,
    pub current_file: String,
    pub downloaded_bytes: u64,
    pub total_files: u32,
    pub completed_files: u32,
}

pub async fn download_product(
    app: &tauri::AppHandle,
    session: &GumroadSession,
    product: &GumroadProduct,
    product_raw: &serde_json::Value,
    output_dir: &str,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let files = api::get_product_files(session, product_raw).await;

    if files.is_empty() {
        return Err(anyhow!(
            "'{}' has no files available for download",
            product.name
        ));
    }

    let product_dir = format!(
        "{}/{}",
        output_dir,
        filename::sanitize_path_component(&product.name)
    );
    tokio::fs::create_dir_all(&product_dir).await?;

    let total_files = files.len();
    let total_bytes = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let _ = app.emit(
        "gumroad-download-progress",
        &GumroadDownloadProgress {
            product_id: product.id.clone(),
            product_name: product.name.clone(),
            percent: 0.0,
            current_file: "Starting...".to_string(),
            downloaded_bytes: 0,
            total_files: total_files as u32,
            completed_files: 0,
        },
    );

    for (_i, file) in files.iter().enumerate() {
        if cancel_token.is_cancelled() {
            return Err(anyhow!("Download cancelled by user"));
        }

        let file_name = filename::sanitize_path_component(&file.name);
        let dest_path = format!("{}/{}", product_dir, file_name);

        if tokio::fs::try_exists(&dest_path).await.unwrap_or(false) {
            let meta = tokio::fs::metadata(&dest_path).await;
            if meta.map(|m| m.len() > 0).unwrap_or(false) {
                tracing::info!("[gumroad] Skipping existing: {}", dest_path);
                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = app.emit(
                    "gumroad-download-progress",
                    &GumroadDownloadProgress {
                        product_id: product.id.clone(),
                        product_name: product.name.clone(),
                        percent: done as f64 / total_files as f64 * 100.0,
                        current_file: file.name.clone(),
                        downloaded_bytes: total_bytes.load(Ordering::Relaxed),
                        total_files: total_files as u32,
                        completed_files: done as u32,
                    },
                );
                continue;
            }
        }

        if api::is_media_file(file) {
            match api::get_stream_url(session, &product.url_redirect_token, &file.id).await {
                Ok(playlist_url) => {
                    match download_with_ytdlp(
                        &playlist_url,
                        &product_dir,
                        &session.token,
                        &cancel_token,
                    )
                    .await
                    {
                        Ok(size) => {
                            total_bytes.fetch_add(size, Ordering::Relaxed);
                        }
                        Err(e) => {
                            tracing::error!(
                                "[gumroad] yt-dlp download failed for '{}': {}",
                                file.name,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "[gumroad] Failed to get stream URL for '{}': {}, trying direct download",
                        file.name,
                        e
                    );
                    let download_url = file
                        .download_url
                        .clone()
                        .unwrap_or_else(|| api::get_direct_download_url(&product.url_redirect_token, &file.id));
                    match download_file_direct(&session.client, &download_url, &dest_path, &cancel_token).await {
                        Ok(size) => {
                            total_bytes.fetch_add(size, Ordering::Relaxed);
                        }
                        Err(e2) => {
                            tracing::error!(
                                "[gumroad] Direct download also failed for '{}': {}",
                                file.name,
                                e2
                            );
                        }
                    }
                }
            }
        } else {
            let download_url = file
                .download_url
                .clone()
                .unwrap_or_else(|| api::get_direct_download_url(&product.url_redirect_token, &file.id));
            match download_file_direct(&session.client, &download_url, &dest_path, &cancel_token).await {
                Ok(size) => {
                    total_bytes.fetch_add(size, Ordering::Relaxed);
                }
                Err(e) => {
                    tracing::error!(
                        "[gumroad] File download failed for '{}': {}",
                        file.name,
                        e
                    );
                }
            }
        }

        let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = app.emit(
            "gumroad-download-progress",
            &GumroadDownloadProgress {
                product_id: product.id.clone(),
                product_name: product.name.clone(),
                percent: done as f64 / total_files as f64 * 100.0,
                current_file: file.name.clone(),
                downloaded_bytes: total_bytes.load(Ordering::Relaxed),
                total_files: total_files as u32,
                completed_files: done as u32,
            },
        );
    }

    if cancel_token.is_cancelled() {
        return Err(anyhow!("Download cancelled by user"));
    }

    Ok(())
}

async fn download_with_ytdlp(
    url: &str,
    output_dir: &str,
    token: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let ytdlp_path = crate::core::ytdlp::ensure_ytdlp().await?;
    let out_dir = Path::new(output_dir);
    let (tx, _rx) = mpsc::channel(8);

    let auth_header = format!("Authorization: Bearer {}", token);
    let extra_args: Vec<String> = vec![
        "--add-header".to_string(),
        auth_header,
        "--add-header".to_string(),
        "User-Agent: okhttp/4.8.1".to_string(),
    ];

    let result = crate::core::ytdlp::download_video(
        &ytdlp_path,
        url,
        out_dir,
        None,
        tx,
        None,
        None,
        None,
        Some("https://gumroad.com/"),
        cancel_token.clone(),
        None,
        8,
        false,
        &extra_args,
    )
    .await?;

    Ok(result.file_size_bytes)
}

async fn download_file_direct(
    client: &reqwest::Client,
    url: &str,
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let out = Path::new(output_path);
    let (tx, _rx) = mpsc::channel(8);

    crate::core::direct_downloader::download_direct(
        client,
        url,
        out,
        tx,
        Some(cancel_token),
    )
    .await
}
