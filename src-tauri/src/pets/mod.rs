pub mod download;
pub mod install;
pub mod manifest;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use download::{fetch_manifest, RemoteManifest};
use install::{
    diff as compute_diff, force_refresh as install_force_refresh,
    install_bundle as install_bundle_impl, install_missing as install_missing_impl,
    refresh_index, uninstall as uninstall_impl, DiffReport, InstallReport, LocalIndex,
};
use manifest::{ensure_root, load, now_iso, pets_root, save_atomic};

#[derive(Default)]
pub struct PetsState {
    pub active_slug: Mutex<Option<String>>,
    pub remote_cache: Mutex<Option<RemoteManifest>>,
    pub display_prefs_cache: Mutex<Option<DisplayPrefs>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayPrefs {
    pub enabled: bool,
    pub motion_enabled: bool,
    pub default_state: String,
    pub event_overrides: serde_json::Map<String, serde_json::Value>,
}

impl Default for DisplayPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            motion_enabled: true,
            default_state: "idle".to_string(),
            event_overrides: serde_json::Map::new(),
        }
    }
}

fn prefs_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_path(app)?;
    Ok(pets_root(&dir).join("display_prefs.json"))
}

fn read_prefs(app: &AppHandle) -> DisplayPrefs {
    let Ok(path) = prefs_file(app) else {
        return DisplayPrefs::default();
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return DisplayPrefs::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_prefs(app: &AppHandle, prefs: &DisplayPrefs) -> Result<(), String> {
    let dir = app_data_path(app)?;
    ensure_root(&dir).map_err(|e| e.to_string())?;
    let path = prefs_file(app)?;
    let body = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePref {
    pub slug: String,
}

fn app_data_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {}", e))
}

fn pref_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_path(app)?;
    Ok(pets_root(&dir).join("active.json"))
}

fn read_active(app: &AppHandle) -> Option<String> {
    let path = pref_file(app).ok()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v.get("slug").and_then(|s| s.as_str()).map(|s| s.to_string())
}

fn write_active(app: &AppHandle, slug: &str) -> Result<(), String> {
    let dir = app_data_path(app)?;
    ensure_root(&dir).map_err(|e| e.to_string())?;
    let path = pref_file(app)?;
    let body = serde_json::json!({ "slug": slug });
    std::fs::write(&path, serde_json::to_vec_pretty(&body).unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pets_get_local_index(app: AppHandle) -> Result<LocalIndex, String> {
    let dir = app_data_path(&app)?;
    refresh_index(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pets_fetch_remote_manifest(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
) -> Result<RemoteManifest, String> {
    let manifest = fetch_manifest().await.map_err(|e| e.to_string())?;
    *state.remote_cache.lock().await = Some(manifest.clone());
    let _ = app;
    Ok(manifest)
}

#[tauri::command]
pub async fn pets_diff(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
) -> Result<DiffReport, String> {
    let dir = app_data_path(&app)?;
    let local = refresh_index(&dir).map_err(|e| e.to_string())?;
    let remote = fetch_manifest().await.map_err(|e| e.to_string())?;
    *state.remote_cache.lock().await = Some(remote.clone());
    let mut local_manifest = load(&dir);
    local_manifest.last_remote_manifest_at = remote.generated_at.clone();
    let _ = save_atomic(&dir, &local_manifest);
    Ok(compute_diff(&local, &remote))
}

#[tauri::command]
pub async fn pets_install_bundle(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
) -> Result<InstallReport, String> {
    let dir = app_data_path(&app)?;
    let remote = match state.remote_cache.lock().await.clone() {
        Some(m) => m,
        None => fetch_manifest().await.map_err(|e| e.to_string())?,
    };
    Ok(install_bundle_impl(app, dir, remote).await)
}

#[tauri::command]
pub async fn pets_install_missing(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
    slugs: Option<Vec<String>>,
) -> Result<InstallReport, String> {
    let dir = app_data_path(&app)?;
    let remote = match state.remote_cache.lock().await.clone() {
        Some(m) => m,
        None => fetch_manifest().await.map_err(|e| e.to_string())?,
    };
    Ok(install_missing_impl(app, dir, remote, slugs).await)
}

#[tauri::command]
pub async fn pets_force_refresh(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
    slug: String,
) -> Result<InstallReport, String> {
    let dir = app_data_path(&app)?;
    let remote = match state.remote_cache.lock().await.clone() {
        Some(m) => m,
        None => fetch_manifest().await.map_err(|e| e.to_string())?,
    };
    Ok(install_force_refresh(app, dir, remote, slug).await)
}

#[tauri::command]
pub async fn pets_uninstall(app: AppHandle, slug: String) -> Result<(), String> {
    let dir = app_data_path(&app)?;
    uninstall_impl(&dir, &slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pets_open_folder(app: AppHandle) -> Result<(), String> {
    let dir = app_data_path(&app)?;
    let root = pets_root(&dir);
    ensure_root(&dir).map_err(|e| e.to_string())?;
    open_native_folder(&root)
}

fn open_native_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn pets_set_active(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
    slug: String,
) -> Result<(), String> {
    write_active(&app, &slug)?;
    *state.active_slug.lock().await = Some(slug);
    Ok(())
}

#[tauri::command]
pub async fn pets_get_active(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
) -> Result<Option<String>, String> {
    {
        let g = state.active_slug.lock().await;
        if let Some(s) = g.clone() {
            return Ok(Some(s));
        }
    }
    let stored = read_active(&app);
    *state.active_slug.lock().await = stored.clone();
    Ok(stored)
}

#[tauri::command]
pub async fn pets_get_display_prefs(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
) -> Result<DisplayPrefs, String> {
    {
        let g = state.display_prefs_cache.lock().await;
        if let Some(p) = g.clone() {
            return Ok(p);
        }
    }
    let prefs = read_prefs(&app);
    *state.display_prefs_cache.lock().await = Some(prefs.clone());
    Ok(prefs)
}

#[tauri::command]
pub async fn pets_set_display_prefs(
    app: AppHandle,
    state: tauri::State<'_, PetsState>,
    prefs: DisplayPrefs,
) -> Result<DisplayPrefs, String> {
    write_prefs(&app, &prefs)?;
    *state.display_prefs_cache.lock().await = Some(prefs.clone());
    let _ = app.emit("pets:prefs:changed", &prefs);
    Ok(prefs)
}

#[tauri::command]
pub async fn pets_resolve_path(app: AppHandle, slug: String) -> Result<String, String> {
    let dir = app_data_path(&app)?;
    let pet_dir = pets_root(&dir).join(&slug);
    let webp = pet_dir.join("spritesheet.webp");
    let png = pet_dir.join("spritesheet.png");
    let path = if webp.is_file() {
        webp
    } else if png.is_file() {
        png
    } else {
        return Err(format!("no spritesheet for slug '{}'", slug));
    };
    Ok(path.to_string_lossy().into_owned())
}

pub fn _now_iso_export() -> String {
    now_iso()
}
