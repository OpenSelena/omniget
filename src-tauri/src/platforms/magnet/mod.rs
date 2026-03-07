use async_trait::async_trait;
use tokio::sync::mpsc;
use librqbit::{Session, SessionOptions, AddTorrent};

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

pub struct MagnetDownloader;

impl MagnetDownloader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MagnetDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDownloader for MagnetDownloader {
    fn name(&self) -> &str {
        "magnet"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.starts_with("magnet:") || url.ends_with(".torrent")
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let mut title = "Torrent Download".to_string();

        if url.starts_with("magnet:") {
            if let Ok(parsed_url) = url::Url::parse(url) {
                for (key, value) in parsed_url.query_pairs() {
                    if key == "dn" {
                        title = value.to_string();
                        break;
                    }
                }
            }
        }

        Ok(MediaInfo {
            title,
            author: "BitTorrent".to_string(),
            platform: "magnet".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![VideoQuality {
                label: "Original".to_string(),
                width: 0,
                height: 0,
                url: url.to_string(),
                format: "torrent".to_string(),
            }],
            media_type: MediaType::Video,
            file_size_bytes: None,
        })
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<f64>,
    ) -> anyhow::Result<DownloadResult> {
        let _ = progress.send(0.0).await;

        let url = match info.available_qualities.first() {
            Some(q) => &q.url,
            None => anyhow::bail!("No URL found in MediaInfo"),
        };

        let output_dir = &opts.output_dir;

        let listen_port = opts.torrent_listen_port.unwrap_or(6881).min(65525);
        tracing::info!("[magnet] initializing session, output: {}, port: {}", output_dir.display(), listen_port);
        let session_opts = SessionOptions {
            listen_port_range: Some(listen_port..listen_port.saturating_add(10)),
            ..Default::default()
        };
        let session = match Session::new_with_opts(output_dir.into(), session_opts).await {
            Ok(s) => s,
            Err(e) => anyhow::bail!("Failed to initialize torrent session: {}", e),
        };

        let add_torrent = AddTorrent::from_url(url);

        tracing::info!("[magnet] adding torrent...");
        let managed_torrent = match session.add_torrent(add_torrent, None).await {
            Ok(handle) => match handle.into_handle() {
                Some(h) => h,
                None => anyhow::bail!("Failed to get torrent handle: torrent might have been added in a paused state or failed initialization"),
            },
            Err(e) => anyhow::bail!("Failed to add torrent: {}", e),
        };

        tracing::info!("[magnet] torrent added, waiting for download...");

        // Pin the completion future so it persists across select! iterations
        let completion = managed_torrent.wait_until_completed();
        tokio::pin!(completion);

        let cancel_rx = opts.cancel_token.clone();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    let stats = managed_torrent.stats();

                    let total = stats.total_bytes;
                    let downloaded = stats.progress_bytes;

                    if total > 0 {
                        let pct = (downloaded as f64 / total as f64) * 100.0;
                        let _ = progress.send(pct).await;
                        tracing::debug!(
                            "[magnet] progress: {:.1}% ({:.1} MB / {:.1} MB)",
                            pct,
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                        );
                    }
                }
                _ = cancel_rx.cancelled() => {
                    tracing::info!("[magnet] download cancelled by user");
                    anyhow::bail!("Download cancelled");
                }
                res = &mut completion => {
                    if let Err(e) = res {
                        anyhow::bail!("Torrent download failed: {}", e);
                    }
                    let _ = progress.send(100.0).await;
                    tracing::info!("[magnet] download complete");
                    break;
                }
            }
        }

        let (total_size, torrent_name) = managed_torrent.with_metadata(|meta| {
            let size = meta.info.iter_file_lengths().ok().map(|iter| iter.sum::<u64>())
                .unwrap_or_else(|| meta.file_infos.iter().map(|f| f.len).sum());
            let name = meta.info.name.as_ref()
                .map(|n| String::from_utf8_lossy(n.as_ref()).to_string())
                .unwrap_or_default();
            (size, name)
        }).unwrap_or((0, String::new()));

        let file_path = if torrent_name.is_empty() {
            output_dir.clone()
        } else {
            output_dir.join(sanitize_filename::sanitize(&torrent_name))
        };

        Ok(DownloadResult {
            file_path,
            file_size_bytes: total_size,
            duration_seconds: 0.0,
        })
    }
}
