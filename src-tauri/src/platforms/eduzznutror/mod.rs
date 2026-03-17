use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType};
use crate::platforms::traits::PlatformDownloader;

pub struct EduzzNutrorDownloader;

impl EduzzNutrorDownloader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EduzzNutrorDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDownloader for EduzzNutrorDownloader {
    fn name(&self) -> &str {
        "eduzznutror"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "nutror.com"
                    || host.ends_with(".nutror.com")
                    || host == "eduzz.com"
                    || host.ends_with(".eduzz.com");
            }
        }
        false
    }

    async fn get_media_info(&self, _url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Eduzz Nutror".to_string(),
            author: String::new(),
            platform: "eduzznutror".to_string(),
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
        Err(anyhow!("Use the courses interface to download from Eduzz Nutror"))
    }
}
