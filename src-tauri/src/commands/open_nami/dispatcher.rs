use super::args::{build_gallery_dl_args, build_yt_dlp_args, GalleryDlArgsOptions, YtDlpArgsOptions};
use super::cookies::{find_candidate_cookie, SafeCookieSandbox};
use super::fetcher::{ensure_latest_gallery_dl, resolve_gallery_dl, resolve_yt_dlp};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadResult {
    pub success: bool,
    pub message: String,
    pub files_count: u32,
    pub cancelled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub message: String,
    pub files_downloaded: u32,
    pub stage: Option<String>,
    pub stage_index: Option<u32>,
    pub stage_total: Option<u32>,
}

struct RunningDownload {
    pid: Option<u32>,
    cancelled: Arc<AtomicBool>,
}

static RUNNING_DOWNLOADS: LazyLock<Mutex<HashMap<String, RunningDownload>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_running_downloads() -> std::sync::MutexGuard<'static, HashMap<String, RunningDownload>> {
    RUNNING_DOWNLOADS.lock().unwrap_or_else(|p| p.into_inner())
}

pub struct DownloadRegistration {
    pub id: String,
    pub cancelled: Arc<AtomicBool>,
}

impl DownloadRegistration {
    pub fn new(id: &str) -> Result<Self, String> {
        let mut map = get_running_downloads();
        if map.contains_key(id) {
            return Err("A download with this ID is already registered".to_string());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        map.insert(
            id.to_string(),
            RunningDownload {
                pid: None,
                cancelled: cancelled.clone(),
            },
        );
        Ok(Self {
            id: id.to_string(),
            cancelled,
        })
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn set_pid(&self, pid: Option<u32>) {
        let mut map = get_running_downloads();
        if let Some(rd) = map.get_mut(&self.id) {
            rd.pid = pid;
        }
    }
}

impl Drop for DownloadRegistration {
    fn drop(&mut self) {
        let pid = get_running_downloads()
            .remove(&self.id)
            .and_then(|rd| rd.pid);
        if let (Some(pid), true) = (pid, thread::panicking()) {
            kill_process_tree(pid);
        }
    }
}

pub fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
        kill_cmd.creation_flags(0x08000000);
        let _ = kill_cmd.output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &format!("-{}", pid)])
            .output();
    }
}

pub fn cancel_running_download(download_id: &str) -> Result<String, String> {
    let pid = {
        let mut map = get_running_downloads();
        let rd = map
            .get_mut(download_id)
            .ok_or("No running download found for that ID")?;
        rd.cancelled.store(true, Ordering::SeqCst);
        rd.pid
    };
    if let Some(pid) = pid {
        kill_process_tree(pid);
    }
    Ok("Download cancelled".to_string())
}

pub fn detect_platform_name(url: &str) -> &'static str {
    let low = url.to_lowercase();
    let host = if let Some(after_scheme) = low.split("://").nth(1) {
        after_scheme.split('/').next().unwrap_or("")
    } else {
        low.split('/').next().unwrap_or("")
    };
    let host = host.split(':').next().unwrap_or("");

    if host == "instagram.com" || host.ends_with(".instagram.com") {
        "Instagram"
    } else if host == "tiktok.com" || host.ends_with(".tiktok.com") {
        "TikTok"
    } else if host == "facebook.com" || host.ends_with(".facebook.com") {
        "Facebook"
    } else if host == "x.com"
        || host.ends_with(".x.com")
        || host == "twitter.com"
        || host.ends_with(".twitter.com")
    {
        "X"
    } else {
        "Other"
    }
}

pub fn sanitize_path_component(input: &str) -> String {
    let mut cleaned: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '.' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    while cleaned.starts_with('.') || cleaned.starts_with('-') || cleaned.starts_with(' ') {
        cleaned.remove(0);
    }
    while cleaned.ends_with('.') || cleaned.ends_with(' ') {
        cleaned.pop();
    }
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", "_");
    }
    let trimmed = cleaned.trim().trim_matches('_');
    let lower = trimmed.to_ascii_lowercase();
    const RESERVED: &[&str] = &[
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if trimmed.is_empty() || RESERVED.contains(&lower.as_str()) {
        return "unknown_user".to_string();
    }
    trimmed.chars().take(120).collect()
}

pub fn extract_username_generic(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if trimmed.contains("://") {
        let after_scheme = trimmed.split("://").nth(1).unwrap_or(trimmed);
        let mut parts = after_scheme.split('/');
        parts.next()?;
        parts.next()?
    } else {
        trimmed
    };

    let cleaned = candidate.split(['?', '#']).next()?;
    let cleaned = cleaned.trim_start_matches('@');
    if cleaned.is_empty() || ["p", "reel", "tv", "stories", "highlights"].contains(&cleaned) {
        return None;
    }

    Some(sanitize_path_component(cleaned))
}

pub async fn execute_download_workflow(
    app: AppHandle,
    url: String,
    output_dir: String,
    cookies_file: Option<String>,
    content_type: String,
    download_id: String,
    app_data_dir: PathBuf,
) -> Result<DownloadResult, String> {
    let reg = DownloadRegistration::new(&download_id)?;

    // 1. Ensure primary engine binary exists (auto-fetch if needed)
    let gallery_bin = match resolve_gallery_dl(&app_data_dir) {
        Some(p) => p,
        None => {
            let _ = app.emit(
                &format!("download_{}", download_id),
                DownloadProgress {
                    message: "Provisioning download engine binary...".to_string(),
                    files_downloaded: 0,
                    stage: Some("Setup".to_string()),
                    stage_index: Some(0),
                    stage_total: Some(1),
                },
            );
            ensure_latest_gallery_dl(&app_data_dir).await?
        }
    };

    let yt_bin = resolve_yt_dlp(&app_data_dir);

    // 2. Discover and create ephemeral cookie sandbox if cookies exist
    let platform_name = detect_platform_name(&url);
    let resolved_cookie_path = find_candidate_cookie(platform_name, cookies_file.as_deref(), &app_data_dir);
    let temp_base = app_data_dir.join("open_nami").join("temp");
    let cookie_sandbox = resolved_cookie_path
        .as_deref()
        .and_then(|p| SafeCookieSandbox::create(p, &temp_base).ok());

    let active_cookie_str = cookie_sandbox
        .as_ref()
        .map(|s| s.path().to_string_lossy().to_string());

    // 3. Plan stages
    let mut sub_types: Vec<&str> = match content_type.as_str() {
        "photos" => vec!["photos"],
        "videos" => vec!["videos"],
        "stories" => vec!["stories"],
        "highlights" => vec!["highlights"],
        _ => vec!["photos", "videos"],
    };

    if platform_name == "Instagram" && active_cookie_str.is_some() && content_type == "all" {
        sub_types.push("stories");
        sub_types.push("highlights");
    }

    let stage_total = sub_types.len() as u32;
    let mut total_files = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for (idx, sub_type) in sub_types.into_iter().enumerate() {
        if reg.is_cancelled() {
            return Ok(DownloadResult {
                success: false,
                message: "Download cancelled".to_string(),
                files_count: total_files,
                cancelled: true,
            });
        }

        let stage_index = idx as u32 + 1;
        let stage_label = match sub_type {
            "photos" => "Photos",
            "videos" => "Videos",
            "stories" => "Stories",
            "highlights" => "Highlights",
            _ => "Media",
        };

        let _ = app.emit(
            &format!("download_{}", download_id),
            DownloadProgress {
                message: format!("Starting {}", stage_label),
                files_downloaded: total_files,
                stage: Some(stage_label.to_string()),
                stage_index: Some(stage_index),
                stage_total: Some(stage_total),
            },
        );

        match run_single_stage(
            &app,
            &gallery_bin,
            yt_bin.as_deref(),
            &url,
            &output_dir,
            active_cookie_str.as_deref(),
            sub_type,
            &reg,
            platform_name,
            total_files,
            stage_index,
            stage_total,
        ) {
            Ok(result) => {
                total_files += result.files_count;
                if result.cancelled || reg.is_cancelled() {
                    return Ok(DownloadResult {
                        success: false,
                        message: "Download cancelled".to_string(),
                        files_count: total_files,
                        cancelled: true,
                    });
                }
                if !result.success {
                    failures.push(format!("{}: {}", sub_type, result.message));
                }
            }
            Err(e) => {
                if reg.is_cancelled() {
                    return Ok(DownloadResult {
                        success: false,
                        message: "Download cancelled".to_string(),
                        files_count: total_files,
                        cancelled: true,
                    });
                }
                failures.push(format!("{}: {}", sub_type, e));
            }
        }
    }

    if reg.is_cancelled() {
        return Ok(DownloadResult {
            success: false,
            message: "Download cancelled".to_string(),
            files_count: total_files,
            cancelled: true,
        });
    }

    if failures.is_empty() || total_files > 0 {
        Ok(DownloadResult {
            success: true,
            message: format!(
                "Download completed successfully. {} files downloaded.",
                total_files
            ),
            files_count: total_files,
            cancelled: false,
        })
    } else {
        Ok(DownloadResult {
            success: false,
            message: failures.join("\n"),
            files_count: total_files,
            cancelled: false,
        })
    }
}

fn run_single_stage(
    app: &AppHandle,
    gallery_bin: &Path,
    yt_bin: Option<&Path>,
    url: &str,
    output_dir: &str,
    cookie_path: Option<&str>,
    content_type: &str,
    reg: &DownloadRegistration,
    platform_name: &str,
    base_count: u32,
    stage_index: u32,
    stage_total: u32,
) -> Result<DownloadResult, String> {
    if reg.is_cancelled() {
        return Ok(DownloadResult {
            success: false,
            message: "Download cancelled".to_string(),
            files_count: 0,
            cancelled: true,
        });
    }

    let username = extract_username_generic(url).unwrap_or_else(|| "unknown_user".to_string());
    let media_folder = match content_type {
        "photos" => "photos",
        "videos" => "videos",
        "stories" => "stories",
        "highlights" => "highlights",
        _ => "all",
    };

    let final_dest = PathBuf::from(output_dir)
        .join("OpenNami")
        .join(sanitize_path_component(platform_name))
        .join(sanitize_path_component(&username))
        .join(media_folder);

    fs::create_dir_all(&final_dest).map_err(|e| {
        format!(
            "Failed to create output folder {}: {}",
            final_dest.display(),
            e
        )
    })?;

    let media_type_name = match content_type {
        "photos" => "Photos",
        "videos" => "Videos",
        "stories" => "Stories",
        "highlights" => "Highlights",
        _ => "Media",
    };

    // Primary execution via gallery-dl
    let gdl_opts = GalleryDlArgsOptions {
        dest_dir: &final_dest,
        content_type,
        platform: platform_name,
        clean_url: url,
        username: &username,
        cookie_path,
        custom_ua: None,
        proxy: None,
    };
    let gdl_args = build_gallery_dl_args(&gdl_opts);

    let gdl_result = run_process_stream(
        app,
        gallery_bin,
        &gdl_args,
        reg,
        base_count,
        media_type_name,
        stage_index,
        stage_total,
    )?;

    // Dual-Engine Fallback for Videos
    if content_type == "videos" && gdl_result.files_count == 0 && !reg.is_cancelled() {
        if let Some(yt_executable) = yt_bin {
            let _ = app.emit(
                &format!("download_{}", reg.id),
                DownloadProgress {
                    message: "Switching to secondary video engine (yt-dlp)...".to_string(),
                    files_downloaded: base_count,
                    stage: Some(media_type_name.to_string()),
                    stage_index: Some(stage_index),
                    stage_total: Some(stage_total),
                },
            );

            let yt_opts = YtDlpArgsOptions {
                dest_dir: &final_dest,
                platform: platform_name,
                clean_url: url,
                cookie_path,
                proxy: None,
            };
            let yt_args = build_yt_dlp_args(&yt_opts);

            return run_process_stream(
                app,
                yt_executable,
                &yt_args,
                reg,
                base_count,
                media_type_name,
                stage_index,
                stage_total,
            );
        }
    }

    Ok(gdl_result)
}

fn run_process_stream(
    app: &AppHandle,
    executable: &Path,
    args: &[String],
    reg: &DownloadRegistration,
    base_count: u32,
    stage_label: &str,
    stage_index: u32,
    stage_total: u32,
) -> Result<DownloadResult, String> {
    let mut cmd = Command::new(executable);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start engine {}: {}", executable.display(), e))?;

    let pid = child.id();
    reg.set_pid(Some(pid));

    if reg.is_cancelled() {
        kill_process_tree(pid);
    }

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let files_downloaded = Arc::new(AtomicU32::new(0));

    let stdout_handle = {
        let app = app.clone();
        let download_id = reg.id.clone();
        let files_downloaded = files_downloaded.clone();
        let stage_label = stage_label.to_string();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(pos) = line.find("FILE_OK:") {
                    let filename = line[pos + "FILE_OK:".len()..].trim();
                    let count = files_downloaded.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = app.emit(
                        &format!("download_{}", download_id),
                        DownloadProgress {
                            message: format!("Downloaded: {}", filename),
                            files_downloaded: base_count + count,
                            stage: Some(stage_label.clone()),
                            stage_index: Some(stage_index),
                            stage_total: Some(stage_total),
                        },
                    );
                } else if line.contains("[download]") && line.contains("Destination:") {
                    let count = files_downloaded.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = app.emit(
                        &format!("download_{}", download_id),
                        DownloadProgress {
                            message: line.trim().to_string(),
                            files_downloaded: base_count + count,
                            stage: Some(stage_label.clone()),
                            stage_index: Some(stage_index),
                            stage_total: Some(stage_total),
                        },
                    );
                }
            }
        })
    };

    let stderr_tail = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(30)));
    let stderr_handle = {
        let app = app.clone();
        let download_id = reg.id.clone();
        let files_downloaded = files_downloaded.clone();
        let stderr_tail = stderr_tail.clone();
        let stage_label = stage_label.to_string();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                {
                    let mut tail = stderr_tail.lock().unwrap_or_else(|p| p.into_inner());
                    if tail.len() >= 30 {
                        tail.pop_front();
                    }
                    tail.push_back(line.clone());
                }

                let lower = trimmed.to_lowercase();
                if !lower.starts_with("[debug]") && !lower.contains("sleeping") {
                    let count = files_downloaded.load(Ordering::SeqCst);
                    let _ = app.emit(
                        &format!("download_{}", download_id),
                        DownloadProgress {
                            message: trimmed.to_string(),
                            files_downloaded: base_count + count,
                            stage: Some(stage_label.clone()),
                            stage_index: Some(stage_index),
                            stage_total: Some(stage_total),
                        },
                    );
                }
            }
        })
    };

    let status = child
        .wait()
        .map_err(|e| format!("Engine process failed: {}", e))?;

    reg.set_pid(None);
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    let final_count = files_downloaded.load(Ordering::SeqCst);

    if reg.is_cancelled() {
        return Ok(DownloadResult {
            success: false,
            message: "Download cancelled".to_string(),
            files_count: final_count,
            cancelled: true,
        });
    }

    if status.success() || final_count > 0 {
        Ok(DownloadResult {
            success: true,
            message: format!("Successfully downloaded {} files", final_count),
            files_count: final_count,
            cancelled: false,
        })
    } else {
        let err_tail = stderr_tail.lock().unwrap_or_else(|p| p.into_inner());
        let err_msg = err_tail.iter().cloned().collect::<Vec<_>>().join("\n");
        Ok(DownloadResult {
            success: false,
            message: if !err_msg.is_empty() { err_msg } else { "Download engine exited with error".to_string() },
            files_count: final_count,
            cancelled: false,
        })
    }
}
