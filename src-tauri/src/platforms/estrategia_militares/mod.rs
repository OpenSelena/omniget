pub mod api;
pub mod downloader;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

pub struct EstrategiaMilitaresDownloader;

impl Default for EstrategiaMilitaresDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl EstrategiaMilitaresDownloader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformDownloader for EstrategiaMilitaresDownloader {
    fn name(&self) -> &str {
        "estrategia_militares"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "militares.estrategia.com";
            }
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Estratégia Militares".to_string(),
            author: String::new(),
            platform: "estrategia_militares".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![VideoQuality {
                label: "course".to_string(),
                width: 0,
                height: 0,
                url: url.to_string(),
                format: "course".to_string(),
            }],
            media_type: MediaType::Course,
            file_size_bytes: None,
        })
    }

    async fn download(
        &self,
        _info: &MediaInfo,
        _opts: &DownloadOptions,
        _progress: mpsc::Sender<f64>,
    ) -> anyhow::Result<DownloadResult> {
        Err(anyhow!("Use the courses interface to download Estratégia Militares courses"))
    }
}
