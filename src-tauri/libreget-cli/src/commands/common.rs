use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use libreget_core::core::http_client;
use libreget_core::core::registry::PlatformRegistry;
use libreget_core::core::ytdlp;
use libreget_core::models::media::{DownloadOptions, MediaInfo};
use libreget_core::models::settings::ProxySettings;
use libreget_core::platforms::{
    BilibiliDownloader, BlueskyDownloader, DirectFileDownloader, DouyinDownloader,
    GenericYtdlpDownloader, InstagramDownloader, P2pDownloader, PinterestDownloader,
    PlatformDownloader, RedditDownloader, ThreadsDownloader, TikTokDownloader,
    TwitchClipsDownloader, TwitterDownloader, VimeoDownloader, YouTubeDownloader,
};
use tokio_util::sync::CancellationToken;

use crate::reporter;

pub fn init_cli_runtime(proxy: Option<&str>) -> Result<()> {
    init_cookie_provider();

    if let Some(proxy_url) = proxy {
        http_client::init_proxy(parse_proxy(proxy_url)?);
    } else {
        http_client::init_proxy(ProxySettings::default());
    }

    Ok(())
}

fn init_cookie_provider() {
    ytdlp::set_global_cookie_file_fn(|| {
        reporter::default_cookie_path().map(|path| path.to_string_lossy().to_string())
    });
}

fn parse_proxy(raw: &str) -> Result<ProxySettings> {
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| anyhow!("Proxy must include a scheme, e.g. http://127.0.0.1:7897"))?;

    let proxy_type = match scheme {
        "http" | "https" | "socks5" => scheme.to_string(),
        other => return Err(anyhow!("Unsupported proxy scheme: {}", other)),
    };

    let authority = rest.split('/').next().unwrap_or(rest);
    let (auth, host_port) = match authority.rsplit_once('@') {
        Some((auth, host_port)) => (Some(auth), host_port),
        None => (None, authority),
    };

    let (host, port) = host_port
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("Proxy must include host and port, e.g. http://127.0.0.1:7897"))?;

    if host.is_empty() {
        return Err(anyhow!("Proxy host cannot be empty"));
    }

    let port = port
        .parse::<u16>()
        .with_context(|| format!("Invalid proxy port: {}", port))?;

    let (username, password) = match auth.and_then(|a| a.split_once(':')) {
        Some((u, p)) => (u.to_string(), p.to_string()),
        None => (String::new(), String::new()),
    };

    Ok(ProxySettings {
        enabled: true,
        proxy_type,
        host: host.to_string(),
        port,
        username,
        password,
    })
}

pub fn core_platform_registry() -> PlatformRegistry {
    let mut registry = PlatformRegistry::new();
    registry.register(Arc::new(InstagramDownloader::new()));
    registry.register(Arc::new(ThreadsDownloader::new()));
    registry.register(Arc::new(PinterestDownloader::new()));
    registry.register(Arc::new(TikTokDownloader::new()));
    registry.register(Arc::new(TwitchClipsDownloader::new()));
    registry.register(Arc::new(TwitterDownloader::new()));
    registry.register(Arc::new(BlueskyDownloader::new()));
    registry.register(Arc::new(RedditDownloader::new()));
    registry.register(Arc::new(YouTubeDownloader::new()));
    registry.register(Arc::new(VimeoDownloader::new()));
    registry.register(Arc::new(BilibiliDownloader::new()));
    registry.register(Arc::new(DouyinDownloader::new()));
    registry.register(Arc::new(P2pDownloader::new()));
    registry.register(Arc::new(DirectFileDownloader::new()));
    registry.register(Arc::new(GenericYtdlpDownloader::new()));
    registry
}

fn is_cjk_or_punct(c: char) -> bool {
    matches!(
        c,
        '\u{3000}'..='\u{303F}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{FF01}'..='\u{FF5E}'
        | '^' | '<' | '>'
    )
}

pub fn extract_url(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("magnet:")
        || trimmed.starts_with("p2p:")
    {
        return trimmed;
    }

    for prefix in &["https://", "http://", "magnet:?xt=", "p2p:"] {
        if let Some(start_idx) = trimmed.find(prefix) {
            let slice = &trimmed[start_idx..];
            let end_idx = slice
                .find(|c: char| c.is_whitespace() || is_cjk_or_punct(c))
                .unwrap_or(slice.len());
            let candidate = &slice[..end_idx];
            let cleaned = candidate
                .trim_end_matches(&['.', ',', ';', ':', '!', '?', '\'', '"', ')', ']', '>'][..]);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    trimmed
}

pub async fn resolve_media_info(
    registry: &PlatformRegistry,
    raw_url: &str,
) -> Result<(Arc<dyn PlatformDownloader>, MediaInfo)> {
    let url = extract_url(raw_url);
    let platform = registry
        .find_platform(url)
        .unwrap_or_else(|| Arc::new(GenericYtdlpDownloader::new()));

    match platform.get_media_info(url).await {
        Ok(info) => Ok((platform, info)),
        Err(primary_error)
            if platform.name() != "generic" && GenericYtdlpDownloader::new().can_handle(url) =>
        {
            let generic: Arc<dyn PlatformDownloader> = Arc::new(GenericYtdlpDownloader::new());
            match generic.get_media_info(url).await {
                Ok(info) => Ok((generic, info)),
                Err(fallback_error) => Err(fallback_error).with_context(|| {
                    format!(
                        "{} extractor failed first: {}; generic fallback also failed",
                        platform.name(),
                        primary_error
                    )
                }),
            }
        }
        Err(error) => Err(error),
    }
}

/// Turns `--format` into a yt-dlp format selector and extra flags.
///
/// `mp4`, `mkv` and `webm` are container names, not selectors: handed to yt-dlp
/// as-is, `mp4` means "best pre-muxed MP4", which on YouTube is the 360p itag 18
/// and wins over the quality-limited fallback appended after it. So a container
/// becomes a stream filter that keeps the height limit, plus the merge container.
/// Anything else is passed through as a raw selector.
fn video_format(format: &str, quality: Option<u32>) -> (Option<String>, Vec<String>) {
    let (height, fallback) = match quality.filter(|h| *h > 0) {
        Some(h) => (
            format!("[height<={h}]"),
            format!("bv*[height<={h}]+ba/b[height<={h}]/b"),
        ),
        None => (String::new(), "bv*+ba/b".to_string()),
    };
    let (selector, merge) = match format.trim().to_ascii_lowercase().as_str() {
        "mp4" => (
            Some(format!("bv*[ext=mp4]{height}+ba[ext=m4a]/{fallback}")),
            "mp4",
        ),
        // WebM can't hold AAC, so let yt-dlp fall back to MKV when only
        // incompatible streams are left.
        "webm" => (
            Some(format!("bv*[ext=webm]{height}+ba[ext=webm]/{fallback}")),
            "webm/mkv",
        ),
        "mkv" => (None, "mkv"),
        _ => return (Some(format.to_string()), Vec::new()),
    };
    let mut flags = vec!["--merge-output-format".to_string(), merge.to_string()];
    if merge.starts_with("webm") {
        // yt-dlp can't embed a thumbnail in WebM and fails the whole download
        // in post-processing when "embed thumbnail" is on.
        flags.push("--no-embed-thumbnail".to_string());
    }
    (selector, flags)
}

pub fn download_options(
    output_dir: PathBuf,
    quality: Option<u32>,
    audio_only: bool,
    subs: Option<String>,
    format: Option<String>,
    ytdlp_path: Option<PathBuf>,
) -> DownloadOptions {
    let mut custom_ytdlp_args = Vec::new();
    let download_subtitles = subs.is_some();
    if let Some(lang) = subs {
        custom_ytdlp_args.push("--write-subs".to_string());
        custom_ytdlp_args.push("--sub-langs".to_string());
        custom_ytdlp_args.push(lang);
    }

    let mut format_id = None;
    if !audio_only {
        if let Some(format) = format.as_deref() {
            let (selector, flags) = video_format(format, quality);
            format_id = selector;
            custom_ytdlp_args.extend(flags);
        }
    }

    DownloadOptions {
        quality: quality.map(|q| q.to_string()),
        output_dir,
        filename_template: None,
        download_subtitles,
        include_auto_subtitles: false,
        download_mode: audio_only.then(|| "audio".to_string()),
        audio_format: audio_only.then(|| format.clone()).flatten(),
        format_id,
        referer: None,
        extra_headers: None,
        page_url: None,
        user_agent: None,
        cancel_token: CancellationToken::new(),
        concurrent_fragments: 4,
        ytdlp_path,
        torrent_listen_port: None,
        torrent_id_slot: None,
        custom_ytdlp_args: (!custom_ytdlp_args.is_empty()).then_some(custom_ytdlp_args),
        torrent_files: None,
        torrent_auto_trackers: true,
        torrent_upnp: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(quality: Option<u32>, audio_only: bool, format: &str) -> DownloadOptions {
        download_options(
            PathBuf::from("out"),
            quality,
            audio_only,
            None,
            Some(format.to_string()),
            None,
        )
    }

    #[test]
    fn mp4_keeps_the_quality_limit() {
        let o = opts(Some(1080), false, "mp4");
        assert_eq!(
            o.format_id.as_deref(),
            Some("bv*[ext=mp4][height<=1080]+ba[ext=m4a]/bv*[height<=1080]+ba/b[height<=1080]/b")
        );
        assert_eq!(
            o.custom_ytdlp_args,
            Some(vec!["--merge-output-format".to_string(), "mp4".to_string()])
        );
    }

    #[test]
    fn mp4_without_quality_still_falls_back() {
        let o = opts(None, false, "MP4");
        assert_eq!(
            o.format_id.as_deref(),
            Some("bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b")
        );
    }

    #[test]
    fn webm_prefers_webm_streams_and_may_merge_into_mkv() {
        let o = opts(Some(720), false, "webm");
        assert_eq!(
            o.format_id.as_deref(),
            Some("bv*[ext=webm][height<=720]+ba[ext=webm]/bv*[height<=720]+ba/b[height<=720]/b")
        );
        assert_eq!(
            o.custom_ytdlp_args,
            Some(vec![
                "--merge-output-format".to_string(),
                "webm/mkv".to_string(),
                "--no-embed-thumbnail".to_string(),
            ])
        );
    }

    #[test]
    fn mkv_only_sets_the_merge_container() {
        let o = opts(Some(2160), false, "mkv");
        assert_eq!(o.format_id, None);
        assert_eq!(
            o.custom_ytdlp_args,
            Some(vec!["--merge-output-format".to_string(), "mkv".to_string()])
        );
    }

    #[test]
    fn raw_selectors_pass_through() {
        let o = opts(Some(1080), false, "137+140");
        assert_eq!(o.format_id.as_deref(), Some("137+140"));
        assert_eq!(o.custom_ytdlp_args, None);
    }

    #[test]
    fn audio_only_uses_format_as_audio_format() {
        let o = opts(None, true, "mp3");
        assert_eq!(o.format_id, None);
        assert_eq!(o.audio_format.as_deref(), Some("mp3"));
        assert_eq!(o.custom_ytdlp_args, None);
    }

    #[test]
    fn extract_url_from_share_text() {
        assert_eq!(
            extract_url("https://www.bilibili.com/video/BV1GJ411x7h7"),
            "https://www.bilibili.com/video/BV1GJ411x7h7"
        );
        assert_eq!(
            extract_url(
                "复制打开抖音，看看这个 https://www.bilibili.com/video/BV1GJ411x7h7 复制此链接"
            ),
            "https://www.bilibili.com/video/BV1GJ411x7h7"
        );
        assert_eq!(
            extract_url("Check this: https://example.com/video.mp4!"),
            "https://example.com/video.mp4"
        );
        assert_eq!(
            extract_url("看作品https://v.douyin.com/xyz/，复制"),
            "https://v.douyin.com/xyz/"
        );
        assert_eq!(extract_url("regular search query"), "regular search query");
    }
}
