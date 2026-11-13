use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::Serialize;

const PDFIUM_DOWNLOAD_BASE: &str =
    "https://github.com/bblanchon/pdfium-binaries/releases/latest/download";

#[derive(Debug, Clone, Serialize)]
pub struct DependencyVariant {
    pub id: String,
    pub label: String,
    pub recommended: bool,
}

pub fn pdfium_lib_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    }
}

fn default_archive_id() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "win-x64";
    }
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        return "win-x86";
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return "win-arm64";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "mac-x64";
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "mac-arm64";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "linux-x64";
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "linux-arm64";
    }
    #[allow(unreachable_code)]
    "win-x64"
}

pub fn list_variants() -> Vec<DependencyVariant> {
    let default = default_archive_id();
    let make = |id: &str, label: &str| DependencyVariant {
        id: id.to_string(),
        label: label.to_string(),
        recommended: id == default,
    };

    if cfg!(target_os = "windows") {
        vec![
            make("win-x64", "Windows x64 (64-bit)"),
            make("win-x64-v8", "Windows x64 + V8 (com JS, maior)"),
            make("win-x86", "Windows x86 (32-bit)"),
            make("win-arm64", "Windows ARM64"),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            make("mac-x64", "macOS x64 (Intel)"),
            make("mac-arm64", "macOS ARM64 (Apple Silicon)"),
            make("mac-univ", "macOS universal"),
        ]
    } else {
        vec![
            make("linux-x64", "Linux x64"),
            make("linux-arm64", "Linux ARM64"),
            make("linux-arm", "Linux ARM (32-bit)"),
            make("linux-musl-x64", "Linux musl x64 (Alpine)"),
            make("linux-musl-arm64", "Linux musl ARM64"),
        ]
    }
}

fn archive_name_for_variant(variant: Option<&str>) -> String {
    let id = variant
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "auto")
        .unwrap_or(default_archive_id());
    if let Some(rest) = id.strip_prefix("win-x64-v8") {
        let _ = rest;
        return "pdfium-v8-win-x64.tgz".to_string();
    }
    format!("pdfium-{}.tgz", id)
}

pub fn pdfium_target_dir() -> Option<PathBuf> {
    let app_data = crate::core::paths::app_data_dir()?;
    Some(app_data.join("plugins").join("study").join("data"))
}

pub fn pdfium_target_path() -> Option<PathBuf> {
    pdfium_target_dir().map(|d| d.join(pdfium_lib_filename()))
}

pub fn pdfium_version_marker_path() -> Option<PathBuf> {
    pdfium_target_dir().map(|d| d.join("pdfium.version"))
}

/// The PDFium library actually in use, and where it came from: `"custom"` for
/// a file the user pointed at, `"managed"` for the one OmniGet downloaded.
///
/// Issue #222: yt-dlp and FFmpeg have honoured the user's own binary since
/// the feature shipped, through `find_tool_with_source`. PDFium never did —
/// only the managed folder was ever checked — so a linked library was
/// recorded and displayed while the status stayed "missing".
pub fn resolve_with_source() -> Option<(PathBuf, &'static str)> {
    if let Some(custom) = crate::core::binary_overrides::get("PDFium") {
        if custom.is_file() {
            return Some((custom, "custom"));
        }
    }
    let managed = pdfium_target_path()?;
    if managed.is_file() {
        return Some((managed, "managed"));
    }
    None
}

/// Path to the PDFium library to load, honouring a custom override.
pub fn resolve_path() -> Option<PathBuf> {
    resolve_with_source().map(|(p, _)| p)
}

pub fn is_installed() -> bool {
    resolve_with_source().is_some()
}

pub fn parse_pdfium_version(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if let Some(build) = line.strip_prefix("BUILD=") {
            let build = build.trim();
            if !build.is_empty() {
                return Some(build.to_string());
            }
        }
    }
    None
}

pub fn clean_version_marker(raw: &str) -> String {
    if let Some(parsed) = parse_pdfium_version(raw) {
        return parsed;
    }
    if let Some(pos) = raw.find("BUILD=") {
        let slice = &raw[pos + 6..];
        let digits: String = slice.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits;
        }
    }
    let trimmed = raw.trim();
    if let Some((base, _)) = trimmed.split_once('(') {
        let base = base.trim();
        if !base.is_empty() {
            return base.to_string();
        }
    }
    trimmed.to_string()
}

pub fn read_version_marker() -> Option<String> {
    let p = pdfium_version_marker_path()?;
    let s = std::fs::read_to_string(&p).ok()?;
    let cleaned = clean_version_marker(&s);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

pub async fn ensure_pdfium() -> anyhow::Result<PathBuf> {
    ensure_pdfium_with_variant(None).await
}

pub async fn ensure_pdfium_with_variant(variant: Option<String>) -> anyhow::Result<PathBuf> {
    let target_dir =
        pdfium_target_dir().ok_or_else(|| anyhow!("could not determine app data dir"))?;
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating pdfium target dir {}", target_dir.display()))?;

    let archive_name = archive_name_for_variant(variant.as_deref());
    let url = format!("{}/{}", PDFIUM_DOWNLOAD_BASE, archive_name);
    let lib_filename = pdfium_lib_filename();
    let target_path = target_dir.join(lib_filename);

    tracing::info!("Downloading pdfium from {}", url);

    let client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download pdfium from {}: HTTP {}",
            url,
            response.status()
        ));
    }
    let bytes = response.bytes().await?.to_vec();
    if bytes.len() < 100_000 {
        return Err(anyhow!(
            "Downloaded pdfium archive is too small ({} bytes) — likely an error page",
            bytes.len()
        ));
    }

    let target_path_clone = target_path.clone();
    let target_dir_clone = target_dir.clone();
    let lib_name = lib_filename.to_string();
    let extracted_version =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
            extract_pdfium_archive(&bytes, &target_path_clone, &target_dir_clone, &lib_name)
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {}", e))??;

    if let Some(version_marker) = pdfium_version_marker_path() {
        let base = extracted_version.unwrap_or_else(|| "latest".to_string());
        let clean = clean_version_marker(&base);
        let value = format!("{} ({})", clean, archive_name);
        let _ = std::fs::write(&version_marker, value);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = tokio::task::spawn_blocking({
            let p = target_path.clone();
            move || {
                crate::core::process::std_command("xattr")
                    .args(["-d", "com.apple.quarantine"])
                    .arg(&p)
                    .output()
            }
        })
        .await;
    }

    Ok(target_path)
}

pub fn set_pdfium_from_path(source: &Path) -> anyhow::Result<PathBuf> {
    if !source.is_file() {
        return Err(anyhow!("source not a file: {}", source.display()));
    }
    let target_dir =
        pdfium_target_dir().ok_or_else(|| anyhow!("could not determine app data dir"))?;
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating pdfium target dir {}", target_dir.display()))?;
    let lib_filename = pdfium_lib_filename();
    let target_path = target_dir.join(lib_filename);

    let tmp = target_dir.join(format!(".{}.tmp", lib_filename));
    std::fs::copy(source, &tmp)
        .with_context(|| format!("copying {} → {}", source.display(), tmp.display()))?;
    if target_path.exists() {
        let _ = std::fs::remove_file(&target_path);
    }
    std::fs::rename(&tmp, &target_path)
        .with_context(|| format!("renaming temp to {}", target_path.display()))?;

    if let Some(version_marker) = pdfium_version_marker_path() {
        let label = source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("custom");
        let value = format!("custom ({})", label);
        let _ = std::fs::write(&version_marker, value);
    }
    Ok(target_path)
}

fn extract_pdfium_archive(
    bytes: &[u8],
    target_path: &std::path::Path,
    target_dir: &std::path::Path,
    lib_filename: &str,
) -> anyhow::Result<Option<String>> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut tar_archive = tar::Archive::new(gz);

    let mut found_lib = false;
    let mut version: Option<String> = None;

    for entry in tar_archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if file_name == lib_filename {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            let tmp = target_dir.join(format!(".{}.tmp", lib_filename));
            std::fs::write(&tmp, &buf)
                .with_context(|| format!("writing temp pdfium lib {}", tmp.display()))?;
            if target_path.exists() {
                let _ = std::fs::remove_file(target_path);
            }
            std::fs::rename(&tmp, target_path)
                .with_context(|| format!("moving pdfium to {}", target_path.display()))?;
            found_lib = true;
        } else if file_name == "VERSION" {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            version = parse_pdfium_version(&buf).or_else(|| {
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });
        }
    }

    if !found_lib {
        return Err(anyhow!("{} not found inside pdfium archive", lib_filename));
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pdfium_version_string() {
        let raw_version_file = "MAJOR=156\nMINOR=0\nBUILD=8076\nPATCH=0\n";
        assert_eq!(
            parse_pdfium_version(raw_version_file),
            Some("8076".to_string())
        );

        let legacy_marker = "MAJOR=156\nMINOR=0\nBUILD=8076\nPATCH=0 (pdfium-win-x64.tgz)";
        assert_eq!(clean_version_marker(legacy_marker), "8076");

        let clean_marker = "8076";
        assert_eq!(clean_version_marker(clean_marker), "8076");

        let archive_marker = "8076 (pdfium-win-x64.tgz)";
        assert_eq!(clean_version_marker(archive_marker), "8076");
    }
}
