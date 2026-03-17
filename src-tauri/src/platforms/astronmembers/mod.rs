use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType};
use crate::platforms::traits::PlatformDownloader;

pub struct AstronMembersDownloader;

impl AstronMembersDownloader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AstronMembersDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDownloader for AstronMembersDownloader {
    fn name(&self) -> &str {
        "astronmembers"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "astronmembers.com"
                    || host.ends_with(".astronmembers.com");
            }
        }
        false
    }

    async fn get_media_info(&self, _url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Astron Members".to_string(),
            author: String::new(),
            platform: "astronmembers".to_string(),
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
        Err(anyhow!("Use the courses interface to download from Astron Members"))
    }
}
