use std::collections::HashSet;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::ytdlp;
use crate::models::media::{
    DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality as MediaVideoQuality,
};
use crate::platforms::traits::PlatformDownloader;

pub struct GenericYtdlpDownloader;

impl Default for GenericYtdlpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericYtdlpDownloader {
    pub fn new() -> Self {
        Self
    }

    fn extract_quality_height(quality_str: &str) -> Option<u32> {
        let s = quality_str.trim().to_lowercase();
        if s == "best" || s == "highest" {
            return None;
        }
        s.trim_end_matches('p').parse::<u32>().ok()
    }

    fn detect_platform(json: &serde_json::Value) -> String {
        json.get("extractor_key")
            .or_else(|| json.get("extractor"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "generic".to_string())
    }

    fn detect_media_type(json: &serde_json::Value) -> MediaType {
        let has_video = json
            .get("formats")
            .and_then(|v| v.as_array())
            .map(|formats| {
                formats.iter().any(|f| {
                    f.get("vcodec")
                        .and_then(|v| v.as_str())
                        .map(|v| v != "none")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if has_video {
            MediaType::Video
        } else {
            MediaType::Audio
        }
    }

    pub fn parse_video_info(json: &serde_json::Value) -> anyhow::Result<MediaInfo> {
        let title = json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let author = json
            .get("uploader")
            .or_else(|| json.get("channel"))
            .or_else(|| json.get("uploader_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let duration = json.get("duration").and_then(|v| v.as_f64());

        let thumbnail = json
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let webpage_url = json
            .get("webpage_url")
            .or_else(|| json.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let platform = Self::detect_platform(json);
        let media_type = Self::detect_media_type(json);

        let mut qualities: Vec<MediaVideoQuality> = Vec::new();
        let mut seen_heights: HashSet<u32> = HashSet::new();

        if media_type == MediaType::Video {
            if let Some(formats) = json.get("formats").and_then(|v| v.as_array()) {
                for f in formats {
                    let height = f.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let width = f.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let vcodec = f
                        .get("vcodec")
                        .and_then(|v| v.as_str())
                        .unwrap_or("none");

                    if vcodec == "none" || height == 0 {
                        continue;
                    }

                    if seen_heights.insert(height) {
                        qualities.push(MediaVideoQuality {
                            label: format!("{}p", height),
                            width,
                            height,
                            url: webpage_url.clone(),
                            format: "ytdlp".to_string(),
                        });
                    }
                }
            }

            qualities.sort_by(|a, b| b.height.cmp(&a.height));
        }

        if qualities.is_empty() {
            qualities.push(MediaVideoQuality {
                label: "best".to_string(),
                width: 0,
                height: 0,
                url: webpage_url,
                format: "ytdlp".to_string(),
            });
        }

        Ok(MediaInfo {
            title,
            author,
            platform,
            duration_seconds: duration,
            thumbnail_url: thumbnail,
            available_qualities: qualities,
            media_type,
            file_size_bytes: None,
        })
    }
}

#[async_trait]
impl PlatformDownloader for GenericYtdlpDownloader {
    fn name(&self) -> &str {
        "generic"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            let scheme = parsed.scheme();
            return scheme == "http" || scheme == "https";
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let ytdlp_path = ytdlp::ensure_ytdlp().await.map_err(|e| {
            anyhow!("yt-dlp unavailable: {}", e)
        })?;

        let extra = platform_extra_flags(url);
        let json = ytdlp::get_video_info(&ytdlp_path, url, &extra).await?;
        Self::parse_video_info(&json)
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<f64>,
    ) -> anyhow::Result<DownloadResult> {
        let _ = progress.send(0.0).await;

        let ytdlp_path = if let Some(ref p) = opts.ytdlp_path {
            p.clone()
        } else {
            ytdlp::ensure_ytdlp().await?
        };

        let first = info
            .available_qualities
            .first()
            .ok_or_else(|| anyhow!("No quality available"))?;

        let selected = if let Some(ref wanted) = opts.quality {
            info.available_qualities
                .iter()
                .find(|q| q.label == *wanted)
                .unwrap_or(first)
        } else {
            first
        };

        let quality_height = Self::extract_quality_height(&selected.label);
        let video_url = &selected.url;

        let referer = opts.referer.as_deref()
            .or_else(|| platform_referer(video_url));

        ytdlp::download_video(
            &ytdlp_path,
            video_url,
            &opts.output_dir,
            quality_height,
            progress,
            opts.download_mode.as_deref(),
            opts.format_id.as_deref(),
            opts.filename_template.as_deref(),
            referer,
            opts.cancel_token.clone(),
            None,
            opts.concurrent_fragments,
            opts.download_subtitles,
            &[],
        )
        .await
    }
}

fn platform_extra_flags(url: &str) -> Vec<String> {
    match platform_referer(url) {
        Some(r) => vec!["--referer".into(), r.to_string()],
        None => Vec::new(),
    }
}

fn platform_referer(url: &str) -> Option<&'static str> {
    let lower = url.to_lowercase();

    if lower.contains("douyin.com") || lower.contains("iesdouyin.com") {
        return Some("https://www.douyin.com/");
    }
    if lower.contains("v.qq.com") || lower.contains("qq.com/x/") {
        return Some("https://v.qq.com/");
    }
    if lower.contains("youku.com") {
        return Some("https://www.youku.com/");
    }
    if lower.contains("iqiyi.com") {
        return Some("https://www.iqiyi.com/");
    }
    if lower.contains("mgtv.com") {
        return Some("https://www.mgtv.com/");
    }
    if lower.contains("kuaishou.com") {
        return Some("https://www.kuaishou.com/");
    }
    if lower.contains("xiaohongshu.com") || lower.contains("xhslink.com") {
        return Some("https://www.xiaohongshu.com/");
    }
    if lower.contains("bilibili.com") || lower.contains("bilibili.tv") {
        return Some("https://www.bilibili.com/");
    }

    None
}
