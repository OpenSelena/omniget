use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{command, AppHandle, Emitter};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub url: String,
    pub username: Option<String>,
    pub platform: String,
    pub added_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadResult {
    pub success: bool,
    pub message: String,
    pub files_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub message: String,
    pub files_downloaded: u32,
    pub stage: Option<String>,
    pub stage_index: Option<u32>,
    pub stage_total: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub output_directory: Option<String>,
    pub cookies_file: Option<String>,
}

static RUNNING_DOWNLOADS: Lazy<Mutex<HashMap<String, u32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Settings and profiles share small JSON files. Serialize access so two Tauri
// commands cannot read-modify-write the same file concurrently.
static OPEN_OMNI_STORAGE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn omniget_open_omni_dir() -> Result<PathBuf, String> {
    let base =
        omniget_core::core::paths::app_data_dir().ok_or("Could not find app data directory")?;
    let dir = base.join("open_omni");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(dir)
}

#[command]
pub fn open_omni_check_python_dependencies() -> Result<String, String> {
    let python_found = ["python3", "python"].iter().any(|bin| {
        let mut cmd = Command::new(bin);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        cmd.arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    });

    if !python_found {
        return Err("Python is not installed".to_string());
    }

    let mut gallery_cmd = Command::new("gallery-dl");
    #[cfg(target_os = "windows")]
    gallery_cmd.creation_flags(0x08000000);

    let gallery_check = gallery_cmd
        .arg("--version")
        .output()
        .map_err(|e| format!("gallery-dl not found: {}", e))?;

    if !gallery_check.status.success() {
        return Err("gallery-dl is not installed".to_string());
    }

    Ok("All dependencies OK".to_string())
}

fn detect_platform_name(url: &str) -> &'static str {
    let low = url.to_lowercase();
    if low.contains("instagram.com") {
        "Instagram"
    } else if low.contains("tiktok.com") {
        "TikTok"
    } else if low.contains("facebook.com") {
        "Facebook"
    } else if low.contains("x.com") || low.contains("twitter.com") {
        "X"
    } else {
        "Other"
    }
}

/// SECURITY: strips any character that isn't safe inside a single path
/// segment, and collapses ".." sequences so a crafted/malformed URL can
/// never make the extracted "username" walk outside the intended
/// output directory (path traversal). This is the ONLY place raw,
/// user-influenced text becomes part of a filesystem path in this
/// module, so sanitizing here closes the issue everywhere it's used.
fn sanitize_path_component(input: &str) -> String {
    let mut cleaned: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Leading dots/dashes and any ".." run are how traversal or hidden
    // files sneak in even after the character filter above.
    while cleaned.starts_with('.') || cleaned.starts_with('-') {
        cleaned.remove(0);
    }
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", "_");
    }

    cleaned = cleaned.trim_matches('_').to_string();

    if cleaned.is_empty() {
        return "unknown_user".to_string();
    }

    // Cap length so we never hit filesystem path-length limits.
    cleaned.chars().take(120).collect()
}

fn extract_username_generic(url: &str) -> Option<String> {
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

    // SECURITY: sanitize before this value ever gets used to build a
    // filesystem path (see run_single_content_download).
    Some(sanitize_path_component(cleaned))
}

fn canonicalize_profile_url(platform: &str, raw_input: &str) -> String {
    let trimmed = raw_input.trim();
    if trimmed.contains("://") {
        return trimmed.to_string();
    }

    let username = trimmed.trim_start_matches('@');
    match platform {
        "instagram" => format!("https://www.instagram.com/{}/", username),
        "tiktok" => format!("https://www.tiktok.com/@{}", username),
        "facebook" => format!("https://www.facebook.com/{}", username),
        "x" => format!("https://x.com/{}", username),
        _ => trimmed.to_string(),
    }
}

fn validate_content_type(content_type: &str) -> Result<(), String> {
    match content_type {
        "all" | "photos" | "videos" | "stories" | "highlights" => Ok(()),
        _ => Err("Unsupported content type".to_string()),
    }
}

fn validate_profile_platform(platform: &str) -> Result<(), String> {
    match platform {
        "instagram" | "tiktok" | "facebook" | "x" => Ok(()),
        _ => Err("Unsupported profile platform".to_string()),
    }
}

#[command]
pub async fn open_omni_run_gallery_dl_download(
    app: AppHandle,
    url: String,
    output_dir: String,
    cookies_file: Option<String>,
    content_type: String,
    download_id: String,
) -> Result<DownloadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_gallery_dl_download_blocking(
            app,
            url,
            output_dir,
            cookies_file,
            content_type,
            download_id,
        )
    })
    .await
    .map_err(|e| format!("Download task panicked: {}", e))?
}

fn run_gallery_dl_download_blocking(
    app: AppHandle,
    url: String,
    output_dir: String,
    cookies_file: Option<String>,
    content_type: String,
    download_id: String,
) -> Result<DownloadResult, String> {
    validate_content_type(&content_type)?;
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let platform_name = detect_platform_name(&url);
    if content_type == "all" {
        // Each stage gets its own final directory. Instagram's profile
        // extractor supports stories and highlights through `include`.
        let mut sub_types: Vec<&str> = vec!["photos", "videos"];
        if platform_name == "Instagram" {
            sub_types.push("stories");
            sub_types.push("highlights");
        }

        let stage_total = sub_types.len() as u32;
        let mut total_files = 0u32;
        let mut failures: Vec<String> = Vec::new();

        for (idx, sub_type) in sub_types.into_iter().enumerate() {
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

            match run_single_content_download(
                &app,
                &url,
                &output_dir,
                &cookies_file,
                sub_type,
                &download_id,
                platform_name,
                total_files,
                stage_index,
                stage_total,
            ) {
                Ok(result) => {
                    total_files += result.files_count;
                    if !result.success {
                        failures.push(format!("{}: {}", sub_type, result.message));
                    }
                }
                Err(e) => failures.push(format!("{}: {}", sub_type, e)),
            }
        }

        if failures.is_empty() {
            Ok(DownloadResult {
                success: true,
                message: format!(
                    "Download completed successfully. {} files downloaded.",
                    total_files
                ),
                files_count: total_files,
            })
        } else {
            Ok(DownloadResult {
                success: false,
                message: format!(
                    "{} files downloaded, but some steps failed: {}",
                    total_files,
                    failures.join(" | ")
                ),
                files_count: total_files,
            })
        }
    } else {
        run_single_content_download(
            &app,
            &url,
            &output_dir,
            &cookies_file,
            &content_type,
            &download_id,
            platform_name,
            0,
            1,
            1,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn run_single_content_download(
    app: &AppHandle,
    url: &str,
    output_dir: &str,
    cookies_file: &Option<String>,
    content_type: &str,
    download_id: &str,
    platform_name: &str,
    base_count: u32,
    stage_index: u32,
    stage_total: u32,
) -> Result<DownloadResult, String> {
    if (content_type == "stories" || content_type == "highlights") && platform_name != "Instagram" {
        let label = if content_type == "stories" {
            "Stories"
        } else {
            "Highlights"
        };
        return Err(format!(
            "{} are only available for Instagram. gallery-dl has no {} support for {}.",
            label, content_type, platform_name
        ));
    }

    if (content_type == "stories" || content_type == "highlights")
        && cookies_file.as_deref().unwrap_or("").trim().is_empty()
    {
        let label = if content_type == "stories" {
            "Stories"
        } else {
            "Highlights"
        };
        return Err(format!(
            "{} require an Instagram cookies file to be configured.",
            label
        ));
    }

    if let Some(cookies) = cookies_file {
        if !cookies.trim().is_empty() && !PathBuf::from(cookies).is_file() {
            return Err("The configured cookies file does not exist or is not a file.".to_string());
        }
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
        .join("OpenOmni")
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

    let _ = app.emit(
        &format!("download_{}", download_id),
        DownloadProgress {
            message: format!(
                "Preparing to download {} for {}...",
                media_type_name, username
            ),
            files_downloaded: base_count,
            stage: Some(media_type_name.to_string()),
            stage_index: Some(stage_index),
            stage_total: Some(stage_total),
        },
    );

    let mut cmd = Command::new("gallery-dl");
    cmd.arg("--directory").arg(&final_dest);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    match content_type {
        "photos" => {
            if platform_name == "Instagram" {
                cmd.arg("-o").arg("include=posts,reels");
            }
            cmd.arg("--filter").arg("extension in ('jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'jfif', 'heic', 'avif', 'tiff', 'svg')");
        }
        "videos" => {
            if platform_name == "Instagram" {
                cmd.arg("-o").arg("include=posts,reels");
            }
            cmd.arg("--filter").arg("extension in ('mp4', 'webm', 'mkv', 'mov', 'avi', 'm4v', 'flv', 'wmv', '3gp', 'mpeg', 'mpg', 'ts', 'f4v', 'mts', 'm2ts')");
        }
        "stories" => {
            cmd.arg("-o").arg("include=stories");
        }
        "highlights" => {
            cmd.arg("-o").arg("include=highlights");
        }
        _ => {}
    }

    if let Some(cookies) = cookies_file {
        if !cookies.trim().is_empty() {
            cmd.arg("--cookies").arg(cookies);
        }
    }

    let app_settings = crate::storage::config::load_settings_standalone();
    let user_agent = app_settings.advanced.user_agent.trim();
    if !user_agent.is_empty() {
        cmd.arg("--user-agent").arg(user_agent);
    }
    if let Some(proxy) = omniget_core::core::http_client::proxy_url() {
        cmd.arg("--proxy").arg(&proxy);
    }

    cmd.arg("--Print")
        .arg("after:FILE_OK:{filename}.{extension}");
    cmd.arg("--sleep-request")
        .arg(if platform_name == "Instagram" {
            "6-12"
        } else {
            "2"
        });
    cmd.arg("--http-timeout").arg("30");

    cmd.arg(url);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start gallery-dl: {}", e))?;

    RUNNING_DOWNLOADS
        .lock()
        .unwrap()
        .insert(download_id.to_string(), child.id());

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let files_downloaded = Arc::new(AtomicU32::new(0));

    let stdout_handle = {
        let app = app.clone();
        let download_id = download_id.to_string();
        let files_downloaded = files_downloaded.clone();
        let stage_label = media_type_name.to_string();
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
                }
            }
        })
    };

    let stderr_tail = Arc::new(std::sync::Mutex::new(VecDeque::<String>::new()));
    let stderr_handle = {
        let app = app.clone();
        let download_id = download_id.to_string();
        let files_downloaded = files_downloaded.clone();
        let stderr_tail = stderr_tail.clone();
        let stage_label = media_type_name.to_string();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    let mut tail = stderr_tail.lock().unwrap();
                    if tail.len() == 12 {
                        tail.pop_front();
                    }
                    tail.push_back(line.clone());
                }
                let count = files_downloaded.load(Ordering::SeqCst);
                let _ = app.emit(
                    &format!("download_{}", download_id),
                    DownloadProgress {
                        message: line,
                        files_downloaded: base_count + count,
                        stage: Some(stage_label.clone()),
                        stage_index: Some(stage_index),
                        stage_total: Some(stage_total),
                    },
                );
            }
        })
    };

    let watchdog_completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watchdog_done = watchdog_completed.clone();
    let watchdog_pid = child.id();
    let _watchdog = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_secs(1800)); // 30 minutes
        if !watchdog_done.load(Ordering::SeqCst) {
            kill_process_tree(watchdog_pid);
        }
    });

    let status = child
        .wait()
        .map_err(|e| format!("Download process failed: {}", e))?;

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    watchdog_completed.store(true, Ordering::SeqCst);

    RUNNING_DOWNLOADS.lock().unwrap().remove(download_id);

    let final_count = files_downloaded.load(Ordering::SeqCst);

    if status.success() {
        Ok(DownloadResult {
            success: true,
            message: format!(
                "Download completed successfully. {} files downloaded.",
                final_count
            ),
            files_count: final_count,
        })
    } else {
        let stderr_message = stderr_tail
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let error_msg = if !stderr_message.is_empty() {
            stderr_message
        } else {
            format!(
                "gallery-dl exited with code {}",
                status.code().unwrap_or(-1)
            )
        };
        Ok(DownloadResult {
            success: false,
            message: error_msg,
            files_count: final_count,
        })
    }
}

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
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

#[command]
pub fn open_omni_cancel_download(download_id: String) -> Result<String, String> {
    let pid = RUNNING_DOWNLOADS.lock().unwrap().get(&download_id).copied();

    match pid {
        Some(pid) => {
            kill_process_tree(pid);
            RUNNING_DOWNLOADS.lock().unwrap().remove(&download_id);
            Ok("Download cancelled".to_string())
        }
        None => Err("No running download found for that ID".to_string()),
    }
}

#[command]
pub fn open_omni_save_app_settings(
    output_directory: Option<String>,
    cookies_file: Option<String>,
) -> Result<String, String> {
    let _storage_guard = OPEN_OMNI_STORAGE_LOCK
        .lock()
        .map_err(|_| "Open Omni storage lock is unavailable")?;
    let settings = AppSettings {
        output_directory,
        cookies_file,
    };

    let config_dir = omniget_open_omni_dir()?;
    let settings_file = config_dir.join("app_settings.json");

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_file, content).map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok("Settings saved successfully".to_string())
}

#[command]
pub fn open_omni_load_app_settings() -> Result<AppSettings, String> {
    let _storage_guard = OPEN_OMNI_STORAGE_LOCK
        .lock()
        .map_err(|_| "Open Omni storage lock is unavailable")?;
    let settings_file = omniget_open_omni_dir()?.join("app_settings.json");

    if !settings_file.exists() {
        return Ok(AppSettings {
            output_directory: None,
            cookies_file: None,
        });
    }

    let content = fs::read_to_string(&settings_file)
        .map_err(|e| format!("Failed to read settings: {}", e))?;

    let settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;

    Ok(settings)
}

#[command]
pub fn open_omni_load_profiles(platform: String) -> Result<Vec<Profile>, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = OPEN_OMNI_STORAGE_LOCK
        .lock()
        .map_err(|_| "Open Omni storage lock is unavailable")?;
    let profiles_file = omniget_open_omni_dir()?.join("profiles.json");

    if !profiles_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&profiles_file)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;

    let all_profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    let platform_profiles = all_profiles
        .get(&platform)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(platform_profiles)
}

#[command]
pub fn open_omni_save_profile(platform: String, url: String) -> Result<String, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = OPEN_OMNI_STORAGE_LOCK
        .lock()
        .map_err(|_| "Open Omni storage lock is unavailable")?;
    let raw_input = url.trim().to_string();

    if raw_input.is_empty() {
        return Err("URL or username cannot be empty".to_string());
    }

    let url = canonicalize_profile_url(&platform, &raw_input);

    let config_dir = omniget_open_omni_dir()?;
    let profiles_file = config_dir.join("profiles.json");

    let mut all_profiles: serde_json::Value = if profiles_file.exists() {
        let content = fs::read_to_string(&profiles_file).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let username = extract_username_generic(&url);

    if let Some(arr) = all_profiles
        .as_object()
        .and_then(|obj| obj.get(&platform))
        .and_then(|v| v.as_array())
    {
        for profile in arr {
            let existing_url = profile.get("url").and_then(|v| v.as_str());
            if existing_url == Some(url.as_str()) {
                return Err("Profile already exists".to_string());
            }
            if let Some(new_name) = &username {
                let existing_name = profile.get("username").and_then(|v| v.as_str());
                if let Some(existing_name) = existing_name {
                    if !existing_name.is_empty() && existing_name.eq_ignore_ascii_case(new_name) {
                        return Err("Profile already exists".to_string());
                    }
                }
            }
        }
    }

    let new_profile = serde_json::json!({
        "url": url,
        "username": username,
        "platform": platform,
        "added_at": Utc::now().timestamp()
    });

    if let Some(arr) = all_profiles
        .as_object_mut()
        .and_then(|obj| obj.get_mut(&platform))
        .and_then(|v| v.as_array_mut())
    {
        arr.push(new_profile);
    } else {
        if let Some(obj) = all_profiles.as_object_mut() {
            obj.insert(platform.clone(), serde_json::json!([new_profile]));
        }
    }

    let content = serde_json::to_string_pretty(&all_profiles)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(&profiles_file, content).map_err(|e| format!("Failed to write profiles: {}", e))?;

    Ok("Profile saved successfully".to_string())
}

#[command]
pub fn open_omni_delete_profile(platform: String, profile_url: String) -> Result<String, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = OPEN_OMNI_STORAGE_LOCK
        .lock()
        .map_err(|_| "Open Omni storage lock is unavailable")?;
    let profiles_file = omniget_open_omni_dir()?.join("profiles.json");

    if !profiles_file.exists() {
        return Err("Profiles file not found".to_string());
    }

    let content = fs::read_to_string(&profiles_file)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;

    let mut all_profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    let deleted = all_profiles
        .as_object_mut()
        .and_then(|obj| obj.get_mut(&platform))
        .and_then(|v| v.as_array_mut())
        .map(|arr| {
            let len_before = arr.len();
            arr.retain(|p| p.get("url").and_then(|u| u.as_str()) != Some(&profile_url));
            len_before != arr.len()
        })
        .unwrap_or(false);

    if !deleted {
        return Err("Profile not found".to_string());
    }

    let content = serde_json::to_string_pretty(&all_profiles)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(&profiles_file, content).map_err(|e| format!("Failed to write profiles: {}", e))?;

    Ok("Profile deleted".to_string())
}

#[command]
pub fn open_omni_setup_folders(base_dir: String, cookies_dir: String) -> Result<String, String> {
    let base_path = PathBuf::from(base_dir);
    let cookies_path = PathBuf::from(cookies_dir);

    fs::create_dir_all(&base_path).map_err(|e| format!("Failed to create base dir: {}", e))?;
    fs::create_dir_all(&cookies_path)
        .map_err(|e| format!("Failed to create cookies dir: {}", e))?;

    let platforms = vec!["instagram", "tiktok", "facebook", "x"];

    for platform in &platforms {
        let platform_dir = base_path.join(platform);
        fs::create_dir_all(&platform_dir)
            .map_err(|e| format!("Failed to create platform dir: {}", e))?;

        let profile_file = base_path.join(format!("{}_profiles.txt", platform));
        if !profile_file.exists() {
            fs::write(
                &profile_file,
                format!("# Add {} profile URLs here\n", platform),
            )
            .map_err(|e| format!("Failed to create profile file: {}", e))?;
        }

        let cookie_file = cookies_path.join(format!("{}.com_cookies.txt", platform));
        if !cookie_file.exists() {
            fs::write(&cookie_file, format!("# Add {} cookies here\n", platform))
                .map_err(|e| format!("Failed to create cookie file: {}", e))?;
        }
    }

    Ok("Folder structure created successfully".to_string())
}
