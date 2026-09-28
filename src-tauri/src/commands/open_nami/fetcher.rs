use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const YTDLP_RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
pub const GALLERYDL_GITHUB_FALLBACK: &str = "https://github.com/mikf/gallery-dl/releases/latest/download";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineStatus {
    pub gallery_dl_path: Option<PathBuf>,
    pub gallery_dl_version: Option<String>,
    pub yt_dlp_path: Option<PathBuf>,
    pub yt_dlp_version: Option<String>,
    pub ready: bool,
}

pub fn get_open_nami_bin_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("open_nami").join("bin")
}

pub fn gallery_dl_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "gallery-dl.exe"
    } else {
        "gallery-dl.bin"
    }
}

pub fn yt_dlp_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else if cfg!(target_arch = "aarch64") {
        "yt-dlp_linux_aarch64"
    } else {
        "yt-dlp_linux"
    }
}

pub fn get_binary_version(executable_path: &Path) -> Option<String> {
    let mut cmd = Command::new(executable_path);
    cmd.arg("--version");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    match cmd.output() {
        Ok(out) if out.status.success() => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !v.is_empty() {
                Some(v)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn resolve_gallery_dl(app_data_dir: &Path) -> Option<PathBuf> {
    let bin_dir = get_open_nami_bin_dir(app_data_dir);
    let local = bin_dir.join(if cfg!(target_os = "windows") { "gallery-dl.exe" } else { "gallery-dl" });
    if local.exists() {
        return Some(local);
    }

    if let Ok(p) = which::which("gallery-dl") {
        return Some(p);
    }

    None
}

pub fn resolve_yt_dlp(app_data_dir: &Path) -> Option<PathBuf> {
    let bin_dir = get_open_nami_bin_dir(app_data_dir);
    let local = bin_dir.join(if cfg!(target_os = "windows") { "yt-dlp.exe" } else { "yt-dlp" });
    if local.exists() {
        return Some(local);
    }

    if let Ok(p) = which::which("yt-dlp") {
        return Some(p);
    }

    None
}

pub fn check_engines_status(app_data_dir: &Path) -> EngineStatus {
    let g_path = resolve_gallery_dl(app_data_dir);
    let g_ver = g_path.as_deref().and_then(get_binary_version);

    let yt_path = resolve_yt_dlp(app_data_dir);
    let yt_ver = yt_path.as_deref().and_then(get_binary_version);

    let ready = g_ver.is_some();

    EngineStatus {
        gallery_dl_path: g_path,
        gallery_dl_version: g_ver,
        yt_dlp_path: yt_path,
        yt_dlp_version: yt_ver,
        ready,
    }
}

pub async fn download_binary(url: &str, target_path: &Path) -> Result<PathBuf, String> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create binary dir: {}", e))?;
    }

    let client = reqwest::Client::builder()
        .user_agent("omniget-open-nami/1.0")
        .build()
        .map_err(|e| format!("Client builder error: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request error from {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP status {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed reading response bytes: {}", e))?;

    let temp_download = target_path.with_extension("download");
    fs::write(&temp_download, bytes).map_err(|e| format!("Failed writing temp binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        let _ = fs::set_permissions(&temp_download, perms);
    }

    fs::rename(&temp_download, target_path).map_err(|e| format!("Failed to finalize binary rename: {}", e))?;

    Ok(target_path.to_path_buf())
}

pub async fn ensure_latest_gallery_dl(app_data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(existing) = resolve_gallery_dl(app_data_dir) {
        if get_binary_version(&existing).is_some() {
            return Ok(existing);
        }
    }

    let bin_dir = get_open_nami_bin_dir(app_data_dir);
    let target = bin_dir.join(if cfg!(target_os = "windows") { "gallery-dl.exe" } else { "gallery-dl" });
    let url = format!("{}/{}", GALLERYDL_GITHUB_FALLBACK, gallery_dl_asset_name());

    download_binary(&url, &target).await
}

pub async fn ensure_latest_yt_dlp(app_data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(existing) = resolve_yt_dlp(app_data_dir) {
        if get_binary_version(&existing).is_some() {
            return Ok(existing);
        }
    }

    let bin_dir = get_open_nami_bin_dir(app_data_dir);
    let target = bin_dir.join(if cfg!(target_os = "windows") { "yt-dlp.exe" } else { "yt-dlp" });
    let url = format!("{}/{}", YTDLP_RELEASE_BASE, yt_dlp_asset_name());

    download_binary(&url, &target).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_names() {
        let g_asset = gallery_dl_asset_name();
        let yt_asset = yt_dlp_asset_name();
        assert!(!g_asset.is_empty());
        assert!(!yt_asset.is_empty());
    }
}
