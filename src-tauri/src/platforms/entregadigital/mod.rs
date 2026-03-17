pub mod api;
pub mod downloader;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType};
use crate::platforms::traits::PlatformDownloader;

pub struct EntregaDigitalDownloader;

impl EntregaDigitalDownloader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EntregaDigitalDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDownloader for EntregaDigitalDownloader {
    fn name(&self) -> &str {
        "entregadigital"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "entregadigital.app.br"
                    || host.ends_with(".entregadigital.app.br");
            }
        }
        false
    }

    async fn get_media_info(&self, _url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Entrega Digital".to_string(),
            author: String::new(),
            platform: "entregadigital".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![],
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
        Err(anyhow!("Use the courses interface to download from Entrega Digital"))
    }
}
