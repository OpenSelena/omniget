use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use omniget_core::core::ffmpeg;
use omniget_core::core::process;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipRequest {
    pub source_path: String,
    pub start_secs: f64,
    pub duration_secs: f64,
    pub dest_dir: Option<String>,
    pub label: Option<String>,
    pub reencode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipResult {
    pub output_path: String,
    pub duration_secs: f64,
    pub size_bytes: u64,
    pub used_reencode: bool,
}

#[tauri::command]
pub async fn clip_video(_app: AppHandle, req: ClipRequest) -> Result<ClipResult, String> {
    if !ffmpeg::is_ffmpeg_available().await {
        return Err("ffmpeg not found".to_string());
    }

    let source = PathBuf::from(&req.source_path);
    if !source.is_file() {
        return Err(format!("source not found: {}", req.source_path));
    }

    let duration = req.duration_secs.max(0.5);
    let start = req.start_secs.max(0.0);

    let dest_dir = match req.dest_dir.as_deref() {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => default_dest_dir(&source),
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("mkdir failed: {}", e))?;

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("mp4");
    let label = sanitize(&req.label.clone().unwrap_or_else(|| "clip".to_string()));
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = dest_dir.join(format!("{}.{}.{}.{}", stem, label, ts, ext));

    let want_reencode = req.reencode.unwrap_or(false);

    if !want_reencode {
        let copy_result = run_copy_clip(&source, start, duration, &dest).await;
        if copy_result.is_ok() && dest.is_file() && file_size(&dest) > 1024 {
            return Ok(ClipResult {
                output_path: dest.to_string_lossy().into_owned(),
                duration_secs: duration,
                size_bytes: file_size(&dest),
                used_reencode: false,
            });
        }
    }

    run_reencode_clip(&source, start, duration, &dest).await?;
    if !dest.is_file() {
        return Err("clip output missing after reencode".to_string());
    }

    Ok(ClipResult {
        output_path: dest.to_string_lossy().into_owned(),
        duration_secs: duration,
        size_bytes: file_size(&dest),
        used_reencode: true,
    })
}

fn default_dest_dir(source: &Path) -> PathBuf {
    if let Some(parent) = source.parent() {
        return parent.join("clips");
    }
    PathBuf::from("clips")
}

fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c == ' ' {
            out.push('_');
        }
    }
    if out.is_empty() {
        return "clip".into();
    }
    out
}

fn file_size(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

async fn run_copy_clip(
    source: &Path,
    start: f64,
    duration: f64,
    dest: &Path,
) -> Result<(), String> {
    let status = process::command("ffmpeg")
        .args([
            "-y",
            "-ss",
            &format!("{:.3}", start),
            "-i",
            &source.to_string_lossy(),
            "-t",
            &format!("{:.3}", duration),
            "-c",
            "copy",
            "-avoid_negative_ts",
            "make_zero",
            "-movflags",
            "+faststart",
            &dest.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("spawn ffmpeg failed: {}", e))?;
    if !status.success() {
        return Err(format!("ffmpeg copy clip failed (exit {})", status));
    }
    Ok(())
}

async fn run_reencode_clip(
    source: &Path,
    start: f64,
    duration: f64,
    dest: &Path,
) -> Result<(), String> {
    let status = process::command("ffmpeg")
        .args([
            "-y",
            "-ss",
            &format!("{:.3}", start),
            "-i",
            &source.to_string_lossy(),
            "-t",
            &format!("{:.3}", duration),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "20",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "+faststart",
            &dest.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("spawn ffmpeg failed: {}", e))?;
    if !status.success() {
        return Err(format!("ffmpeg reencode clip failed (exit {})", status));
    }
    Ok(())
}
