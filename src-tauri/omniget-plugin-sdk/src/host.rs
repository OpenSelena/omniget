use std::path::PathBuf;

pub trait PluginHost: Send + Sync {
    fn emit_event(&self, name: &str, payload: serde_json::Value) -> anyhow::Result<()>;
    fn show_toast(&self, toast_type: &str, message: &str) -> anyhow::Result<()>;
    fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf;
    fn plugin_frontend_dir(&self, plugin_id: &str) -> PathBuf;
    fn get_settings(&self, plugin_id: &str) -> serde_json::Value;
    fn save_settings(&self, plugin_id: &str, settings: serde_json::Value) -> anyhow::Result<()>;
    fn proxy_config(&self) -> Option<ProxyConfig>;
    fn tool_path(&self, tool: &str) -> Option<PathBuf>;
    fn default_output_dir(&self) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}
