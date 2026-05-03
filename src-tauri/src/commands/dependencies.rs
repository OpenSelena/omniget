use std::path::PathBuf;

use serde::Serialize;

use crate::core::{dependencies, pdfium};

#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyVariantInfo {
    pub id: String,
    pub label: String,
    pub recommended: bool,
}

#[tauri::command]
pub async fn check_dependencies() -> Result<Vec<DependencyStatus>, String> {
    let (ytdlp_version, ffmpeg_version) = tokio::join!(
        dependencies::check_version("yt-dlp"),
        dependencies::check_version("ffmpeg"),
    );

    let pdfium_installed = pdfium::is_installed();
    let pdfium_version = if pdfium_installed {
        Some(pdfium::read_version_marker().unwrap_or_else(|| "installed".to_string()))
    } else {
        None
    };

    Ok(vec![
        DependencyStatus {
            name: "yt-dlp".into(),
            installed: ytdlp_version.is_some(),
            version: ytdlp_version,
        },
        DependencyStatus {
            name: "FFmpeg".into(),
            installed: ffmpeg_version.is_some(),
            version: ffmpeg_version,
        },
        DependencyStatus {
            name: "PDFium".into(),
            installed: pdfium_installed,
            version: pdfium_version,
        },
    ])
}

#[tauri::command]
pub async fn check_ytdlp_available() -> Result<bool, String> {
    Ok(crate::core::ytdlp::find_ytdlp_cached().await.is_some())
}

#[tauri::command]
pub async fn install_dependency(
    name: String,
    variant: Option<String>,
) -> Result<String, String> {
    match name.as_str() {
        "yt-dlp" => {
            crate::core::ytdlp::ensure_ytdlp()
                .await
                .map_err(|e| e.to_string())?;
            crate::core::ytdlp::reset_ytdlp_cache();
        }
        "FFmpeg" => {
            dependencies::ensure_ffmpeg()
                .await
                .map_err(|e| e.to_string())?;
            crate::core::ytdlp::reset_ffmpeg_location_cache();
            crate::core::ffmpeg::reset_ffmpeg_available_cache();
        }
        "PDFium" => {
            let _path: PathBuf = pdfium::ensure_pdfium_with_variant(variant)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(pdfium::read_version_marker()
                .unwrap_or_else(|| "installed".to_string()));
        }
        _ => return Err(format!("Unknown dependency: {}", name)),
    }

    dependencies::check_version(match name.as_str() {
        "FFmpeg" => "ffmpeg",
        other => other,
    })
    .await
    .ok_or_else(|| "Installed but version check failed".into())
}

#[tauri::command]
pub fn dependency_variants(name: String) -> Result<Vec<DependencyVariantInfo>, String> {
    match name.as_str() {
        "PDFium" => Ok(pdfium::list_variants()
            .into_iter()
            .map(|v| DependencyVariantInfo {
                id: v.id,
                label: v.label,
                recommended: v.recommended,
            })
            .collect()),
        "yt-dlp" | "FFmpeg" => Ok(Vec::new()),
        _ => Err(format!("Unknown dependency: {}", name)),
    }
}

#[tauri::command]
pub fn dependency_install_dir(name: String) -> Result<String, String> {
    let dir = match name.as_str() {
        "PDFium" => pdfium::pdfium_target_dir()
            .ok_or_else(|| "could not determine plugin data dir".to_string())?,
        "yt-dlp" | "FFmpeg" => crate::core::paths::app_data_dir()
            .ok_or_else(|| "could not determine app data dir".to_string())?
            .join("bin"),
        _ => return Err(format!("Unknown dependency: {}", name)),
    };
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_dependency_path(name: String, source_path: String) -> Result<String, String> {
    let src = PathBuf::from(&source_path);
    match name.as_str() {
        "PDFium" => {
            pdfium::set_pdfium_from_path(&src).map_err(|e| e.to_string())?;
            Ok(pdfium::read_version_marker()
                .unwrap_or_else(|| "custom".to_string()))
        }
        _ => Err(format!(
            "Custom file path not supported for: {}",
            name
        )),
    }
}
