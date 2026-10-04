use omniget_core::models::settings::AppSettings;

pub fn load_settings() -> AppSettings {
    if let Some(data_dir) = dirs::data_dir() {
        let path = data_dir
            .join("com.openselena.omniget")
            .join("settings.json");
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(app_settings) = val.get("app_settings") {
                    if let Ok(settings) =
                        serde_json::from_value::<AppSettings>(app_settings.clone())
                    {
                        return settings;
                    }
                }
            }
        }
    }
    AppSettings::default()
}
