use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use omniget_core::models::settings::AppSettings;

const STORE_PATH: &str = "settings.json";
const STORE_KEY: &str = "app_settings";

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let store = match app.store(STORE_PATH) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };

    match store.get(STORE_KEY) {
        Some(val) => serde_json::from_value::<AppSettings>(val.clone()).unwrap_or_default(),
        None => AppSettings::default(),
    }
}
