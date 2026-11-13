use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use omniget_core::core::filename;

use super::api::{self, KiwifyCourse, KiwifySession};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum LessonStatus {
    Saved,
    Unavailable,
    Failed,
}

#[derive(Debug, serde::Serialize)]
struct LessonReport {
    module: String,
    lesson: String,
    status: LessonStatus,
    reason: Option<String>,
}

fn report_item(
    module: &str,
    lesson: &str,
    status: LessonStatus,
    reason: Option<&str>,
) -> LessonReport {
    LessonReport {
        module: module.into(),
        lesson: lesson.into(),
        status,
        reason: reason.map(String::from),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KiwifyCourseDownloadProgress {
    pub course_id: String,
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
    pub failed_items: u32,
}

pub async fn download_full_course(
    app: &tauri::AppHandle,
    session: &KiwifySession,
    course: &KiwifyCourse,
    output_dir: &str,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    download_course(session, course, output_dir, cancel_token, |progress| {
        let _ = app.emit("download-progress", progress);
    })
    .await
}

/// Shared by the desktop command and the opt-in live integration example.
pub async fn download_course(
    session: &KiwifySession,
    course: &KiwifyCourse,
    output_dir: &str,
    cancel_token: CancellationToken,
    mut on_progress: impl FnMut(&KiwifyCourseDownloadProgress),
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
    match tokio::fs::remove_file(Path::new(&course_dir).join(".complete")).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    // Recheck the catalog even when an earlier run wrote a completion marker:
    // new lessons and changed permissions must not be hidden by a stale marker.
    let discovered: usize = modules.iter().map(|m| m.lessons.len()).sum();
    let total_lessons = modules
        .iter()
        .flat_map(|m| &m.lessons)
        .filter(|l| l.available)
        .count();
    tracing::info!(
        "[kiwify] catalog: {} modules, {} contents, {} candidates, {} catalog-restricted",
        modules.len(),
        discovered,
        total_lessons,
        discovered - total_lessons
    );
    let mut report = Vec::new();
    let media_client = api::build_media_client()?;
    let mut failures = 0usize;
    let total_modules = modules.len();
    let total_bytes = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    on_progress(&KiwifyCourseDownloadProgress {
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
        failed_items: 0,
    });

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
            if !lesson.available {
                report.push(report_item(
                    &module.name,
                    &lesson.name,
                    LessonStatus::Unavailable,
                    Some("catalog_access_restricted"),
                ));
                continue;
            }

            let lesson_name = filename::sanitize_path_component(&lesson.name);
            let lesson_dir = format!("{}/{}. {}", mod_dir, li + 1, lesson_name);

            let lesson_course_id = lesson.course_id.as_deref().unwrap_or(&course.id);
            let detail = match api::get_lesson_detail(session, lesson_course_id, &lesson.id).await {
                Ok(d) => d,
                Err(e) => {
                    failures += 1;
                    report.push(report_item(
                        &module.name,
                        &lesson.name,
                        LessonStatus::Failed,
                        Some("lesson_detail_request_failed"),
                    ));
                    tracing::error!(
                        "[kiwify] Failed to get lesson detail '{}': {}. Continuing...",
                        lesson.name,
                        e
                    );
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    on_progress(&KiwifyCourseDownloadProgress {
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
                        failed_items: failures as u32,
                    });
                    continue;
                }
            };

            if detail.available {
                // Publication metadata alone is not an access decision. The server's
                // lesson response must authorize the actual content before saving it.
                tokio::fs::create_dir_all(&lesson_dir).await?;
                let failures_before = failures;

                if detail.video_url.is_none()
                    && detail.files.is_empty()
                    && detail.description.is_none()
                {
                    failures += 1;
                }

                if let Some(ref desc) = detail.description {
                    if omniget_core::core::course_utils::save_description(&lesson_dir, desc, "html")
                        .await
                        .is_err()
                    {
                        failures += 1;
                    }
                }

                if let Some(ref video_url) = detail.video_url {
                    let video_ext = if video_url.contains(".m3u8") {
                        "mp4"
                    } else {
                        video_url
                            .rsplit('.')
                            .next()
                            .and_then(|e| {
                                let e = e.split('?').next().unwrap_or(e);
                                if e.len() <= 4 {
                                    Some(e)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or("mp4")
                    };

                    let video_path =
                        format!("{}/{}. {}.{}", lesson_dir, li + 1, lesson_name, video_ext);

                    if nonempty_file(&video_path).await {
                        // Attachments below still need to run when the video already exists.
                        tracing::debug!("[kiwify] reusing existing video");
                    } else if video_url.contains(".m3u8") {
                        match download_hls_video(video_url, &video_path, &cancel_token).await {
                            Ok(size) => {
                                total_bytes.fetch_add(size, Ordering::Relaxed);
                            }
                            Err(e) => {
                                failures += 1;
                                tracing::error!(
                                    "[kiwify] HLS download failed for '{}': {}",
                                    lesson.name,
                                    e
                                );
                                let _ = tokio::fs::remove_file(&video_path).await;
                            }
                        }
                    } else {
                        match download_with_ytdlp(video_url, &video_path, &cancel_token).await {
                            Ok(size) => {
                                total_bytes.fetch_add(size, Ordering::Relaxed);
                            }
                            Err(e) => {
                                tracing::error!(
                                "[kiwify] yt-dlp download failed for '{}': {}, trying direct download",
                                lesson.name,
                                e
                            );
                                match download_file_direct(
                                    &media_client,
                                    video_url,
                                    &video_path,
                                    &cancel_token,
                                )
                                .await
                                {
                                    Ok(size) => {
                                        total_bytes.fetch_add(size, Ordering::Relaxed);
                                    }
                                    Err(e2) => {
                                        failures += 1;
                                        tracing::error!(
                                            "[kiwify] Direct download also failed for '{}': {}",
                                            lesson.name,
                                            e2
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                for file in &detail.files {
                    if cancel_token.is_cancelled() {
                        return Err(anyhow!("Download cancelled by user"));
                    }

                    let file_name = filename::sanitize_path_component(&file.name);
                    let file_path = format!("{}/{}", lesson_dir, file_name);

                    if tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
                        let meta = tokio::fs::metadata(&file_path).await;
                        if meta.map(|m| m.len() > 0).unwrap_or(false) {
                            continue;
                        }
                    }

                    let download_url = if let Some(ref url) = file.url {
                        url.clone()
                    } else {
                        match api::get_file_download_url(session, lesson_course_id, &file.id).await
                        {
                            Ok(url) => url,
                            Err(e) => {
                                failures += 1;
                                tracing::error!(
                                    "[kiwify] Failed to get download URL for file '{}': {}",
                                    file.name,
                                    e
                                );
                                continue;
                            }
                        }
                    };

                    match download_file_direct(
                        &media_client,
                        &download_url,
                        &file_path,
                        &cancel_token,
                    )
                    .await
                    {
                        Ok(size) => {
                            total_bytes.fetch_add(size, Ordering::Relaxed);
                        }
                        Err(e) => {
                            failures += 1;
                            tracing::error!(
                                "[kiwify] File download failed for '{}': {}",
                                file.name,
                                e
                            );
                        }
                    }
                }

                report.push(report_item(
                    &module.name,
                    &lesson.name,
                    if failures == failures_before {
                        LessonStatus::Saved
                    } else {
                        LessonStatus::Failed
                    },
                    if failures == failures_before {
                        None
                    } else {
                        Some("content_save_failed")
                    },
                ));
            } else {
                report.push(report_item(
                    &module.name,
                    &lesson.name,
                    LessonStatus::Unavailable,
                    Some("lesson_locked_expired_or_access_restricted"),
                ));
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            on_progress(&KiwifyCourseDownloadProgress {
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
                failed_items: failures as u32,
            });
        }
    }

    if cancel_token.is_cancelled() {
        return Err(anyhow!("Download cancelled by user"));
    }

    finish_course(&course_dir, discovered, &report).await?;

    Ok(())
}

async fn nonempty_file(path: &str) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

async fn finish_course(
    course_dir: &str,
    total_contents: usize,
    items: &[LessonReport],
) -> anyhow::Result<()> {
    let saved = items
        .iter()
        .filter(|i| i.status == LessonStatus::Saved)
        .count();
    let unavailable = items
        .iter()
        .filter(|i| i.status == LessonStatus::Unavailable)
        .count();
    let failed = items
        .iter()
        .filter(|i| i.status == LessonStatus::Failed)
        .count();
    let not_processed = total_contents.saturating_sub(items.len());
    let complete = total_contents > 0
        && saved == total_contents
        && unavailable == 0
        && failed == 0
        && not_processed == 0;
    let summary = serde_json::json!({
        "schema_version": 1, "complete": complete, "total_contents": total_contents,
        "saved_contents": saved, "unavailable_contents": unavailable,
        "failed_contents": failed, "not_processed_contents": not_processed, "items": items,
    });
    let root = Path::new(course_dir);
    tokio::fs::write(
        root.join("download-report.json"),
        serde_json::to_vec_pretty(&summary)?,
    )
    .await?;
    let mut text = format!("Kiwify download report\nTotal contents: {}\nSaved: {}\nUnavailable: {}\nFailed: {}\nNot processed: {}\n\n",total_contents,saved,unavailable,failed,not_processed);
    for item in items {
        text.push_str(&format!(
            "{:?}\t{}\t{}\t{}\n",
            item.status,
            item.module,
            item.lesson,
            item.reason.as_deref().unwrap_or("")
        ));
    }
    tokio::fs::write(root.join("download-report.txt"), text).await?;
    if !complete {
        // Old versions could write this marker despite failures.
        let _ = tokio::fs::remove_file(Path::new(course_dir).join(".complete")).await;
        return Err(anyhow!(
            "Partial Kiwify download: {}/{} contents saved; {} unavailable, {} failed, {} not processed. See download-report.txt.",
            saved,total_contents,unavailable,failed,not_processed
        ));
    }
    omniget_core::core::course_utils::mark_course_complete(course_dir).await
}

async fn download_hls_video(
    m3u8_url: &str,
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    use omniget_core::core::media_processor::MediaProcessor;

    let _result = MediaProcessor::download_hls(
        m3u8_url,
        output_path,
        "https://admin.kiwify.com.br/",
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
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let ytdlp_path = omniget_core::core::ytdlp::ensure_ytdlp().await?;
    let out_path = Path::new(output_path);
    let out_dir = out_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid video output path"))?;
    let template = video_filename_template(out_path)?;
    let (tx, _rx) = mpsc::channel(8);

    let result = omniget_core::core::ytdlp::download_video(
        &ytdlp_path,
        url,
        out_dir,
        None,
        tx,
        None,
        None,
        Some(&template),
        Some("https://admin.kiwify.com.br/"),
        cancel_token.clone(),
        None,
        8,
        false,
        &[],
        None,
    )
    .await?;

    Ok(result.file_size_bytes)
}

fn video_filename_template(output_path: &Path) -> anyhow::Result<String> {
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid video filename"))?;
    // Keep the extractor's actual container extension (e.g. WebM), and escape
    // literal percent signs in lesson titles for yt-dlp's template language.
    Ok(format!("{}.%(ext)s", stem.replace('%', "%%")))
}

async fn download_file_direct(
    client: &reqwest::Client,
    url: &str,
    output_path: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let out = Path::new(output_path);
    let (tx, _rx) = mpsc::channel(8);

    omniget_core::core::direct_downloader::download_direct(client, url, out, tx, Some(cancel_token))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractor_template_preserves_native_format_and_literal_percent_in_titles() {
        let template = video_filename_template(Path::new("1. A 100% lesson.mp4")).unwrap();
        assert_eq!(template, "1. A 100%% lesson.%(ext)s");
        assert!(!template.ends_with(".mp4"));
    }

    #[tokio::test]
    async fn failed_batch_cannot_leave_a_success_marker_and_success_can_resume() {
        let path =
            std::env::temp_dir().join(format!("omniget-kiwify-test-{}", rand::random::<u64>()));
        tokio::fs::create_dir_all(&path).await.unwrap();
        let dir = path.to_str().unwrap();
        tokio::fs::write(path.join(".complete"), "old incorrect marker")
            .await
            .unwrap();
        let failed = [report_item(
            "module",
            "lesson",
            LessonStatus::Failed,
            Some("test_failure"),
        )];
        assert!(finish_course(dir, 1, &failed).await.is_err());
        assert!(!path.join(".complete").exists());
        let saved = [report_item("module", "lesson", LessonStatus::Saved, None)];
        finish_course(dir, 1, &saved).await.unwrap();
        assert!(path.join(".complete").exists());
        tokio::fs::remove_dir_all(path).await.unwrap();
    }

    #[tokio::test]
    async fn skipped_or_unprocessed_content_is_partial_not_complete() {
        let path =
            std::env::temp_dir().join(format!("omniget-kiwify-partial-{}", rand::random::<u64>()));
        tokio::fs::create_dir_all(&path).await.unwrap();
        let items = [
            report_item("module", "saved", LessonStatus::Saved, None),
            report_item(
                "module",
                "blocked",
                LessonStatus::Unavailable,
                Some("server_locked"),
            ),
        ];
        let error = finish_course(path.to_str().unwrap(), 3, &items)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("1/3 contents saved; 1 unavailable, 0 failed, 1 not processed"));
        assert!(!path.join(".complete").exists());
        let report: serde_json::Value = serde_json::from_slice(
            &tokio::fs::read(path.join("download-report.json"))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(report["complete"], false);
        assert_eq!(report["unavailable_contents"], 1);
        assert_eq!(report["not_processed_contents"], 1);
        tokio::fs::remove_dir_all(path).await.unwrap();
    }
}
