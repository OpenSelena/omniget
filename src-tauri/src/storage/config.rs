use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::models::settings::AppSettings;

const STORE_PATH: &str = "settings.json";
const STORE_KEY: &str = "app_settings";

/// Absolute path of the settings store, anchored to the app data dir that
/// honors OMNIGET_DATA_DIR. A relative store path resolves against Tauri's
/// own AppData dir, which ignores the portable-mode override — the store
/// writers and the standalone readers below would then use different files.
fn store_path() -> std::path::PathBuf {
    match crate::core::paths::app_data_dir() {
        Some(dir) => dir.join(STORE_PATH),
        None => std::path::PathBuf::from(STORE_PATH),
    }
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let store = match app.store(store_path()) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };

    match store.get(STORE_KEY) {
        Some(val) => serde_json::from_value::<AppSettings>(val.clone()).unwrap_or_default(),
        None => AppSettings::default(),
    }
}

pub fn load_settings_standalone() -> AppSettings {
    let data_dir = match crate::core::paths::app_data_dir() {
        Some(d) => d,
        None => return AppSettings::default(),
    };

    let store_path = data_dir.join(STORE_PATH);
    let content = match std::fs::read_to_string(&store_path) {
        Ok(c) => c,
        Err(_) => return AppSettings::default(),
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return AppSettings::default(),
    };

    match json.get(STORE_KEY) {
        Some(val) => serde_json::from_value::<AppSettings>(val.clone()).unwrap_or_default(),
        None => AppSettings::default(),
    }
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> anyhow::Result<()> {
    let store = app.store(store_path())?;
    let val = serde_json::to_value(settings)?;
    store.set(STORE_KEY, val);
    store.save()?;
    Ok(())
}
