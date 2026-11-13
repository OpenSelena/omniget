use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub min_omniget_version: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub nav: Vec<PluginNavItem>,
    #[serde(default)]
    pub events: PluginEvents,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub settings_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub rust_crate: Option<String>,
    #[serde(default)]
    pub frontend_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginNavItem {
    pub route: String,
    pub label: HashMap<String, String>,
    #[serde(default)]
    pub icon_svg: Option<String>,
    #[serde(default = "default_nav_group")]
    pub group: NavGroup,
    #[serde(default = "default_nav_order")]
    pub order: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginEvents {
    #[serde(default)]
    pub progress: Vec<String>,
    #[serde(default)]
    pub complete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NavGroup {
    Primary,
    Secondary,
}

impl Default for NavGroup {
    fn default() -> Self {
        NavGroup::Secondary
    }
}

fn default_nav_group() -> NavGroup {
    NavGroup::Secondary
}

fn default_nav_order() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub version: String,
    pub installed_at: String,
    pub updated_at: String,
    pub enabled: bool,
    pub repo: Option<String>,
    pub source_release: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub repo: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    #[serde(default)]
    pub schema_version: u32,
    pub plugins: Vec<RegistryEntry>,
}
