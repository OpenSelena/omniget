use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use librqbit::{Session, AddTorrent};

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
        // Return a generic placeholder for the torrent since fetching the full
        // metadata might take some time or require connection to DHT/Peers.
        // We will just show "Torrent file" or derived name from the magnet link if possible.
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
            media_type: MediaType::Video, // fallback to Video since File variant doesn't exist
            file_size_bytes: None, // Cannot easily know upfront for magnet links
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
        
        // Initialize the rqbit session pointed to the output directory
        let session = match Session::new(output_dir.into()).await {
            Ok(s) => s,
            Err(e) => anyhow::bail!("Failed to initialize torrent session: {}", e),
        };

        // Add the torrent from the URL (can be a magnet link or HTTP URL to a .torrent)
        let add_torrent = AddTorrent::from_url(url);
        
        let managed_torrent = match session.add_torrent(add_torrent, None).await {
            Ok(handle) => match handle.into_handle() {
                Some(h) => h,
                None => anyhow::bail!("Failed to get torrent handle: torrent might have been added in a paused state or failed initialization"),
            },
            Err(e) => anyhow::bail!("Failed to add torrent: {}", e),
        };

        // Poll for progress updates
        let mut cancel_rx = opts.cancel_token.clone();
        
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    let stats = managed_torrent.stats();
                    
                    let total = stats.total_bytes;
                    let downloaded = stats.progress_bytes;
                    
                    if total > 0 {
                        let pct = (downloaded as f64 / total as f64) * 100.0;
                        let _ = progress.send(pct).await;
                    }

                    match stats.state {
                        librqbit::TorrentStatsState::Error => anyhow::bail!("Torrent download entered error state"),
                        _ => {}
                    }
                }
                _ = cancel_rx.cancelled() => {
                    let _ = session.pause(&managed_torrent).await;
                    anyhow::bail!("Download cancelled");
                }
                res = managed_torrent.wait_until_completed() => {
                    if let Err(e) = res {
                        anyhow::bail!("Torrent download failed: {}", e);
                    }
                    let _ = progress.send(100.0).await;
                    break;
                }
            }
        }

        let total_size = managed_torrent.with_metadata(|meta| {
            meta.info.iter_file_lengths().ok().map(|iter| iter.sum::<u64>())
                .unwrap_or_else(|| meta.file_infos.iter().map(|f| f.len).sum())
        }).unwrap_or(0);

        Ok(DownloadResult {
            file_path: output_dir.clone(), // Best we can do is point to output dir
            file_size_bytes: total_size,
            duration_seconds: 0.0,
        })
    }
}
