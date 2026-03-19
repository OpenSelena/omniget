use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::core::filename;

use super::api::{self, MedwayCourse, MedwaySession};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MedwayCourseDownloadProgress {
    pub course_id: i64,
    pub course_name: String,
    pub percent: f64,
    pub current_module: String,
    #[serde(rename = "current_page")]
    pub current_lesson: String,
    pub downloaded_bytes: u64,
    #[serde(rename = "total_pages")]
    pub total_lessons: u32,
    #[serde(rename = "completed_pages")]
    pub completed_lessons: u32,
    pub total_modules: u32,
    pub current_module_index: u32,
}

pub async fn download_full_course(
    app: &tauri::AppHandle,
    session: &MedwaySession,
    course: &MedwayCourse,
    output_dir: &str,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let modules = api::get_modules(session, course.id).await?;

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

    if crate::core::course_utils::is_course_complete(&course_dir) {
        tracing::info!("[medway] course already complete, skipping");
        return Ok(());
    }

    let total_lessons: usize = modules.iter().map(|m| m.lessons.len()).sum();
    let total_modules = modules.len();
    let total_bytes = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let _ = app.emit(
        "download-progress",
        &MedwayCourseDownloadProgress {
            course_id: course.id,
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
            if let Some(ref desc) = lesson.description {
                let lesson_desc_dir = format!("{}/{}. {}", mod_dir, li + 1, lesson_name);
                tokio::fs::create_dir_all(&lesson_desc_dir).await?;
                crate::core::course_utils::save_description(&lesson_desc_dir, desc, "html").await.ok();
            }

            let item_type = lesson.item_type.to_lowercase();

            if item_type.contains("v\u{00ed}deoaula") || item_type.contains("video") {
                if let Some(ref raw_url) = lesson.video_url {
                    if !raw_url.is_empty() {
                        let video_url = api::resolve_video_url(raw_url);
                        let video_path = format!(
                            "{}/{}. {}.mp4",
                            mod_dir,
                            li + 1,
                            lesson_name
                        );

                        if tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
                            let meta = tokio::fs::metadata(&video_path).await;
                            if meta.map(|m| m.len() > 0).unwrap_or(false) {
                                tracing::info!("[medway] Skipping existing: {}", video_path);
                                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                                let _ = app.emit(
                                    "download-progress",
                                    &MedwayCourseDownloadProgress {
                                        course_id: course.id,
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
                        }

                        match download_with_ytdlp(&video_url, &mod_dir, &cancel_token).await {
                            Ok(size) => {
                                total_bytes.fetch_add(size, Ordering::Relaxed);
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[medway] yt-dlp download failed for '{}': {}, trying direct download",
                                    lesson.name,
                                    e
                                );
                                match download_file_direct(&session.client, &video_url, &video_path, &cancel_token).await {
                                    Ok(size) => {
                                        total_bytes.fetch_add(size, Ordering::Relaxed);
                                    }
                                    Err(e2) => {
                                        tracing::error!(
                                            "[medway] Direct download also failed for '{}': {}",
                                            lesson.name,
                                            e2
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else if item_type.contains("documento") {
                if let Some(ref doc_id) = lesson.document_id {
                    if !doc_id.is_empty() {
                        match api::get_document_url(session, doc_id).await {
                            Ok((doc_url, doc_name)) => {
                                let ext = doc_url.rsplit('.').next().unwrap_or("pdf");
                                let ext = ext.split('?').next().unwrap_or(ext);
                                let safe_name = filename::sanitize_path_component(&doc_name);
                                let doc_path = format!("{}/{}. {}.{}", mod_dir, li + 1, safe_name, ext);

                                if tokio::fs::try_exists(&doc_path).await.unwrap_or(false) {
                                    let meta = tokio::fs::metadata(&doc_path).await;
                                    if meta.map(|m| m.len() > 0).unwrap_or(false) {
                                        let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                                        let _ = app.emit(
                                            "download-progress",
                                            &MedwayCourseDownloadProgress {
                                                course_id: course.id,
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
                                }

                                match download_file_direct(&session.client, &doc_url, &doc_path, &cancel_token).await {
                                    Ok(size) => {
                                        total_bytes.fetch_add(size, Ordering::Relaxed);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "[medway] Document download failed for '{}': {}",
                                            doc_name,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[medway] Failed to get document URL for '{}': {}",
                                    lesson.name,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app.emit(
                "download-progress",
                &MedwayCourseDownloadProgress {
                    course_id: course.id,
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

    crate::core::course_utils::mark_course_complete(&course_dir).await.ok();

    Ok(())
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
        Some("https://app.medway.com.br/"),
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
