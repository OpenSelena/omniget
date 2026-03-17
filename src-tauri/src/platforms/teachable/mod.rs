pub mod api;
pub mod downloader;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

pub struct TeachableDownloader;

impl Default for TeachableDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl TeachableDownloader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformDownloader for TeachableDownloader {
    fn name(&self) -> &str {
        "teachable"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "teachable.com"
                    || host.ends_with(".teachable.com")
                    || host == "sso.teachable.com";
            }
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Teachable Course".to_string(),
            author: String::new(),
            platform: "teachable".to_string(),
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
        Err(anyhow!("Use the courses interface to download Teachable courses"))
    }
}
