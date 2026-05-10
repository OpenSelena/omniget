use serde::Serialize;

use crate::core::queue::{self, emit_queue_state_from_state, QueueItemInfo};
use crate::core::url_parser;
use crate::platforms::Platform;
use crate::storage::config;
use crate::AppState;

#[cfg(not(target_os = "android"))]
use crate::core::ytdlp;
#[cfg(not(target_os = "android"))]
use crate::models::media::FormatInfo;

#[derive(Clone, Serialize)]
pub struct PlatformInfo {
    pub platform: String,
    pub supported: bool,
    pub content_id: Option<String>,
    pub content_type: Option<String>,
}

#[tauri::command]
pub fn check_cookie_error() -> bool {
    let has_error = crate::core::ytdlp::has_cookie_error();
    if has_error {
        crate::core::ytdlp::clear_cookie_error();
    }
    has_error
}

#[derive(Clone, Serialize)]
pub struct PathLimitInfo {
    pub limit: usize,
    pub current: usize,
    pub reserve: usize,
    pub ok: bool,
}

#[tauri::command]
pub fn validate_output_path(output_dir: String) -> PathLimitInfo {
    match crate::core::path_limits::validate_output_dir(&output_dir) {
        Ok(()) => PathLimitInfo {
            limit: crate::core::path_limits::MAX_PATH_LEN,
            current: output_dir.chars().count() + crate::core::path_limits::SEPARATOR_RESERVE,
            reserve: crate::core::path_limits::MIN_FILENAME_RESERVE,
            ok: true,
        },
        Err(err) => PathLimitInfo {
            limit: err.limit,
            current: err.current,
            reserve: err.reserve,
            ok: false,
        },
    }
}

#[tauri::command]
pub async fn detect_platform(url: String) -> Result<PlatformInfo, String> {
    let _timer_start = std::time::Instant::now();
    match Platform::from_url(&url) {
        Some(platform) => {
            let parsed = url_parser::parse_url(&url);
            let result = Ok(PlatformInfo {
                platform: platform.to_string(),
                supported: true,
                content_id: parsed.as_ref().and_then(|p| p.content_id.clone()),
                content_type: parsed.map(|p| format!("{:?}", p.content_type).to_lowercase()),
            });
            tracing::debug!("[perf] detect_platform took {:?}", _timer_start.elapsed());
            result
        }
        None => {
            let is_valid_url = url::Url::parse(&url)
                .map(|u| u.scheme() == "http" || u.scheme() == "https")
                .unwrap_or(false);
            let result = Ok(PlatformInfo {
                platform: if is_valid_url {
                    "generic".to_string()
                } else {
                    "unknown".to_string()
                },
                supported: is_valid_url,
                content_id: None,
                content_type: None,
            });
            tracing::debug!("[perf] detect_platform took {:?}", _timer_start.elapsed());
            result
        }
    }
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn get_media_formats(url: String) -> Result<Vec<FormatInfo>, String> {
    let _timer_start = std::time::Instant::now();
    let ytdlp_path = ytdlp::ensure_ytdlp()
        .await
        .map_err(|e| format!("yt-dlp unavailable: {}", e))?;

    let json = ytdlp::get_video_info(&ytdlp_path, &url, &[])
        .await
        .map_err(|e| format!("Failed to get formats: {}", e))?;

    tracing::debug!("[perf] get_media_formats took {:?}", _timer_start.elapsed());
    Ok(ytdlp::parse_formats(&json))
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn prefetch_media_info(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    let platform = Platform::from_url(&url);
    let platform_name = platform
        .map(|p| p.to_string())
        .unwrap_or_else(|| "generic".to_string());

    let downloader = match state.registry.find_platform(&url) {
        Some(d) => d,
        None => return Err("No downloader available".to_string()),
    };

    let ytdlp_path = ytdlp::find_ytdlp_cached().await;

    tokio::spawn(async move {
        queue::prefetch_info_with_emit(
            &url,
            &*downloader,
            &platform_name,
            ytdlp_path.as_deref(),
            Some(app),
        )
        .await;
    });

    Ok(())
}

#[derive(Clone, Serialize)]
pub struct DownloadStarted {
    pub id: u64,
    pub title: String,
}

#[cfg(not(target_os = "android"))]
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn download_from_url(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
    output_dir: String,
    download_mode: Option<String>,
    quality: Option<String>,
    format_id: Option<String>,
    referer: Option<String>,
) -> Result<DownloadStarted, String> {
    let _timer_start = std::time::Instant::now();
    let platform = Platform::from_url(&url);

    if let Err(err) = crate::core::path_limits::validate_output_dir(&output_dir) {
        return Err(format!(
            "PathTooLong|{}|{}|{}",
            err.limit, err.current, err.reserve
        ));
    }

    let download_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let download_queue = state.download_queue.clone();

    {
        let settings = config::load_settings(&app);
        let mut q = download_queue.lock().await;
        q.max_concurrent = settings.advanced.max_concurrent_downloads.max(1);
        q.stagger_delay_ms = settings.advanced.stagger_delay_ms;
        q.default_max_retries = settings.advanced.max_retries;
        if q.has_url(&url) {
            tracing::debug!("[perf] download_from_url took {:?}", _timer_start.elapsed());
            return Err("Download already in progress for this URL".to_string());
        }
    }

    let downloader = match state.registry.find_platform(&url) {
        Some(d) => d,
        None => {
            tracing::debug!("[perf] download_from_url took {:?}", _timer_start.elapsed());
            return Err("No downloader available for this URL".to_string());
        }
    };

    let platform_name = platform
        .map(|p| p.to_string())
        .unwrap_or_else(|| "generic".to_string());
    let title = url.clone();
    let ytdlp_path = ytdlp::find_ytdlp_cached().await;

    let cached_info = queue::try_get_cached_info(&url).await;

    let state_to_emit = {
        let mut q = download_queue.lock().await;
        q.enqueue(
            download_id,
            url,
            platform_name,
            title.clone(),
            output_dir,
            download_mode,
            quality,
            format_id,
            referer,
            None,
            None,
            None,
            cached_info,
            None,
            None,
            downloader,
            ytdlp_path,
            false,
        );

        let next_ids = q.next_queued_ids();
        for nid in &next_ids {
            q.mark_active(*nid);
        }
        q.get_state()
    };
    emit_queue_state_from_state(&app, state_to_emit);

    let q_clone = download_queue.clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        let ids_to_start = {
            let q = q_clone.lock().await;
            q.items
                .iter()
                .filter(|i| i.status == queue::QueueStatus::Active)
                .filter(|i| i.id == download_id)
                .map(|i| i.id)
                .collect::<Vec<_>>()
        };

        let stagger = {
            let q = q_clone.lock().await;
            q.stagger_delay_ms
        };

        for (i, nid) in ids_to_start.into_iter().enumerate() {
            if i > 0 && stagger > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(stagger)).await;
            }
            let a = app_clone.clone();
            let qc = q_clone.clone();
            tokio::spawn(async move {
                queue::spawn_download(a, qc, nid).await;
            });
        }
    });

    tracing::debug!("[perf] download_from_url took {:?}", _timer_start.elapsed());
    Ok(DownloadStarted {
        id: download_id,
        title,
    })
}

#[tauri::command]
pub async fn cancel_generic_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    download_id: u64,
) -> Result<String, String> {
    let (state_to_emit, seeding_torrent_id) = {
        let mut q = state.download_queue.lock().await;
        let (cancelled, torrent_id) = q.cancel(download_id);
        if cancelled {
            (Some(q.get_state()), torrent_id)
        } else {
            (None, None)
        }
    };
    if let Some(tid) = seeding_torrent_id {
        if let Some(session) = state.torrent_session.lock().await.as_ref() {
            let _ = session
                .delete(librqbit::api::TorrentIdOrHash::Id(tid), false)
                .await;
        }
    }
    if let Some(s) = state_to_emit {
        emit_queue_state_from_state(&app, s);
        queue::try_start_next(app, state.download_queue.clone()).await;
        Ok("Download cancelled".to_string())
    } else {
        Err("No active download for this ID".to_string())
    }
}

#[tauri::command]
pub async fn pause_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    download_id: u64,
) -> Result<String, String> {
    let (state_to_emit, torrent_id) = {
        let mut q = state.download_queue.lock().await;
        if q.pause(download_id) {
            let tid = q
                .items
                .iter()
                .find(|i| i.id == download_id)
                .and_then(|i| i.torrent_id);
            (Some(q.get_state()), tid)
        } else {
            (None, None)
        }
    };
    if let Some(tid) = torrent_id {
        if let Some(session) = state.torrent_session.lock().await.as_ref() {
            if let Some(handle) = session.get(librqbit::api::TorrentIdOrHash::Id(tid)) {
                let _ = session.pause(&handle).await;
            }
        }
    }
    if let Some(s) = state_to_emit {
        emit_queue_state_from_state(&app, s);
        queue::try_start_next(app, state.download_queue.clone()).await;
        Ok("Download paused".to_string())
    } else {
        Err("Download cannot be paused".to_string())
    }
}

#[tauri::command]
pub async fn resume_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    download_id: u64,
) -> Result<String, String> {
    let (state_to_emit, torrent_id) = {
        let mut q = state.download_queue.lock().await;
        if q.resume(download_id) {
            let tid = q
                .items
                .iter()
                .find(|i| i.id == download_id)
                .and_then(|i| i.torrent_id);
            (Some(q.get_state()), tid)
        } else {
            (None, None)
        }
    };
    if let Some(tid) = torrent_id {
        if let Some(session) = state.torrent_session.lock().await.as_ref() {
            if let Some(handle) = session.get(librqbit::api::TorrentIdOrHash::Id(tid)) {
                let _ = session.unpause(&handle).await;
            }
        }
    }
    if let Some(s) = state_to_emit {
        emit_queue_state_from_state(&app, s);
        queue::try_start_next(app, state.download_queue.clone()).await;
        Ok("Download resumed".to_string())
    } else {
        Err("Download cannot be resumed".to_string())
    }
}

#[tauri::command]
pub async fn retry_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    download_id: u64,
) -> Result<String, String> {
    let state_to_emit = {
        let mut q = state.download_queue.lock().await;
        if q.retry(download_id) {
            Some(q.get_state())
        } else {
            None
        }
    };
    if let Some(s) = state_to_emit {
        emit_queue_state_from_state(&app, s);
        queue::try_start_next(app, state.download_queue.clone()).await;
        Ok("Download re-queued".to_string())
    } else {
        Err("Download cannot be retried".to_string())
    }
}

#[tauri::command]
pub async fn remove_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    download_id: u64,
) -> Result<String, String> {
    let (state_to_emit, seeding_torrent_id) = {
        let mut q = state.download_queue.lock().await;
        match q.remove(download_id) {
            Some(torrent_id) => (Some(q.get_state()), torrent_id),
            None => (None, None),
        }
    };
    if let Some(tid) = seeding_torrent_id {
        if let Some(session) = state.torrent_session.lock().await.as_ref() {
            let _ = session
                .delete(librqbit::api::TorrentIdOrHash::Id(tid), false)
                .await;
        }
    }
    if let Some(s) = state_to_emit {
        crate::core::download_log::clear(download_id);
        emit_queue_state_from_state(&app, s);
        queue::try_start_next(app, state.download_queue.clone()).await;
        Ok("Download removed".to_string())
    } else {
        Err("Download not found".to_string())
    }
}

#[tauri::command]
pub fn get_download_log(download_id: u64) -> Vec<String> {
    crate::core::download_log::get(download_id)
}

#[tauri::command]
pub fn clear_download_log(download_id: u64) {
    crate::core::download_log::clear(download_id);
}

#[tauri::command]
pub fn get_recovery_items() -> Vec<crate::core::recovery::RecoveryItem> {
    crate::core::recovery::list()
}

#[tauri::command]
pub fn get_download_history() -> Vec<crate::core::queue_history::HistoryEntry> {
    crate::core::queue_history::list()
}

#[tauri::command]
pub fn clear_download_history() {
    crate::core::queue_history::clear_all();
}

#[tauri::command]
pub fn discard_recovery() {
    crate::core::recovery::clear_all();
}

#[tauri::command]
pub async fn restore_recovery(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let items = crate::core::recovery::list();
    crate::core::recovery::clear_all();
    let mut restored: u32 = 0;
    for item in items {
        match download_from_url(
            app.clone(),
            state.clone(),
            item.url,
            item.output_dir,
            item.download_mode,
            item.quality,
            item.format_id,
            item.referer,
        )
        .await
        {
            Ok(_) => restored += 1,
            Err(e) => tracing::warn!("[recovery] restore failed: {}", e),
        }
    }
    Ok(restored)
}

#[tauri::command]
pub fn parse_batch_file(path: String) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read error: {}", e))?;
    let mut urls = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let candidate = line.split('|').next().unwrap_or(line).trim();
        if candidate.starts_with("http://")
            || candidate.starts_with("https://")
            || candidate.starts_with("magnet:")
            || candidate.starts_with("p2p:")
        {
            urls.push(candidate.to_string());
        }
    }
    Ok(urls)
}

#[tauri::command]
pub async fn get_queue_state(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<QueueItemInfo>, String> {
    let q = state.download_queue.lock().await;
    Ok(q.get_state())
}

#[tauri::command]
pub async fn update_max_concurrent(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    max: u32,
) -> Result<String, String> {
    if !(1..=10).contains(&max) {
        return Err("Value must be between 1 and 10".to_string());
    }
    let state_to_emit = {
        let mut q = state.download_queue.lock().await;
        q.max_concurrent = max;
        q.get_state()
    };
    emit_queue_state_from_state(&app, state_to_emit);
    queue::try_start_next(app, state.download_queue.clone()).await;
    Ok(format!("Max concurrent set to {}", max))
}

#[tauri::command]
pub async fn pause_all_downloads(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let (state_to_emit, count, paused_torrents) = {
        let mut q = state.download_queue.lock().await;
        let paused = q.pause_all();
        let n = paused.len() as u32;
        let torrents: Vec<usize> = paused.iter().filter_map(|(_, tid)| *tid).collect();
        (q.get_state(), n, torrents)
    };
    if let Some(session) = state.torrent_session.lock().await.as_ref() {
        for tid in paused_torrents {
            if let Some(handle) = session.get(librqbit::api::TorrentIdOrHash::Id(tid)) {
                let _ = session.pause(&handle).await;
            }
        }
    }
    emit_queue_state_from_state(&app, state_to_emit);
    Ok(count)
}

#[tauri::command]
pub async fn resume_all_downloads(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let (state_to_emit, count, resumed_torrents) = {
        let mut q = state.download_queue.lock().await;
        let resumed = q.resume_all();
        let n = resumed.len() as u32;
        let torrents: Vec<usize> = resumed.iter().filter_map(|(_, tid)| *tid).collect();
        (q.get_state(), n, torrents)
    };
    if let Some(session) = state.torrent_session.lock().await.as_ref() {
        for tid in resumed_torrents {
            if let Some(handle) = session.get(librqbit::api::TorrentIdOrHash::Id(tid)) {
                let _ = session.unpause(&handle).await;
            }
        }
    }
    emit_queue_state_from_state(&app, state_to_emit);
    queue::try_start_next(app, state.download_queue.clone()).await;
    Ok(count)
}

#[tauri::command]
pub async fn reorder_queue(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    ids: Vec<u64>,
) -> Result<bool, String> {
    let (changed, state_to_emit) = {
        let mut q = state.download_queue.lock().await;
        let ok = q.reorder(ids);
        (ok, q.get_state())
    };
    if changed {
        emit_queue_state_from_state(&app, state_to_emit);
    }
    Ok(changed)
}

#[tauri::command]
pub async fn clear_finished_downloads(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let (state_to_emit, finished_ids) = {
        let mut q = state.download_queue.lock().await;
        let ids = q
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    crate::core::queue::QueueStatus::Complete { .. }
                        | crate::core::queue::QueueStatus::Error { .. }
                )
            })
            .map(|i| i.id)
            .collect::<Vec<_>>();
        q.clear_finished();
        (q.get_state(), ids)
    };
    for id in finished_ids {
        crate::core::download_log::clear(id);
    }
    emit_queue_state_from_state(&app, state_to_emit);
    Ok("Finished downloads cleared".to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn reveal_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::path::{Path, PathBuf};
        use std::process::Stdio;

        let file_path = Path::new(&path);
        let abs_path: PathBuf = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(file_path))
                .unwrap_or_else(|_| file_path.to_path_buf())
        };

        let dir_path = abs_path.parent().unwrap_or(&abs_path);
        let item_uri = url::Url::from_file_path(&abs_path)
            .or_else(|_| url::Url::from_file_path(file_path))
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("file://{}", abs_path.display()));
        let dir_uri = url::Url::from_directory_path(dir_path)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("file://{}", dir_path.display()));

        let gdbus_show_items_arg = format!(
            "[\"{}\"]",
            item_uri.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let show_items_with_gdbus = tokio::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.FileManager1",
                "--object-path",
                "/org/freedesktop/FileManager1",
                "--method",
                "org.freedesktop.FileManager1.ShowItems",
                &gdbus_show_items_arg,
                "",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        let show_items_ok = if show_items_with_gdbus {
            true
        } else {
            let dbus_send_array_arg = format!("array:string:{}", item_uri);
            tokio::process::Command::new("dbus-send")
                .args([
                    "--session",
                    "--dest=org.freedesktop.FileManager1",
                    "--type=method_call",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                    &dbus_send_array_arg,
                    "string:",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
        };

        if !show_items_ok {
            let portal_ok = tokio::process::Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest",
                    "org.freedesktop.portal.Desktop",
                    "--object-path",
                    "/org/freedesktop/portal/desktop",
                    "--method",
                    "org.freedesktop.portal.OpenURI.OpenDirectory",
                    "",
                    &dir_uri,
                    "{}",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false);

            if !portal_ok {
                std::process::Command::new("xdg-open")
                    .arg(dir_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn open_path_default(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
