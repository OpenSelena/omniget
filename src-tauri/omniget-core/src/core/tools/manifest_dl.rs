//! Downloads an HLS (.m3u8) or DASH (.mpd) manifest straight through FFmpeg
//! (studies 34 and 54): `-c copy`, with the user's headers and cookies.
//! DRM-protected manifests are refused, never decrypted.

use std::path::PathBuf;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestOptions {
    pub url: String,
    pub dest_dir: String,
    pub file_name: String,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub cookie: String,
    #[serde(default)]
    pub extra_headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestResult {
    pub path: String,
    pub seconds: f64,
}

pub fn headers_arg(opts: &ManifestOptions) -> Option<String> {
    let mut h = Vec::new();
    if !opts.referer.trim().is_empty() {
        h.push(format!("Referer: {}", opts.referer.trim()));
        h.push(format!(
            "Origin: {}",
            opts.referer.trim().trim_end_matches('/')
        ));
    }
    if !opts.cookie.trim().is_empty() {
        h.push(format!("Cookie: {}", opts.cookie.trim()));
    }
    for e in &opts.extra_headers {
        if e.contains(':') {
            h.push(e.trim().to_string());
        }
    }
    if h.is_empty() {
        None
    } else {
        Some(h.join("\r\n") + "\r\n")
    }
}

/// Protocols ffmpeg may open for a manifest. `file` is left out so a
/// playlist cannot pull local files into the output.
pub const PROTOCOL_WHITELIST: &str = "http,https,tcp,tls,crypto,data";

/// The manifest must be a public http(s) URL.
pub async fn checked_manifest_url(raw: &str) -> anyhow::Result<String> {
    let url = raw.trim().to_string();
    let parsed =
        url::Url::parse(&url).map_err(|_| anyhow!("paste the URL of a .m3u8 or .mpd manifest"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(anyhow!("paste the http(s) URL of a .m3u8 or .mpd manifest"));
    }
    crate::core::net_guard::ensure_public_web_url(&url).await?;
    Ok(url)
}

pub async fn download(
    opts: ManifestOptions,
    progress: super::ProgressFn,
) -> anyhow::Result<ManifestResult> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let url = checked_manifest_url(&opts.url).await?;
    let ffmpeg = crate::core::dependencies::ensure_ffmpeg().await?;
    std::fs::create_dir_all(&opts.dest_dir)?;
    let mut name = super::sanitize_name(opts.file_name.trim());
    if !name.to_lowercase().ends_with(".mp4") && !name.to_lowercase().ends_with(".mkv") {
        name.push_str(".mp4");
    }
    let out = PathBuf::from(&opts.dest_dir).join(name);
    let mut cmd = crate::core::process::command(&ffmpeg);
    // Segments and keys named by the playlist go through the egress guard.
    crate::core::egress_proxy::apply_to_command(&mut cmd);
    cmd.args([
        "-y",
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostats",
        "-progress",
        "pipe:1",
    ]);
    if !opts.user_agent.trim().is_empty() {
        cmd.args(["-user_agent", opts.user_agent.trim()]);
    }
    if let Some(h) = headers_arg(&opts) {
        cmd.args(["-headers", &h]);
    }
    cmd.args([
        "-protocol_whitelist",
        PROTOCOL_WHITELIST,
        "-allowed_extensions",
        "ALL",
        "-i",
        &url,
    ]);
    cmd.args([
        "-c",
        "copy",
        "-bsf:a",
        "aac_adtstoasc",
        "-movflags",
        "+faststart",
    ])
    .arg(&out);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let id = format!("manifest:{}", url);
    let p2 = progress.clone();
    let id2 = id.clone();
    let t1 = tokio::spawn(async move {
        let mut secs = 0.0f64;
        if let Some(o) = stdout {
            let mut lines = BufReader::new(o).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(us) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|v| v.trim().parse::<u64>().ok())
                {
                    secs = us as f64 / 1_000_000.0;
                    super::report(
                        &p2,
                        &id2,
                        "progress",
                        us / 1_000_000,
                        None,
                        Some(format!("{:.0}s", secs)),
                    );
                }
            }
        }
        secs
    });
    let t2 = tokio::spawn(async move {
        let mut tail = String::new();
        if let Some(e) = stderr {
            let mut lines = BufReader::new(e).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    tail = line;
                }
            }
        }
        tail
    });
    let st = child.wait().await?;
    let secs = t1.await.unwrap_or(0.0);
    let tail = t2.await.unwrap_or_default();
    if !st.success() {
        let lower_tail = tail.to_lowercase();
        if lower_tail.contains("drm") || lower_tail.contains("cenc") {
            return Err(anyhow!("{}", crate::core::drm::DRM_ERROR));
        }
        let hint = if tail.contains("403") || tail.contains("401") {
            " (the server refused: missing Referer/cookie, or the link expired)"
        } else {
            ""
        };
        return Err(anyhow!("ffmpeg failed: {}{}", tail, hint));
    }
    super::report(&progress, &id, "done", secs as u64, Some(secs as u64), None);
    Ok(ManifestResult {
        path: out.to_string_lossy().to_string(),
        seconds: secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manifest_urls_must_be_public_web_links() {
        for bad in [
            "file:///etc/passwd",
            "/home/me/list.m3u8",
            "http://192.168.1.1/live.m3u8",
            "https://127.0.0.1/x.mpd",
            "data:application/vnd.apple.mpegurl,#EXTM3U",
        ] {
            assert!(checked_manifest_url(bad).await.is_err(), "accepted {bad}");
        }
        assert!(!PROTOCOL_WHITELIST.split(',').any(|p| p == "file"));
        assert!(checked_manifest_url(" https://93.184.215.14/a.m3u8 ")
            .await
            .is_ok());
    }

    #[test]
    fn builds_headers() {
        let o = ManifestOptions {
            url: "".into(),
            dest_dir: "".into(),
            file_name: "".into(),
            referer: "https://x.com/".into(),
            user_agent: "".into(),
            cookie: "a=1".into(),
            extra_headers: vec!["X-Y: z".into()],
        };
        assert_eq!(
            headers_arg(&o).unwrap(),
            "Referer: https://x.com/\r\nOrigin: https://x.com\r\nCookie: a=1\r\nX-Y: z\r\n"
        );
    }
}
