use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use omniget_core::core::ytdlp;
use omniget_core::models::progress::ProgressUpdate;

use crate::output;
use crate::reporter::{self, CliReporter};

pub async fn execute(
    url: String,
    quality: Option<u32>,
    output_dir: Option<String>,
    audio_only: bool,
    _subs: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let reporter = Arc::new(CliReporter::new(output::is_json_mode()));

    // 1. Find yt-dlp binary
    reporter.message("Searching for yt-dlp...");
    let ytdlp = reporter::find_yt_dlp().await?;

    // 2. Determine output directory
    let output_path = match output_dir {
        Some(dir) => PathBuf::from(dir),
        None => reporter::default_output_dir(),
    };
    tokio::fs::create_dir_all(&output_path).await.ok();

    // 3. Set up progress channel
    let (tx, mut rx) = mpsc::channel::<ProgressUpdate>(100);
    let reporter_arc = reporter.clone();
    let progress_handle = tokio::spawn(async move {
        while let Some(update) = rx.recv().await {
            reporter_arc.update(&update);
        }
    });

    // 4. Determine download mode
    let download_mode = if audio_only { Some("audio") } else { None };

    // 5. Set up cancellation
    let cancel_token = CancellationToken::new();

    // 6. Execute download
    reporter.message(&format!("Starting download from: {}", url));
    reporter.message(&format!("Output: {}", output_path.display()));

    let result = ytdlp::download_video(
        &ytdlp,
        &url,
        &output_path,
        quality,
        tx.clone(),
        download_mode,
        format.as_deref(),
        None, // filename_template
        None, // referer
        cancel_token,
        reporter::default_cookie_path().as_deref(),
        4,     // concurrent_fragments
        false, // download_subtitles
        &[],   // extra_flags
        None,  // audio_format
    )
    .await;

    drop(tx);
    progress_handle.await.ok();

    match result {
        Ok(dl_result) => {
            reporter.finish(
                true,
                &format!("Downloaded to: {}", dl_result.file_path.display()),
            );
            if output::is_json_mode() {
                output::print_json(&serde_json::json!({
                    "success": true,
                    "file_path": dl_result.file_path,
                    "size": dl_result.file_size_bytes,
                    "duration": dl_result.duration_seconds,
                }));
            }
            Ok(())
        }
        Err(e) => {
            reporter.finish(false, &format!("Download failed: {}", e));
            if output::is_json_mode() {
                output::print_json(&serde_json::json!({
                    "success": false,
                    "error": format!("{}", e),
                }));
            }
            Err(e)
        }
    }
}
