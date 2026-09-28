use std::path::Path;

pub const GDL_ARCHIVE: &str = "archive_gallery-dl.sqlite3";
pub const YTDLP_ARCHIVE: &str = "archive_yt-dlp.txt";
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub const PHOTO_FILTER: &str = "extension in ('jpg','jpeg','png','gif','webp','bmp','jfif','heic','avif','tiff','svg')";
pub const VIDEO_FILTER: &str = "extension in ('mp4','webm','mkv','mov','avi','m4v','flv','wmv','3gp','mpeg','mpg','ts','f4v','mts','m2ts')";

#[derive(Debug, Clone)]
pub struct GalleryDlArgsOptions<'a> {
    pub dest_dir: &'a Path,
    pub content_type: &'a str,
    pub platform: &'a str,
    pub clean_url: &'a str,
    pub username: &'a str,
    pub cookie_path: Option<&'a str>,
    pub custom_ua: Option<&'a str>,
    pub proxy: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct YtDlpArgsOptions<'a> {
    pub dest_dir: &'a Path,
    pub platform: &'a str,
    pub clean_url: &'a str,
    pub cookie_path: Option<&'a str>,
    pub proxy: Option<&'a str>,
}

pub fn get_platform_sleep(platform: &str) -> &'static str {
    match platform.to_lowercase().as_str() {
        "tiktok" => "0.6-1.2",
        "instagram" => "2.0-2.8",
        "facebook" => "1.5-2.5",
        _ => "0.8-1.5",
    }
}

pub fn build_gallery_dl_args(options: &GalleryDlArgsOptions) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-D".to_string(),
        options.dest_dir.to_string_lossy().to_string(),
        "-o".to_string(),
        format!("user-agent={}", options.custom_ua.unwrap_or(UA)),
        "-o".to_string(),
        "cookies-update=false".to_string(),
        "-o".to_string(),
        "headers.sec-ch-ua=\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"".to_string(),
        "-o".to_string(),
        "headers.sec-ch-ua-mobile=?0".to_string(),
        "-o".to_string(),
        "headers.sec-ch-ua-platform=\"Windows\"".to_string(),
        "--download-archive".to_string(),
        options.dest_dir.join(GDL_ARCHIVE).to_string_lossy().to_string(),
        "--retries".to_string(),
        "3".to_string(),
        "--sleep-429".to_string(),
        "10".to_string(),
        "--Print".to_string(),
        "after:FILE_OK:{filename}.{extension}".to_string(),
    ];

    if let Some(proxy) = options.proxy {
        if !proxy.trim().is_empty() {
            args.push("--proxy".to_string());
            args.push(proxy.to_string());
        }
    }

    let sleep_delay = get_platform_sleep(options.platform);
    let mut target_url = options.clean_url.to_string();

    match options.content_type {
        "photos" => {
            args.push("--filter".to_string());
            args.push(PHOTO_FILTER.to_string());
            if options.platform.eq_ignore_ascii_case("tiktok") {
                args.push("-o".to_string());
                args.push("videos=false".to_string());
                args.push("-o".to_string());
                args.push("audio=false".to_string());
            } else if options.platform.eq_ignore_ascii_case("instagram") {
                args.push("-o".to_string());
                args.push("videos=false".to_string());
                args.push("-o".to_string());
                args.push("include=posts,reels".to_string());
            }
            args.push("--sleep-request".to_string());
            args.push(sleep_delay.to_string());
        }
        "videos" => {
            args.push("--filter".to_string());
            args.push(VIDEO_FILTER.to_string());
            if options.platform.eq_ignore_ascii_case("instagram") {
                args.push("-o".to_string());
                args.push("include=posts,reels".to_string());
            }
            args.push("--sleep-request".to_string());
            args.push(sleep_delay.to_string());
        }
        "stories" => {
            if options.platform.eq_ignore_ascii_case("instagram") {
                args.push("-o".to_string());
                args.push("include=stories".to_string());
                args.push("--sleep-request".to_string());
                args.push("2.0-3.0".to_string());
                if !options.username.is_empty() {
                    target_url = format!("https://www.instagram.com/stories/{}/", options.username);
                }
            }
        }
        "highlights" => {
            if options.platform.eq_ignore_ascii_case("instagram") {
                args.push("-o".to_string());
                args.push("include=highlights".to_string());
                args.push("--sleep-request".to_string());
                args.push("2.0-3.0".to_string());
                if !options.username.is_empty() {
                    target_url = format!("https://www.instagram.com/{}/highlights/", options.username);
                }
            }
        }
        _ => {
            args.push("--sleep-request".to_string());
            args.push(sleep_delay.to_string());
        }
    }

    if let Some(cookie_path) = options.cookie_path {
        if !cookie_path.trim().is_empty() {
            args.push("--cookies".to_string());
            args.push(cookie_path.to_string());
        }
    }

    // Guard against CLI argument injection
    args.push("--".to_string());
    args.push(target_url);

    args
}

pub fn build_yt_dlp_args(options: &YtDlpArgsOptions) -> Vec<String> {
    let out_template = options.dest_dir.join("%(title).150B [%(id)s].%(ext)s").to_string_lossy().to_string();
    let mut args: Vec<String> = vec![
        "-o".to_string(),
        out_template,
        "--windows-filenames".to_string(),
        "--newline".to_string(),
        "--ignore-errors".to_string(),
        "--no-overwrites".to_string(),
        "--user-agent".to_string(),
        UA.to_string(),
        "--download-archive".to_string(),
        options.dest_dir.join(YTDLP_ARCHIVE).to_string_lossy().to_string(),
        "--concurrent-fragments".to_string(),
        "4".to_string(),
        "--retries".to_string(),
        "3".to_string(),
        "--fragment-retries".to_string(),
        "5".to_string(),
        "--impersonate".to_string(),
        "chrome".to_string(),
    ];

    let plat_lower = options.platform.to_lowercase();
    if plat_lower == "tiktok" {
        args.push("--sleep-requests".to_string());
        args.push("0.5:1.2".to_string());
        args.push("--retry-sleep".to_string());
        args.push("429:exp=5:30".to_string());
    } else if plat_lower == "instagram" {
        args.push("--sleep-requests".to_string());
        args.push("1.0:2.0".to_string());
        args.push("--retry-sleep".to_string());
        args.push("429:exp=5:30".to_string());
    } else if plat_lower == "x" {
        args.push("--sleep-requests".to_string());
        args.push("0.8:1.8".to_string());
        args.push("--retry-sleep".to_string());
        args.push("429:exp=5:30".to_string());
    }

    if let Some(proxy) = options.proxy {
        if !proxy.trim().is_empty() {
            args.push("--proxy".to_string());
            args.push(proxy.to_string());
        }
    }

    if let Some(cookie_path) = options.cookie_path {
        if !cookie_path.trim().is_empty() {
            args.push("--cookies".to_string());
            args.push(cookie_path.to_string());
        }
    }

    // Guard against CLI argument injection
    args.push("--".to_string());
    args.push(options.clean_url.to_string());

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_gallery_dl_args_anti_bot_and_injection_guard() {
        let dest = PathBuf::from("C:/downloads/OpenNami/instagram/test/photos");
        let opts = GalleryDlArgsOptions {
            dest_dir: &dest,
            content_type: "photos",
            platform: "instagram",
            clean_url: "https://www.instagram.com/test/",
            username: "test",
            cookie_path: Some("C:/cookies/ig.txt"),
            custom_ua: None,
            proxy: None,
        };

        let args = build_gallery_dl_args(&opts);

        assert!(args.contains(&"cookies-update=false".to_string()));
        assert!(args.contains(&"--sleep-429".to_string()));
        assert!(args.contains(&"--cookies".to_string()));
        assert!(args.contains(&"C:/cookies/ig.txt".to_string()));

        let dash_dash_pos = args.iter().position(|r| r == "--").unwrap();
        assert_eq!(dash_dash_pos, args.len() - 2);
        assert_eq!(args[args.len() - 1], "https://www.instagram.com/test/");
    }

    #[test]
    fn test_yt_dlp_args_impersonate_and_backoff() {
        let dest = PathBuf::from("C:/downloads/OpenNami/tiktok/creator/videos");
        let opts = YtDlpArgsOptions {
            dest_dir: &dest,
            platform: "tiktok",
            clean_url: "https://www.tiktok.com/@creator",
            cookie_path: None,
            proxy: None,
        };

        let args = build_yt_dlp_args(&opts);

        assert!(args.contains(&"--impersonate".to_string()));
        assert!(args.contains(&"chrome".to_string()));
        assert!(args.contains(&"--retry-sleep".to_string()));
        assert!(args.contains(&"429:exp=5:30".to_string()));

        let dash_dash_pos = args.iter().position(|r| r == "--").unwrap();
        assert_eq!(dash_dash_pos, args.len() - 2);
        assert_eq!(args[args.len() - 1], "https://www.tiktok.com/@creator");
    }
}
