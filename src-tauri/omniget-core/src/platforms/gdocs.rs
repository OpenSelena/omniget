use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::tools::{gdocs, gslides, ToolProgress};
use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::models::progress::ProgressUpdate;
use crate::platforms::traits::PlatformDownloader;

pub struct GdocsDownloader;

impl Default for GdocsDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GdocsDownloader {
    pub fn new() -> Self {
        Self
    }
}

async fn fetch_presentation_title(url: &str) -> Option<String> {
    let target = gslides::parse_gslides_url(url)?;
    let client = gslides::build_google_client(Some(url)).ok()?;
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        client.get(target.viewer_url()).send(),
    )
    .await
    .ok()?
    .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // Leer los primeros ~256 KB basta para extraer `<title>` o `viewerData.title`
    use futures::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::with_capacity(262_144);
    while let Some(Ok(chunk)) = stream.next().await {
        buf.extend_from_slice(&chunk);
        if buf.len() >= 262_144 {
            break;
        }
    }
    let html = String::from_utf8_lossy(&buf);
    let title = gslides::extract_title_from_viewer_html(&html);
    if title == "presentacion_google_slides" {
        None
    } else {
        Some(title)
    }
}

fn compute_path_size(path: &std::path::Path) -> u64 {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.is_file() {
            return meta.len();
        }
        if meta.is_dir() {
            return walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .filter(|m| m.is_file())
                .map(|m| m.len())
                .sum();
        }
    }
    0
}

#[async_trait]
impl PlatformDownloader for GdocsDownloader {
    fn name(&self) -> &str {
        "gdocs"
    }

    fn can_handle(&self, url: &str) -> bool {
        gdocs::parse(url).is_some()
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let parsed =
            gdocs::parse(url).ok_or_else(|| anyhow!("Enlace de Google Docs/Slides no válido"))?;

        let title = if parsed.kind == "presentation" {
            fetch_presentation_title(url).await.unwrap_or_else(|| {
                format!("Google Slides ({})", &parsed.id[..parsed.id.len().min(12)])
            })
        } else {
            format!(
                "Google {} ({})",
                parsed.kind,
                &parsed.id[..parsed.id.len().min(12)]
            )
        };

        let qualities: Vec<VideoQuality> = parsed
            .formats
            .iter()
            .map(|fmt| VideoQuality {
                label: fmt.to_uppercase(),
                width: if parsed.kind == "presentation" {
                    1280
                } else {
                    0
                },
                height: if parsed.kind == "presentation" {
                    720
                } else {
                    0
                },
                url: url.to_string(),
                format: fmt.clone(),
            })
            .collect();

        Ok(MediaInfo {
            title,
            author: "Google Docs".to_string(),
            platform: "gdocs".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: qualities,
            media_type: MediaType::File,
            file_size_bytes: None,
        })
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<ProgressUpdate>,
    ) -> anyhow::Result<DownloadResult> {
        let start = std::time::Instant::now();
        let _ = progress.send(ProgressUpdate::percent(1.0)).await;

        let target_url = opts
            .page_url
            .as_deref()
            .or_else(|| info.available_qualities.first().map(|q| q.url.as_str()))
            .ok_or_else(|| anyhow!("No hay URL disponible para Google Docs/Slides"))?;

        let parsed = gdocs::parse(target_url)
            .ok_or_else(|| anyhow!("Enlace de Google Docs/Slides no válido"))?;

        let requested_format = opts
            .format_id
            .as_deref()
            .or(opts.quality.as_deref())
            .map(|s| s.to_ascii_lowercase())
            .filter(|f| parsed.formats.iter().any(|pf| pf == f))
            .unwrap_or_else(|| "pdf".to_string());

        let dest_dir = opts.output_dir.to_string_lossy().to_string();
        let tx = progress.clone();
        let tool_progress: crate::core::tools::ProgressFn = Arc::new(move |tp: ToolProgress| {
            let pct = match (tp.stage.as_str(), tp.total) {
                ("fetch", _) => 5.0,
                ("images", Some(t)) if t > 0 => {
                    5.0 + ((tp.done as f64 / t as f64) * 45.0).clamp(0.0, 45.0)
                }
                ("compile", Some(t)) if t > 0 => {
                    50.0 + ((tp.done as f64 / t as f64) * 48.0).clamp(0.0, 48.0)
                }
                ("done", _) => 100.0,
                (_, Some(t)) if t > 0 => ((tp.done as f64 / t as f64) * 95.0).clamp(0.0, 95.0),
                _ => 10.0,
            };
            let _ = tx.try_send(ProgressUpdate::percent(pct));
        });

        let out_path_str =
            gdocs::download(target_url, &requested_format, &dest_dir, tool_progress).await?;
        let out_path = PathBuf::from(&out_path_str);
        let size = compute_path_size(&out_path);

        let _ = progress.send(ProgressUpdate::percent(100.0)).await;

        Ok(DownloadResult {
            file_path: out_path,
            file_size_bytes: size,
            duration_seconds: start.elapsed().as_secs_f64(),
            torrent_id: None,
        })
    }
}
