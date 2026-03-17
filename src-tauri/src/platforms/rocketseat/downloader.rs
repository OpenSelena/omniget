use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::core::filename;

use super::api::{self, RocketseatCourse, RocketseatSession};

#[derive(Debug, Clone, serde::Serialize)]
pub struct RocketseatCourseDownloadProgress {
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
    session: &RocketseatSession,
    course: &RocketseatCourse,
    output_dir: &str,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let modules = api::get_course_content(session, course).await?;

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
        "rocketseat-download-progress",
        &RocketseatCourseDownloadProgress {
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
            let safe_name = format!("{}. {}", li + 1, lesson_name);

            if let Some(ref vid_id) = lesson.video_id {
                let video_path = format!("{}/{}.mp4", mod_dir, safe_name);

                if tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
                    let meta = tokio::fs::metadata(&video_path).await;
                    if meta.map(|m| m.len() > 0).unwrap_or(false) {
                        tracing::info!("[rocketseat] Skipping existing: {}", video_path);
                        let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        let _ = app.emit(
                            "rocketseat-download-progress",
                            &RocketseatCourseDownloadProgress {
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
                }

                let embed_url = api::get_video_embed_url(vid_id);

                match download_with_ytdlp(&embed_url, &mod_dir, &cancel_token).await {
                    Ok(size) => {
                        total_bytes.fetch_add(size, Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::error!(
                            "[rocketseat] yt-dlp download failed for '{}': {}",
                            lesson.name,
                            e
                        );
                    }
                }
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app.emit(
                "rocketseat-download-progress",
                &RocketseatCourseDownloadProgress {
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
        Some("https://app.rocketseat.com.br/"),
        cancel_token.clone(),
        None,
        8,
        false,
        &[],
    )
    .await?;

    Ok(result.file_size_bytes)
}
