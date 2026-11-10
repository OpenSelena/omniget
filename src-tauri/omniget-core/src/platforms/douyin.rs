use std::time::Duration;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use tokio::sync::{mpsc, OnceCell};

use crate::core::{direct_downloader, http_client};
use crate::models::media::{
    DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality as MediaVideoQuality,
};
use crate::models::progress::ProgressUpdate;
use crate::platforms::traits::PlatformDownloader;

const DOUYIN_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36";
const MOBILE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
const TTWID_REGISTER_URL: &str = "https://ttwid.bytedance.com/ttwid/union/register/";
const TTWID_REGISTER_BODY: &str = r#"{"region":"cn","aid":1768,"needFid":false,"service":"www.ixigua.com","migrate_info":{"ticket":"","source":"node"},"cbUrlProtocol":"https","union":true}"#;
const MAX_TITLE_CHARS: usize = 80;

static TTWID: OnceCell<String> = OnceCell::const_new();

#[derive(Debug, Clone, PartialEq)]
pub struct ShareVideo {
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub duration_seconds: Option<f64>,
    pub thumbnail_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub play_url: String,
}

pub struct DouyinDownloader;

impl Default for DouyinDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl DouyinDownloader {
    pub fn new() -> Self {
        Self
    }

    fn host_matches(host: &str) -> bool {
        let host = host.to_lowercase();
        host == "douyin.com"
            || host.ends_with(".douyin.com")
            || host == "iesdouyin.com"
            || host.ends_with(".iesdouyin.com")
            || host == "amemv.com"
            || host.ends_with(".amemv.com")
    }

    fn is_short_link(url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host.to_lowercase();
                return host == "v.douyin.com"
                    || host == "amemv.com"
                    || host.ends_with(".amemv.com")
                    || host == "iesdouyin.com"
                    || host.ends_with(".iesdouyin.com");
            }
        }
        false
    }

    fn canonicalize(url: &str) -> String {
        let Ok(parsed) = url::Url::parse(url) else {
            return url.to_string();
        };
        let host = parsed.host_str().unwrap_or("").to_lowercase();
        if !Self::host_matches(&host) {
            return url.to_string();
        }

        let segments: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
        let target = match segments.as_slice() {
            ["share", kind @ ("video" | "note" | "slides"), id, ..] => Some((*kind, *id)),
            [kind @ ("video" | "note"), id, ..] => Some((*kind, *id)),
            _ => None,
        };
        if let Some((kind, id)) = target {
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                let kind = if kind == "slides" { "note" } else { kind };
                return format!("https://www.douyin.com/{}/{}", kind, id);
            }
        }

        if let Some(id) = parsed
            .query_pairs()
            .find(|(k, _)| k == "modal_id")
            .map(|(_, v)| v.to_string())
        {
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                return format!("https://www.douyin.com/video/{}", id);
            }
        }

        url.to_string()
    }

    async fn resolve_url(url: &str) -> String {
        if !Self::is_short_link(url) {
            return Self::canonicalize(url);
        }
        let builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(DOUYIN_UA);
        let client = match http_client::apply_global_proxy(builder).build() {
            Ok(c) => c,
            Err(_) => return Self::canonicalize(url),
        };
        match client.get(url).send().await {
            Ok(resp) => Self::canonicalize(resp.url().as_str()),
            Err(e) => {
                tracing::warn!("[douyin] short-link resolve failed: {}", e);
                Self::canonicalize(url)
            }
        }
    }

    pub fn extract_video_id(url: &str) -> anyhow::Result<String> {
        let parsed = url::Url::parse(url).context("invalid Douyin URL")?;
        if let Some(id) = parsed
            .query_pairs()
            .find(|(k, _)| k == "modal_id")
            .map(|(_, v)| v.to_string())
        {
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                return Ok(id);
            }
        }
        let segments: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
        match segments.as_slice() {
            ["share", "video" | "note" | "slides", id, ..]
            | ["video" | "note" | "slides", id, ..]
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) =>
            {
                return Ok((*id).to_string());
            }
            _ => {}
        }
        for seg in segments {
            if seg.len() >= 15 && seg.chars().all(|c| c.is_ascii_digit()) {
                return Ok(seg.to_string());
            }
        }
        Err(anyhow!("Could not find Douyin video ID in URL: {}", url))
    }

    fn client() -> anyhow::Result<reqwest::Client> {
        let builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(MOBILE_UA);
        Ok(http_client::apply_global_proxy(builder).build()?)
    }

    async fn get_ttwid(client: &reqwest::Client) -> anyhow::Result<&'static str> {
        let value = TTWID
            .get_or_try_init(|| async {
                let resp = client
                    .post(TTWID_REGISTER_URL)
                    .header("Content-Type", "application/json")
                    .body(TTWID_REGISTER_BODY)
                    .send()
                    .await?
                    .error_for_status()?;

                for val in resp.headers().get_all(reqwest::header::SET_COOKIE) {
                    if let Ok(s) = val.to_str() {
                        if let Some(pos) = s.find("ttwid=") {
                            let after = &s[pos + 6..];
                            let token = after.split(';').next().unwrap_or("").trim();
                            if !token.is_empty() {
                                return Ok(token.to_string());
                            }
                        }
                    }
                }
                Err(anyhow!("Douyin ttwid registration returned no cookie"))
            })
            .await?;
        Ok(value.as_str())
    }

    async fn fetch_share_video(video_id: &str) -> anyhow::Result<ShareVideo> {
        let client = Self::client()?;
        let ttwid = Self::get_ttwid(&client).await?;
        let share_url = format!("https://www.iesdouyin.com/share/video/{}/", video_id);
        let html = client
            .get(&share_url)
            .header(COOKIE, format!("ttwid={}", ttwid))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_share_html(&html)
    }

    pub fn build_media_info(video: ShareVideo) -> MediaInfo {
        let mut qualities = Vec::new();

        let scale = |w: u32, h: u32, target: u32| -> (u32, u32) {
            if w >= h {
                let scaled = (w * target).checked_div(h).unwrap_or(target * 16 / 9);
                (scaled, target)
            } else {
                let scaled = (h * target).checked_div(w).unwrap_or(target * 16 / 9);
                (target, scaled)
            }
        };

        if video.height >= 1080 || video.width >= 1080 {
            let (w1080, h1080) = scale(video.width, video.height, 1080);
            qualities.push(MediaVideoQuality {
                label: "1080p".to_string(),
                width: w1080,
                height: h1080,
                url: video.play_url.replace("ratio=720p", "ratio=1080p"),
                format: "mp4".to_string(),
            });
        }

        let (w720, h720) = scale(video.width, video.height, 720);
        qualities.push(MediaVideoQuality {
            label: "720p".to_string(),
            width: w720,
            height: h720,
            url: video.play_url.replace("ratio=1080p", "ratio=720p"),
            format: "mp4".to_string(),
        });

        if qualities.is_empty() {
            qualities.push(MediaVideoQuality {
                label: "best".to_string(),
                width: video.width,
                height: video.height,
                url: video.play_url.clone(),
                format: "mp4".to_string(),
            });
        }

        MediaInfo {
            title: video.title,
            author: video.author,
            platform: "douyin".to_string(),
            duration_seconds: video.duration_seconds,
            thumbnail_url: video.thumbnail_url,
            available_qualities: qualities,
            media_type: MediaType::Video,
            file_size_bytes: None,
        }
    }
}

pub fn parse_share_html(html: &str) -> anyhow::Result<ShareVideo> {
    const MARKERS: [&str; 2] = ["window._ROUTER_DATA = ", "_ROUTER_DATA = "];
    let mut start_idx = None;
    for marker in MARKERS {
        if let Some(pos) = html.find(marker) {
            start_idx = Some(pos + marker.len());
            break;
        }
    }
    let start = start_idx.ok_or_else(|| anyhow!("Douyin share page has no _ROUTER_DATA"))?;
    let end = html[start..]
        .find("</script>")
        .map(|i| start + i)
        .ok_or_else(|| anyhow!("Douyin share page _ROUTER_DATA is not closed"))?;
    let raw_json = html[start..end].trim().trim_end_matches(';');
    let data: serde_json::Value =
        serde_json::from_str(raw_json).context("Douyin _ROUTER_DATA is not valid JSON")?;

    let item = data
        .get("loaderData")
        .and_then(|v| v.as_object())
        .and_then(|loaders| {
            loaders
                .values()
                .find_map(|page| page.pointer("/videoInfoRes/item_list/0"))
        })
        .ok_or_else(|| anyhow!("Douyin video unavailable (deleted, private or region-locked)"))?;

    let video = item
        .get("video")
        .ok_or_else(|| anyhow!("Douyin post has no video"))?;
    let watermarked = video
        .pointer("/play_addr/url_list/0")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Douyin post has no play address"))?;

    let desc = item.get("desc").and_then(|v| v.as_str()).unwrap_or("");
    let title: String = desc
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Douyin Video")
        .chars()
        .take(MAX_TITLE_CHARS)
        .collect();
    let title = if title.trim().is_empty() {
        "Douyin Video".to_string()
    } else {
        title
    };

    let author = item
        .pointer("/author/nickname")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let duration_seconds = video
        .get("duration")
        .and_then(|v| v.as_f64())
        .map(|ms| ms / 1000.0);

    let thumbnail_url = video
        .pointer("/cover/url_list/0")
        .and_then(|v| v.as_str())
        .map(String::from);

    let video_id = item
        .get("aweme_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let w = video.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let h = video.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let play_url = watermarked.replacen("/playwm/", "/play/", 1);

    Ok(ShareVideo {
        video_id,
        title,
        author,
        duration_seconds,
        thumbnail_url,
        width: w,
        height: h,
        play_url,
    })
}

#[async_trait]
impl PlatformDownloader for DouyinDownloader {
    fn name(&self) -> &str {
        "douyin"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return Self::host_matches(host);
            }
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let resolved = Self::resolve_url(url).await;
        let video_id = Self::extract_video_id(&resolved)?;
        let share_video = Self::fetch_share_video(&video_id).await?;
        Ok(Self::build_media_info(share_video))
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<ProgressUpdate>,
    ) -> anyhow::Result<DownloadResult> {
        let _ = progress.send(ProgressUpdate::percent(0.0)).await;

        let quality = opts
            .quality
            .as_ref()
            .and_then(|q| {
                let q_str = q.trim();
                info.available_qualities
                    .iter()
                    .find(|vq| vq.label == q_str || format!("{}p", vq.height) == q_str)
            })
            .or_else(|| info.available_qualities.first())
            .ok_or_else(|| anyhow!("No video URL available"))?;

        let title = if info.title.trim().is_empty() {
            "Douyin Video"
        } else {
            &info.title
        };
        let clean_title = sanitize_filename::sanitize(title);
        let clean_title = if clean_title.trim().is_empty() {
            "Douyin_Video".to_string()
        } else {
            clean_title
        };
        let filename = format!("{}.mp4", clean_title);
        let output = opts.output_dir.join(&filename);

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(MOBILE_UA));
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://www.iesdouyin.com/"),
        );
        http_client::inject_ua_header(&mut headers, opts.user_agent.as_deref());

        let client = Self::client()?;

        let bytes = direct_downloader::download_direct_with_headers(
            &client,
            &quality.url,
            &output,
            progress.clone(),
            Some(headers),
            Some(&opts.cancel_token),
        )
        .await?;

        Ok(DownloadResult {
            file_path: output,
            file_size_bytes: bytes,
            duration_seconds: info.duration_seconds.unwrap_or(0.0),
            torrent_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            DouyinDownloader::extract_video_id("https://www.douyin.com/video/7704466212453371235")
                .unwrap(),
            "7704466212453371235"
        );
        assert_eq!(
            DouyinDownloader::extract_video_id("https://www.douyin.com/note/7704466212453371235")
                .unwrap(),
            "7704466212453371235"
        );
        assert_eq!(
            DouyinDownloader::extract_video_id(
                "https://www.iesdouyin.com/share/video/7704466212453371235/"
            )
            .unwrap(),
            "7704466212453371235"
        );
        assert_eq!(
            DouyinDownloader::extract_video_id(
                "https://www.douyin.com/discover?modal_id=7704466212453371235"
            )
            .unwrap(),
            "7704466212453371235"
        );
    }

    #[test]
    fn test_parse_share_html() {
        let html = r#"<!DOCTYPE html><html><body>
        <script>window._ROUTER_DATA = {"loaderData":{"video_(id)/page":{"videoInfoRes":{"item_list":[{"aweme_id":"7704466212453371235","desc":"Test Douyin Title #tag","author":{"nickname":"Test Creator"},"video":{"duration":46301,"width":1920,"height":1080,"cover":{"url_list":["https://example.com/cover.webp"]},"play_addr":{"url_list":["https://aweme.snssdk.com/aweme/v1/playwm/?line=0&ratio=720p&video_id=v0300test"]}}}]}}}};</script>
        </body></html>"#;

        let video = parse_share_html(html).expect("must parse successfully");
        assert_eq!(video.video_id, "7704466212453371235");
        assert_eq!(video.title, "Test Douyin Title #tag");
        assert_eq!(video.author, "Test Creator");
        assert_eq!(video.duration_seconds, Some(46.301));
        assert_eq!(
            video.thumbnail_url.as_deref(),
            Some("https://example.com/cover.webp")
        );
        assert_eq!(video.width, 1920);
        assert_eq!(video.height, 1080);
        assert_eq!(
            video.play_url,
            "https://aweme.snssdk.com/aweme/v1/play/?line=0&ratio=720p&video_id=v0300test"
        );

        let media_info = DouyinDownloader::build_media_info(video);
        assert_eq!(media_info.platform, "douyin");
        assert_eq!(media_info.available_qualities.len(), 2);
        assert_eq!(media_info.available_qualities[0].label, "1080p");
        assert_eq!(media_info.available_qualities[1].label, "720p");
        assert!(media_info.available_qualities[0]
            .url
            .contains("ratio=1080p"));
        assert!(media_info.available_qualities[1].url.contains("ratio=720p"));
    }
}
