pub mod api;
pub mod downloader;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use omniget_core::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

pub struct GrancursosDownloader;

impl Default for GrancursosDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GrancursosDownloader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformDownloader for GrancursosDownloader {
    fn name(&self) -> &str {
        "grancursos"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "grancursosonline.com.br"
                    || host == "www.grancursosonline.com.br";
            }
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        Ok(MediaInfo {
            title: "Gran Cursos".to_string(),
            author: String::new(),
            platform: "grancursos".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![VideoQuality {
                label: "Original".to_string(),
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
        Err(anyhow!("Use the courses interface to download from Gran Cursos"))
    }
}
