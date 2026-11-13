#![allow(clippy::all)]
#![allow(warnings)]

pub mod args;
pub mod cookies;
pub mod dispatcher;
pub mod fetcher;
pub mod http;
pub mod ledger;
pub mod pull;

use chrono::Utc;
pub use dispatcher::{DownloadProgress, DownloadResult};
pub use http::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::{command, AppHandle};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub url: String,
    pub username: Option<String>,
    pub platform: String,
    pub added_at: i64,
    pub last_pulled_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub output_directory: Option<String>,
    pub cookies_file: Option<String>,
    #[serde(default)]
    pub browser_cookies: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyStatus {
    pub ok: bool,
    pub message: String,
    pub gallery_dl_version: Option<String>,
    pub yt_dlp_version: Option<String>,
    pub auto_provision_available: bool,
}

static NAMI_STORAGE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub fn libreget_nami_dir() -> Result<PathBuf, String> {
    let base =
        libreget_core::core::paths::app_data_dir().ok_or("Could not find app data directory")?;
    let dir = base.join("nami");

    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(dir)
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    let tmp_path = path.with_extension(format!(
        "tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    fs::write(&tmp_path, content).map_err(|e| format!("Failed to write temporary file: {}", e))?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to commit file {}: {}", path.display(), e)
    })?;
    Ok(())
}

fn validate_profile_platform(platform: &str) -> Result<(), String> {
    match platform {
        "instagram" | "tiktok" | "facebook" | "x" => Ok(()),
        _ => Err("Unsupported profile platform".to_string()),
    }
}

fn normalize_profile_url(platform: &str, raw_input: &str) -> String {
    let trimmed = raw_input.trim();

    if !trimmed.contains("://") && !trimmed.contains('/') {
        let username = trimmed.trim_start_matches('@').to_lowercase();
        return match platform {
            "instagram" => format!("https://www.instagram.com/{}/", username),
            "tiktok" => format!("https://www.tiktok.com/@{}", username),
            "facebook" => format!("https://www.facebook.com/{}", username),
            "x" => format!("https://x.com/{}", username),
            _ => trimmed.to_string(),
        };
    }

    let without_scheme = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let (host_and_path, _) = without_scheme
        .split_once(['?', '#'])
        .unwrap_or((without_scheme, ""));

    let mut parts = host_and_path.splitn(2, '/');
    let host = parts.next().unwrap_or("").to_lowercase();
    let path = parts.next().unwrap_or("").trim_matches('/');

    let canonical_host = match platform {
        "instagram" => "www.instagram.com",
        "tiktok" => "www.tiktok.com",
        "facebook" => "www.facebook.com",
        "x" => "x.com",
        _ => host.trim_start_matches("www."),
    };

    let path_part = if path.is_empty() {
        String::new()
    } else {
        format!("/{}/", path.to_lowercase())
    };

    if platform == "tiktok" && !path_part.is_empty() {
        let user = path.trim_start_matches('@');
        return format!("https://www.tiktok.com/@{}", user.to_lowercase());
    }

    if platform == "x" {
        if path.starts_with("i/history/likes") {
            return "https://x.com/i/history/likes".to_string();
        }
        if path.starts_with("i/bookmarks") {
            return "https://x.com/i/bookmarks".to_string();
        }
    }

    format!("https://{}{}", canonical_host, path_part)
}

#[command]
pub async fn nami_check_python_dependencies() -> Result<DependencyStatus, String> {
    let app_dir = libreget_nami_dir().unwrap_or_else(|_| PathBuf::from("."));
    let status = fetcher::check_engines_status(&app_dir);

    let ok = status.ready;
    let message = if ok {
        "All download engines ready".to_string()
    } else {
        "Engine not found (auto-download available)".to_string()
    };

    Ok(DependencyStatus {
        ok,
        message,
        gallery_dl_version: status.gallery_dl_version,
        yt_dlp_version: status.yt_dlp_version,
        auto_provision_available: true,
    })
}

#[command]
pub async fn nami_ensure_binaries() -> Result<DependencyStatus, String> {
    let app_dir = libreget_nami_dir()?;
    let _ = fetcher::ensure_latest_gallery_dl(&app_dir).await;
    let _ = fetcher::ensure_latest_yt_dlp(&app_dir).await;

    nami_check_python_dependencies().await
}

#[command]
pub async fn nami_run_gallery_dl_download(
    app: AppHandle,
    url: String,
    output_dir: String,
    cookies_file: Option<String>,
    content_type: String,
    download_id: String,
    skip_replies: Option<bool>,
) -> Result<DownloadResult, String> {
    let app_dir = libreget_nami_dir()?;
    dispatcher::execute_download_workflow(
        app,
        url,
        output_dir,
        cookies_file,
        content_type,
        download_id,
        app_dir,
        skip_replies,
        None,
    )
    .await
}

#[command]
pub async fn nami_cancel_download(download_id: String) -> Result<String, String> {
    dispatcher::cancel_running_download(&download_id)
}

#[command]
pub fn nami_save_app_settings(
    output_directory: Option<String>,
    cookies_file: Option<String>,
    browser_cookies: Option<String>,
) -> Result<String, String> {
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let settings = AppSettings {
        output_directory,
        cookies_file,
        browser_cookies,
    };

    let config_dir = libreget_nami_dir()?;
    let settings_file = config_dir.join("app_settings.json");
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    atomic_write(&settings_file, &content)?;
    Ok("Settings saved successfully".to_string())
}

#[command]
pub fn nami_load_app_settings() -> Result<AppSettings, String> {
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let settings_file = libreget_nami_dir()?.join("app_settings.json");
    if !settings_file.exists() {
        return Ok(AppSettings {
            output_directory: None,
            cookies_file: None,
            browser_cookies: None,
        });
    }

    let content = fs::read_to_string(&settings_file)
        .map_err(|e| format!("Failed to read settings: {}", e))?;
    let settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;
    Ok(settings)
}

#[command]
pub fn nami_detect_browsers() -> Result<Vec<cookies::DetectedBrowser>, String> {
    Ok(cookies::list_detected_browsers())
}

#[command]
pub fn nami_extract_browser_cookies(
    browser_id: String,
    platform: String,
) -> Result<Option<String>, String> {
    let browsers = cookies::list_detected_browsers();
    let target = if browser_id.eq_ignore_ascii_case("auto") {
        cookies::detect_system_default_browser().unwrap_or_else(|| "chrome".to_string())
    } else {
        browser_id.to_ascii_lowercase()
    };

    let matched = browsers.into_iter().find(|b| b.id == target);
    let browser = match matched {
        Some(b) => b,
        None => {
            return Err(format!(
                "Browser '{}' is not recognized or installed",
                browser_id
            ))
        }
    };

    if !browser.available {
        return Err(format!(
            "Browser '{}' profile directory was not found on this system",
            browser.name
        ));
    }

    if browser.engine == "firefox" {
        let prof_path = PathBuf::from(&browser.profile_dir);
        let gecko_dir = cookies::find_gecko_profile_dir(&prof_path).ok_or_else(|| {
            format!(
                "Could not find cookies.sqlite in '{}' profile",
                browser.name
            )
        })?;
        let domains = cookies::get_platform_cookie_domains(&platform);
        let netscape_data = cookies::extract_gecko_cookies_to_netscape(&gecko_dir, &domains)
            .ok_or_else(|| {
                format!(
                    "No active session cookies found in {} for {}",
                    browser.name, platform
                )
            })?;

        let config_dir = libreget_nami_dir()?;
        let cookies_dir = config_dir.join("cookies");
        fs::create_dir_all(&cookies_dir).map_err(|e| e.to_string())?;

        let file_name = format!("{}.com_cookies.txt", platform.to_lowercase());
        let target_path = cookies_dir.join(&file_name);
        fs::write(&target_path, &netscape_data).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(0o600));
        }

        let line_count = netscape_data
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .count();
        return Ok(Some(format!(
            "Successfully extracted {} cookies from {}",
            line_count, browser.name
        )));
    }

    Ok(Some(format!(
        "{} verified and connected via native browser engine bridge",
        browser.name
    )))
}

#[command]
pub fn nami_load_profiles(platform: String) -> Result<Vec<Profile>, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let profiles_file = libreget_nami_dir()?.join("profiles.json");
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
pub fn nami_save_profile(platform: String, url: String) -> Result<String, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let raw_input = url.trim().to_string();
    if raw_input.is_empty() {
        return Err("URL or username cannot be empty".to_string());
    }

    let url = normalize_profile_url(&platform, &raw_input);
    let config_dir = libreget_nami_dir()?;
    let profiles_file = config_dir.join("profiles.json");

    let mut all_profiles: serde_json::Value = if profiles_file.exists() {
        let content = fs::read_to_string(&profiles_file)
            .map_err(|e| format!("Failed to read profiles: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse profiles file (corrupted): {}", e))?
    } else {
        serde_json::json!({})
    };

    let username = dispatcher::extract_username_generic(&url);

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
            if let Some(existing) = existing_url {
                if normalize_profile_url(&platform, existing) == url {
                    return Err("Profile already exists".to_string());
                }
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
    } else if let Some(obj) = all_profiles.as_object_mut() {
        obj.insert(platform.clone(), serde_json::json!([new_profile]));
    }

    let content = serde_json::to_string_pretty(&all_profiles)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    atomic_write(&profiles_file, &content)?;
    Ok("Profile saved successfully".to_string())
}

#[command]
pub fn nami_delete_profile(platform: String, profile_url: String) -> Result<String, String> {
    validate_profile_platform(&platform)?;
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let profiles_file = libreget_nami_dir()?.join("profiles.json");
    if !profiles_file.exists() {
        return Err("Profiles file not found".to_string());
    }

    let content = fs::read_to_string(&profiles_file)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    let mut all_profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    let normalized_target = normalize_profile_url(&platform, &profile_url);

    let deleted = all_profiles
        .as_object_mut()
        .and_then(|obj| obj.get_mut(&platform))
        .and_then(|v| v.as_array_mut())
        .map(|arr| {
            let len_before = arr.len();
            arr.retain(|p| {
                let u = p.get("url").and_then(|u| u.as_str()).unwrap_or("");
                u != profile_url && normalize_profile_url(&platform, u) != normalized_target
            });
            len_before != arr.len()
        })
        .unwrap_or(false);

    if !deleted {
        return Err("Profile not found".to_string());
    }

    let content = serde_json::to_string_pretty(&all_profiles)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    atomic_write(&profiles_file, &content)?;
    Ok("Profile deleted".to_string())
}

#[command]
pub async fn nami_get_history(
    platform: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<ledger::NamiMediaRecord>, String> {
    ledger::get_history(
        platform.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
}

#[command]
pub async fn nami_reconcile_disk() -> Result<ledger::ReconcileResult, String> {
    ledger::reconcile_disk()
}

#[command]
pub async fn nami_delete_history_record(id: String) -> Result<bool, String> {
    ledger::delete_record(&id)
}

pub fn update_profile_last_pulled(
    platform: &str,
    profile_url: &str,
    timestamp: i64,
) -> Result<(), String> {
    let _storage_guard = NAMI_STORAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let profiles_file = libreget_nami_dir()?.join("profiles.json");
    if !profiles_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&profiles_file)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    let mut all_profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    let normalized_target = normalize_profile_url(platform, profile_url);

    if let Some(arr) = all_profiles
        .as_object_mut()
        .and_then(|obj| obj.get_mut(platform))
        .and_then(|v| v.as_array_mut())
    {
        for p in arr.iter_mut() {
            let u = p.get("url").and_then(|u| u.as_str()).unwrap_or("");
            if u == profile_url || normalize_profile_url(platform, u) == normalized_target {
                if let Some(obj) = p.as_object_mut() {
                    obj.insert("last_pulled_at".to_string(), serde_json::json!(timestamp));
                }
                break;
            }
        }
    }

    let serialized = serde_json::to_string_pretty(&all_profiles)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    atomic_write(&profiles_file, &serialized)?;
    Ok(())
}

#[command]
pub async fn nami_pull_creator(
    app: AppHandle,
    profile_url: String,
    platform: String,
    content_type: Option<String>,
    early_exit_threshold: Option<u32>,
    download_id: Option<String>,
) -> Result<pull::PullResult, String> {
    pull::pull_single_creator(
        app,
        profile_url,
        platform,
        content_type,
        early_exit_threshold,
        download_id,
    )
    .await
}

#[command]
pub async fn nami_pull_all_creators(
    app: AppHandle,
    platform: Option<String>,
    content_type: Option<String>,
    early_exit_threshold: Option<u32>,
) -> Result<pull::PullBatchSummary, String> {
    pull::pull_all_creators(app, platform, content_type, early_exit_threshold).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_profile_url() {
        assert_eq!(
            normalize_profile_url("instagram", "selenagomez"),
            "https://www.instagram.com/selenagomez/"
        );
        assert_eq!(
            normalize_profile_url("tiktok", "@creator"),
            "https://www.tiktok.com/@creator"
        );
        assert_eq!(
            normalize_profile_url("x", "OpenSelena"),
            "https://x.com/openselena"
        );
    }
}
