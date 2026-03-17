use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::core::filename;

use super::api::{self, EstrategiaLdiCourse, EstrategiaLdiSession};

#[derive(Debug, Clone, serde::Serialize)]
pub struct EstrategiaLdiCourseDownloadProgress {
    pub course_id: String,
    pub course_name: String,
    pub percent: f64,
    pub current_module: String,
    pub current_lesson: String,
    pub downloaded_bytes: u64,
    pub total_lessons: u32,
    pub completed_lessons: u32,
    pub total_modules: u32,
    pub current_module_index: u32,
}

pub async fn download_full_course(
    app: &tauri::AppHandle,
    session: &EstrategiaLdiSession,
    course: &EstrategiaLdiCourse,
    output_dir: &str,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let modules = api::get_course_content(session, &course.id).await?;

    if modules.is_empty() {
        return Err(anyhow!(
            "'{}' has no modules available for download",
            course.name
        ));
    }

    let course_dir = format!(
        "{}/{}",
        output_dir,
        filename::sanitize_path_component(&course.name)
    );
    tokio::fs::create_dir_all(&course_dir).await?;

    let total_lessons: usize = modules.iter().map(|m| m.lessons.len()).sum();
    let total_modules = modules.len();
    let total_bytes = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let _ = app.emit(
        "estrategia-ldi-download-progress",
        &EstrategiaLdiCourseDownloadProgress {
            course_id: course.id.clone(),
            course_name: course.name.clone(),
            percent: 0.0,
            current_module: "Starting...".to_string(),
            current_lesson: String::new(),
            downloaded_bytes: 0,
            total_lessons: total_lessons as u32,
            completed_lessons: 0,
            total_modules: total_modules as u32,
            current_module_index: 0,
        },
    );

    for (mi, module) in modules.iter().enumerate() {
        if cancel_token.is_cancelled() {
            return Err(anyhow!("Download cancelled by user"));
        }

        let mod_name = filename::sanitize_path_component(&module.name);
        let mod_dir = format!("{}/{}. {}", course_dir, mi + 1, mod_name);
        tokio::fs::create_dir_all(&mod_dir).await?;

        for (li, lesson) in module.lessons.iter().enumerate() {
            if cancel_token.is_cancelled() {
                return Err(anyhow!("Download cancelled by user"));
            }

            let lesson_name = filename::sanitize_path_component(&lesson.name);

            let item_detail = match api::get_item_detail(session, &lesson.id).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!(
                        "[estrategia_ldi] Failed to get item detail '{}': {}. Continuing...",
                        lesson.name,
                        e
                    );
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = app.emit(
                        "estrategia-ldi-download-progress",
                        &EstrategiaLdiCourseDownloadProgress {
                            course_id: course.id.clone(),
                            course_name: course.name.clone(),
                            percent: done as f64 / total_lessons as f64 * 100.0,
                            current_module: module.name.clone(),
                            current_lesson: lesson.name.clone(),
                            downloaded_bytes: total_bytes.load(Ordering::Relaxed),
                            total_lessons: total_lessons as u32,
                            completed_lessons: done as u32,
                            total_modules: total_modules as u32,
                            current_module_index: (mi + 1) as u32,
                        },
                    );
                    continue;
                }
            };

            let track_ids = api::extract_track_ids(&item_detail);

            for (ti, track_id) in track_ids.iter().enumerate() {
                if cancel_token.is_cancelled() {
                    return Err(anyhow!("Download cancelled by user"));
                }

                let track_info = match api::get_track_info(session, track_id).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!(
                            "[estrategia_ldi] Failed to get track '{}': {}",
                            track_id,
                            e
                        );
                        continue;
                    }
                };

                let suffix = if track_ids.len() > 1 {
                    format!(" ({})", ti + 1)
                } else {
                    String::new()
                };

                let video_path = format!(
                    "{}/{}. {}{}.mp4",
                    mod_dir,
                    li + 1,
                    lesson_name,
                    suffix
                );

                if tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
                    let meta = tokio::fs::metadata(&video_path).await;
                    if meta.map(|m| m.len() > 0).unwrap_or(false) {
                        tracing::info!("[estrategia_ldi] Skipping existing: {}", video_path);
                        continue;
                    }
                }

                if track_info.url.contains(".m3u8") {
                    match download_hls_video(&track_info.url, &video_path, &cancel_token).await {
                        Ok(size) => {
                            total_bytes.fetch_add(size, Ordering::Relaxed);
                        }
                        Err(e) => {
                            tracing::error!(
                                "[estrategia_ldi] HLS download failed for '{}': {}",
                                lesson.name,
                                e
                            );
                            let _ = tokio::fs::remove_file(&video_path).await;
                        }
                    }
                } else {
                    match download_with_ytdlp(&track_info.url, &mod_dir, &cancel_token).await {
                        Ok(size) => {
                            total_bytes.fetch_add(size, Ordering::Relaxed);
                        }
                        Err(e) => {
                            tracing::error!(
                                "[estrategia_ldi] yt-dlp failed for '{}': {}, trying direct",
                                lesson.name,
                                e
                            );
                            match download_file_direct(&session.client, &track_info.url, &video_path, &cancel_token).await {
                                Ok(size) => {
                                    total_bytes.fetch_add(size, Ordering::Relaxed);
                                }
                                Err(e2) => {
                                    tracing::error!(
                                        "[estrategia_ldi] Direct download also failed for '{}': {}",
                                        lesson.name,
                                        e2
                                    );
                                }
                            }
                        }
                    }
                }
            }

            let attachments = api::extract_attachment_urls(&item_detail);
            for (att_name, att_url) in &attachments {
                if cancel_token.is_cancelled() {
                    return Err(anyhow!("Download cancelled by user"));
                }

                let safe_name = filename::sanitize_path_component(att_name);
                let ext = att_url.rsplit('.').next().unwrap_or("pdf");
                let ext = ext.split('?').next().unwrap_or(ext);
                let att_path = format!("{}/{}.{}", mod_dir, safe_name, ext);

                if tokio::fs::try_exists(&att_path).await.unwrap_or(false) {
                    let meta = tokio::fs::metadata(&att_path).await;
                    if meta.map(|m| m.len() > 0).unwrap_or(false) {
                        continue;
                    }
                }

                match download_file_direct(&session.client, att_url, &att_path, &cancel_token).await {
                    Ok(size) => {
                        total_bytes.fetch_add(size, Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::error!(
                            "[estrategia_ldi] Attachment download failed for '{}': {}",
                            att_name,
                            e
                        );
                    }
                }
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app.emit(
                "estrategia-ldi-download-progress",
                &EstrategiaLdiCourseDownloadProgress {
                    course_id: course.id.clone(),
                    course_name: course.name.clone(),
                    percent: done as f64 / total_lessons as f64 * 100.0,
                    current_module: module.name.clone(),
                    current_lesson: lesson.name.clone(),
                    downloaded_bytes: total_bytes.load(Ordering::Relaxed),
                    total_lessons: total_lessons as u32,
                    completed_lessons: done as u32,
                    total_modules: total_modules as u32,
                    current_module_index: (mi + 1) as u32,
                },
            );
        }
    }

    if cancel_token.is_cancelled() {
        return Err(anyhow!("Download cancelled by user"));
    }

    Ok(())
}

async fn download_hls_video(
    m3u8_url: &str,
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    use crate::core::media_processor::MediaProcessor;

    let _result = MediaProcessor::download_hls(
        m3u8_url,
        output_path,
        "https://concursos.estrategia.com/",
        None,
        cancel_token.clone(),
        20,
        3,
        None,
    )
    .await?;

    let meta = tokio::fs::metadata(output_path).await?;
    Ok(meta.len())
}

async fn download_with_ytdlp(
    url: &str,
    output_dir: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let ytdlp_path = crate::core::ytdlp::ensure_ytdlp().await?;
    let out_dir = Path::new(output_dir);
    let (tx, _rx) = mpsc::channel(8);

    let result = crate::core::ytdlp::download_video(
        &ytdlp_path,
        url,
        out_dir,
        None,
        tx,
        None,
        None,
        None,
        Some("https://concursos.estrategia.com/"),
        cancel_token.clone(),
        None,
        8,
        false,
        &[],
    )
    .await?;

    Ok(result.file_size_bytes)
}

async fn download_file_direct(
    client: &reqwest::Client,
    url: &str,
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let out = Path::new(output_path);
    let (tx, _rx) = mpsc::channel(8);

    crate::core::direct_downloader::download_direct(
        client,
        url,
        out,
        tx,
        Some(cancel_token),
    )
    .await
}
