//! Snapchat: public Spotlight videos, public story highlights and public
//! profiles, read from the same public web pages snapchat.com serves to any
//! browser. No login, no private API.
//!
//! * Spotlight pages carry a schema.org `VideoObject` (and `og:video`).
//! * Profile and highlight pages carry Next.js `__NEXT_DATA__` JSON with the
//!   public story, curated highlights and Spotlight snaps.

use std::sync::LazyLock;

use anyhow::anyhow;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::core::direct_downloader;
use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::models::progress::ProgressUpdate;
use crate::platforms::traits::PlatformDownloader;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

static NEXT_DATA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<script id="__NEXT_DATA__" type="application/json"[^>]*>(.*?)</script>"#)
        .expect("valid NEXT_DATA_RE")
});
static LD_JSON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<script type="application/ld\+json"[^>]*>(.*?)</script>"#)
        .expect("valid LD_JSON_RE")
});
static OG_VIDEO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<meta property="og:video" content="([^"]+)""#).expect("valid OG_VIDEO_RE")
});

/// What a Snapchat URL points to.
#[derive(Debug, PartialEq, Eq)]
pub enum SnapTarget {
    Spotlight,
    Highlight(String),
    Profile(String),
}

pub fn is_snapchat_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "snapchat.com" || host.ends_with(".snapchat.com")
}

pub fn classify(url: &str) -> Option<SnapTarget> {
    let parsed = url::Url::parse(url).ok()?;
    if !is_snapchat_host(parsed.host_str()?) {
        return None;
    }
    let segs: Vec<&str> = parsed.path_segments()?.filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        ["spotlight", _, ..] => Some(SnapTarget::Spotlight),
        [user, "spotlight", _, ..] if user.starts_with('@') => Some(SnapTarget::Spotlight),
        [user, "highlight", id, ..] if user.starts_with('@') => {
            Some(SnapTarget::Highlight((*id).to_string()))
        }
        [user, ..] if user.len() > 1 && user.starts_with('@') => {
            Some(SnapTarget::Profile(user[1..].to_string()))
        }
        ["add", user, ..] => Some(SnapTarget::Profile((*user).to_string())),
        _ => None,
    }
}

/// One downloadable snap.
#[derive(Debug, Clone, PartialEq)]
pub struct Snap {
    pub url: String,
    pub is_video: bool,
}

impl Snap {
    fn ext(&self) -> &'static str {
        if self.is_video {
            "mp4"
        } else {
            "jpg"
        }
    }
}

fn unescape_attr(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
}

/// Spotlight page: the `VideoObject` JSON-LD first, then `og:video`.
pub fn parse_spotlight(html: &str) -> Option<(String, Snap)> {
    for cap in LD_JSON_RE.captures_iter(html) {
        let Ok(v) = serde_json::from_str::<Value>(&cap[1]) else {
            continue;
        };
        if v["@type"] == "VideoObject" {
            if let Some(url) = v["contentUrl"].as_str() {
                let title = v["name"]
                    .as_str()
                    .unwrap_or("snapchat_spotlight")
                    .to_string();
                return Some((
                    title,
                    Snap {
                        url: url.to_string(),
                        is_video: true,
                    },
                ));
            }
        }
    }
    let url = OG_VIDEO_RE.captures(html)?.get(1)?.as_str();
    Some((
        "snapchat_spotlight".to_string(),
        Snap {
            url: unescape_attr(url),
            is_video: true,
        },
    ))
}

fn snaps_of(story: &Value, out: &mut Vec<Snap>) {
    let Some(list) = story["snapList"].as_array() else {
        return;
    };
    for s in list {
        let Some(url) = s["snapUrls"]["mediaUrl"].as_str() else {
            continue;
        };
        if url.is_empty() || out.iter().any(|x| x.url == url) {
            continue;
        }
        out.push(Snap {
            url: url.to_string(),
            // 0 = image, 1 = video (checked against the served bytes).
            is_video: s["snapMediaType"].as_i64() != Some(0),
        });
    }
}

/// Profile or highlight page: snaps listed in `__NEXT_DATA__`.
pub fn parse_next_data(html: &str, target: &SnapTarget) -> Option<(String, Vec<Snap>)> {
    let raw = NEXT_DATA_RE.captures(html)?.get(1)?.as_str();
    let v: Value = serde_json::from_str(raw).ok()?;
    let pp = &v["props"]["pageProps"];
    let mut snaps = Vec::new();
    let title = match target {
        SnapTarget::Highlight(id) => {
            snaps_of(&pp["highlight"], &mut snaps);
            let name = pp["highlight"]["storyTitle"]["value"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(id);
            format!("snapchat_{name}")
        }
        SnapTarget::Profile(user) => {
            snaps_of(&pp["story"], &mut snaps);
            for key in ["curatedHighlights", "spotlightHighlights"] {
                if let Some(stories) = pp[key].as_array() {
                    for st in stories {
                        snaps_of(st, &mut snaps);
                    }
                }
            }
            format!("snapchat_{user}")
        }
        SnapTarget::Spotlight => return None,
    };
    (!snaps.is_empty()).then_some((title, snaps))
}

pub fn media_info(title: String, snaps: Vec<Snap>) -> MediaInfo {
    let media_type = match snaps.as_slice() {
        [one] if one.is_video => MediaType::Video,
        [_] => MediaType::Photo,
        _ => MediaType::Carousel,
    };
    MediaInfo {
        title,
        author: String::new(),
        platform: "snapchat".to_string(),
        duration_seconds: None,
        thumbnail_url: None,
        available_qualities: snaps
            .iter()
            .map(|s| VideoQuality {
                label: "original".to_string(),
                width: 0,
                height: 0,
                url: s.url.clone(),
                format: s.ext().to_string(),
            })
            .collect(),
        media_type,
        file_size_bytes: None,
    }
}

pub struct SnapchatDownloader {
    client: reqwest::Client,
}

impl Default for SnapchatDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapchatDownloader {
    pub fn new() -> Self {
        let client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    async fn fetch(&self, url: &str) -> anyhow::Result<(String, String)> {
        let resp = self
            .client
            .get(url)
            .header("Accept", "text/html")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("HTTP {} from Snapchat", resp.status()));
        }
        let final_url = resp.url().to_string();
        Ok((final_url, resp.text().await?))
    }
}

#[async_trait]
impl PlatformDownloader for SnapchatDownloader {
    fn name(&self) -> &str {
        "snapchat"
    }

    fn can_handle(&self, url: &str) -> bool {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(is_snapchat_host))
            .unwrap_or(false)
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        // Short links (t.snapchat.com) redirect to the canonical page.
        let (final_url, html) = self.fetch(url).await?;
        let target = classify(&final_url)
            .or_else(|| classify(url))
            .ok_or_else(|| anyhow!("Unsupported Snapchat link"))?;
        let parsed = match target {
            SnapTarget::Spotlight => parse_spotlight(&html).map(|(t, s)| (t, vec![s])),
            ref other => parse_next_data(&html, other),
        };
        let (title, snaps) =
            parsed.ok_or_else(|| anyhow!("No public Snapchat media found on this page"))?;
        Ok(media_info(title, snaps))
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<ProgressUpdate>,
    ) -> anyhow::Result<DownloadResult> {
        let count = info.available_qualities.len();
        if count == 0 {
            return Err(anyhow!("No media URL available"));
        }
        let base = sanitize_filename::sanitize(&info.title);
        let mut total_bytes = 0u64;
        let mut last_path = opts.output_dir.clone();
        for (i, q) in info.available_qualities.iter().enumerate() {
            let filename = if count == 1 {
                format!("{base}.{}", q.format)
            } else {
                format!("{base}_{:03}.{}", i + 1, q.format)
            };
            let output = opts.output_dir.join(filename);
            let (tx, _rx) = mpsc::channel(8);
            total_bytes += direct_downloader::download_direct(
                &self.client,
                &q.url,
                &output,
                tx,
                Some(&opts.cancel_token),
            )
            .await?;
            last_path = output;
            let percent = ((i + 1) as f64 / count as f64) * 100.0;
            let _ = progress.send(ProgressUpdate::percent(percent)).await;
        }
        Ok(DownloadResult {
            file_path: last_path,
            file_size_bytes: total_bytes,
            duration_seconds: 0.0,
            torrent_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_snapchat_links() {
        assert_eq!(
            classify("https://www.snapchat.com/spotlight/W7_abc"),
            Some(SnapTarget::Spotlight)
        );
        assert_eq!(
            classify("https://www.snapchat.com/@nasa/spotlight/W7_abc"),
            Some(SnapTarget::Spotlight)
        );
        assert_eq!(
            classify("https://www.snapchat.com/@nasa/highlight/ddf0-11"),
            Some(SnapTarget::Highlight("ddf0-11".into()))
        );
        assert_eq!(
            classify("https://www.snapchat.com/@nasa"),
            Some(SnapTarget::Profile("nasa".into()))
        );
        assert_eq!(
            classify("https://www.snapchat.com/add/nasa"),
            Some(SnapTarget::Profile("nasa".into()))
        );
        assert_eq!(classify("https://www.snapchat.com/"), None);
        assert_eq!(classify("https://evil.com/@nasa"), None);
        assert_eq!(classify("https://snapchat.com.evil.com/@nasa"), None);
    }

    #[test]
    fn spotlight_reads_json_ld_then_og_video() {
        let html = r#"<script type="application/ld+json">{"@type":"BreadcrumbList"}</script>
<script type="application/ld+json">{"@type":"VideoObject","name":"Moon","contentUrl":"https://cf-st.sc-cdn.net/d/a.27?mo=x&uc=46"}</script>"#;
        let (title, snap) = parse_spotlight(html).unwrap();
        assert_eq!(title, "Moon");
        assert_eq!(snap.url, "https://cf-st.sc-cdn.net/d/a.27?mo=x&uc=46");

        let og =
            r#"<meta property="og:video" content="https://cf-st.sc-cdn.net/d/b?mo=y&amp;uc=46"/>"#;
        assert_eq!(
            parse_spotlight(og).unwrap().1.url,
            "https://cf-st.sc-cdn.net/d/b?mo=y&uc=46"
        );
        assert!(parse_spotlight("<html></html>").is_none());
    }

    fn next_data(page_props: &str) -> String {
        format!(
            r#"<script id="__NEXT_DATA__" type="application/json">{{"props":{{"pageProps":{page_props}}}}}</script>"#
        )
    }

    #[test]
    fn profile_collects_story_highlights_and_spotlight_without_duplicates() {
        let html = next_data(
            r#"{"story":{"snapList":[{"snapMediaType":0,"snapUrls":{"mediaUrl":"https://x/1"}}]},
            "curatedHighlights":[{"snapList":[{"snapMediaType":1,"snapUrls":{"mediaUrl":"https://x/2"}},
                                              {"snapMediaType":1,"snapUrls":{"mediaUrl":"https://x/1"}}]}],
            "spotlightHighlights":[{"snapList":[{"snapMediaType":1,"snapUrls":{"mediaUrl":"https://x/3"}}]}]}"#,
        );
        let (title, snaps) = parse_next_data(&html, &SnapTarget::Profile("nasa".into())).unwrap();
        assert_eq!(title, "snapchat_nasa");
        let urls: Vec<_> = snaps.iter().map(|s| s.url.as_str()).collect();
        assert_eq!(urls, ["https://x/1", "https://x/2", "https://x/3"]);
        assert!(!snaps[0].is_video && snaps[1].is_video);
        let info = media_info(title, snaps);
        assert_eq!(info.media_type, MediaType::Carousel);
        assert_eq!(info.available_qualities[0].format, "jpg");
        assert_eq!(info.available_qualities[1].format, "mp4");
    }

    #[test]
    fn highlight_uses_its_title() {
        let html = next_data(
            r#"{"highlight":{"storyTitle":{"value":"Moon Joy"},"snapList":[{"snapMediaType":1,"snapUrls":{"mediaUrl":"https://x/9"}}]}}"#,
        );
        let (title, snaps) = parse_next_data(&html, &SnapTarget::Highlight("id".into())).unwrap();
        assert_eq!(title, "snapchat_Moon Joy");
        assert_eq!(media_info(title, snaps).media_type, MediaType::Video);
    }

    #[test]
    fn empty_profile_is_an_error() {
        let html = next_data(r#"{"story":null,"curatedHighlights":[]}"#);
        assert!(parse_next_data(&html, &SnapTarget::Profile("x".into())).is_none());
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Network test against a public NASA profile: `cargo test -- --ignored snapchat_live`.
    #[tokio::test]
    #[ignore]
    async fn snapchat_live_profile_and_spotlight() {
        let d = SnapchatDownloader::new();
        let info = d
            .get_media_info("https://www.snapchat.com/@nasa")
            .await
            .unwrap();
        assert!(info.available_qualities.len() > 1);
        let url = info
            .available_qualities
            .iter()
            .find(|q| q.format == "mp4")
            .unwrap();
        assert!(url.url.starts_with("https://"));
    }
}
