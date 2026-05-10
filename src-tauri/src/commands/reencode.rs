use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use omniget_core::core::ffmpeg;
use omniget_core::core::hwaccel;
use omniget_core::core::process;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReencodeCodec {
    Av1,
    Hevc,
    H264,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReencodeRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    pub codec: ReencodeCodec,
    pub cq: Option<u32>,
    pub replace_original: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReencodeResult {
    pub output_path: String,
    pub encoder_used: String,
    pub used_hwaccel: bool,
    pub original_size_bytes: u64,
    pub new_size_bytes: u64,
    pub savings_pct: f64,
    pub replaced_original: bool,
}

#[tauri::command]
pub async fn reencode_video(req: ReencodeRequest) -> Result<ReencodeResult, String> {
    if !ffmpeg::is_ffmpeg_available().await {
        return Err("ffmpeg not found".to_string());
    }

    let input = PathBuf::from(&req.input_path);
    if !input.is_file() {
        return Err(format!("source not found: {}", req.input_path));
    }
    let original_size = std::fs::metadata(&input)
        .map(|m| m.len())
        .unwrap_or(0);

    let output = req.output_path.clone().unwrap_or_else(|| {
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let parent = input
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let ext = match req.codec {
            ReencodeCodec::Av1 | ReencodeCodec::Hevc => "mp4",
            ReencodeCodec::H264 => "mp4",
        };
        parent
            .join(format!("{}.reencoded.{}", stem, ext))
            .to_string_lossy()
            .into_owned()
    });
    let output_path = PathBuf::from(&output);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {}", e))?;
    }

    let hw = hwaccel::detect_hwaccel().await;
    let (encoder, used_hw) = pick_encoder(&req.codec, &hw);

    let cq = req.cq.unwrap_or(match req.codec {
        ReencodeCodec::Av1 => 32,
        ReencodeCodec::Hevc => 28,
        ReencodeCodec::H264 => 22,
    });
    let cq_str = cq.to_string();
    let cq_label = if encoder.contains("nvenc")
        || encoder.contains("qsv")
        || encoder.contains("amf")
        || encoder.contains("videotoolbox")
    {
        "-cq"
    } else {
        "-crf"
    };

    let status = process::command("ffmpeg")
        .args([
            "-y",
            "-hwaccel",
            "auto",
            "-i",
            input.to_string_lossy().as_ref(),
            "-c:v",
            &encoder,
            cq_label,
            &cq_str,
            "-c:a",
            "copy",
            "-movflags",
            "+faststart",
            output_path.to_string_lossy().as_ref(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("spawn ffmpeg failed: {}", e))?;

    if !status.success() {
        return Err(format!(
            "ffmpeg reencode failed (exit {}). encoder={}",
            status, encoder
        ));
    }
    if !output_path.is_file() {
        return Err("output file missing after ffmpeg".to_string());
    }

    let new_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let savings_pct = if original_size > 0 {
        100.0 * (original_size as f64 - new_size as f64) / original_size as f64
    } else {
        0.0
    };

    let replace = req.replace_original.unwrap_or(false);
    let mut replaced = false;
    if replace && new_size > 0 && new_size < original_size {
        let backup = backup_path(&input);
        std::fs::rename(&input, &backup)
            .map_err(|e| format!("rename original→backup failed: {}", e))?;
        if let Err(e) = std::fs::rename(&output_path, &input) {
            let _ = std::fs::rename(&backup, &input);
            return Err(format!("rename new→original failed: {}", e));
        }
        let _ = std::fs::remove_file(&backup);
        replaced = true;
    }

    let final_path = if replaced {
        input.clone()
    } else {
        output_path.clone()
    };

    Ok(ReencodeResult {
        output_path: final_path.to_string_lossy().into_owned(),
        encoder_used: encoder,
        used_hwaccel: used_hw,
        original_size_bytes: original_size,
        new_size_bytes: new_size,
        savings_pct,
        replaced_original: replaced,
    })
}

fn pick_encoder(codec: &ReencodeCodec, hw: &hwaccel::HwAccelInfo) -> (String, bool) {
    let candidates: &[(&str, ReencodeCodec)] = match codec {
        ReencodeCodec::Av1 => &[
            ("av1_nvenc", ReencodeCodec::Av1),
            ("av1_qsv", ReencodeCodec::Av1),
            ("av1_amf", ReencodeCodec::Av1),
            ("libsvtav1", ReencodeCodec::Av1),
        ],
        ReencodeCodec::Hevc => &[
            ("hevc_nvenc", ReencodeCodec::Hevc),
            ("hevc_qsv", ReencodeCodec::Hevc),
            ("hevc_amf", ReencodeCodec::Hevc),
            ("hevc_videotoolbox", ReencodeCodec::Hevc),
            ("hevc_vaapi", ReencodeCodec::Hevc),
            ("libx265", ReencodeCodec::Hevc),
        ],
        ReencodeCodec::H264 => &[
            ("h264_nvenc", ReencodeCodec::H264),
            ("h264_qsv", ReencodeCodec::H264),
            ("h264_amf", ReencodeCodec::H264),
            ("h264_videotoolbox", ReencodeCodec::H264),
            ("h264_vaapi", ReencodeCodec::H264),
            ("libx264", ReencodeCodec::H264),
        ],
    };

    for (enc, _) in candidates {
        let is_hw = !enc.starts_with("lib");
        let available = if is_hw {
            hw.encoders.iter().any(|e| e == enc)
        } else {
            true
        };
        if available {
            return (enc.to_string(), is_hw);
        }
    }
    let fallback = match codec {
        ReencodeCodec::Av1 => "libsvtav1",
        ReencodeCodec::Hevc => "libx265",
        ReencodeCodec::H264 => "libx264",
    };
    (fallback.to_string(), false)
}

fn backup_path(p: &Path) -> PathBuf {
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    if ext.is_empty() {
        parent.join(format!("{}.backup", stem))
    } else {
        parent.join(format!("{}.backup.{}", stem, ext))
    }
}
