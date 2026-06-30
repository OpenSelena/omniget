use anyhow::Result;

use omniget_core::core::ytdlp;

use crate::output;
use crate::reporter::find_yt_dlp;

pub async fn execute(url: String) -> Result<()> {
    let ytdlp = find_yt_dlp().await?;

    let info = ytdlp::get_video_info(&ytdlp, &url, &[]).await?;
    let formats = ytdlp::parse_formats(&info);

    if output::is_json_mode() {
        output::print_json(&serde_json::json!({
            "url": url,
            "title": info.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "duration": info.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "uploader": info.get("uploader").and_then(|v| v.as_str()).unwrap_or(""),
            "view_count": info.get("view_count").and_then(|v| v.as_u64()),
            "thumbnail": info.get("thumbnail").and_then(|v| v.as_str()).unwrap_or(""),
            "formats": formats,
        }));
    } else {
        println!(
            "Title: {}",
            info.get("title").and_then(|v| v.as_str()).unwrap_or("")
        );
        println!(
            "Duration: {:.0}s",
            info.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0)
        );
        println!(
            "Uploader: {}",
            info.get("uploader").and_then(|v| v.as_str()).unwrap_or("")
        );
        println!("\nAvailable formats:");
        for f in &formats {
            let res = f.resolution.as_deref().unwrap_or("?");
            let note = f.format_note.as_deref().unwrap_or("");
            println!(
                "  {:<10} {:<8} {:>5} fps={:<4} {:<10} {}",
                f.format_id,
                f.ext,
                res,
                f.fps
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".to_string()),
                if f.has_audio { "audio" } else { "video-only" },
                note,
            );
        }
    }

    Ok(())
}
