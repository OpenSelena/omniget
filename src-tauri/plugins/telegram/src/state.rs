use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::platforms::telegram::auth::{TelegramSessionHandle, TelegramState};

fn default_max_threads() -> u32 {
    8
}

fn default_sync_interval() -> u32 {
    30
}

fn default_clone_delay_ms() -> u64 {
    2000
}

fn default_clone_batch_size() -> usize {
    50
}

pub fn current_ymd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

pub fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramDcBucket {
    pub dc_id: i32,
    pub tokens: f64,
    pub last_flood_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramPerfState {
    #[serde(default = "default_max_threads")]
    pub max_threads: u32,
    #[serde(default)]
    pub buckets: Vec<TelegramDcBucket>,
}

impl Default for TelegramPerfState {
    fn default() -> Self {
        Self {
            max_threads: 8,
            buckets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramBandwidthState {
    #[serde(default)]
    pub used_today_bytes: u64,
    #[serde(default)]
    pub total_used_bytes: u64,
    #[serde(default)]
    pub quota_bytes: u64,
    #[serde(default)]
    pub date: String,
}

impl TelegramBandwidthState {
    pub fn normalize_day(&mut self) {
        let today = current_ymd();
        if self.date != today {
            self.used_today_bytes = 0;
            self.date = today;
        }
    }

    pub fn record_bytes(&mut self, bytes: u64) {
        self.normalize_day();
        self.used_today_bytes = self.used_today_bytes.saturating_add(bytes);
        self.total_used_bytes = self.total_used_bytes.saturating_add(bytes);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramSyncState {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_sync_interval")]
    pub interval_min: u32,
    #[serde(default)]
    pub last_success_at: u64,
    #[serde(default)]
    pub last_duration_ms: u64,
    #[serde(default)]
    pub last_updated_count: usize,
    #[serde(skip)]
    pub is_syncing: bool,
}

impl Default for TelegramSyncState {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_min: 30,
            last_success_at: 0,
            last_duration_ms: 0,
            last_updated_count: 0,
            is_syncing: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAccountEntry {
    pub id: String,
    pub label: String,
    pub phone_redacted: Option<String>,
    pub user_id: Option<i64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramCloneOptions {
    #[serde(default = "default_clone_delay_ms")]
    pub delay_ms: u64,
    #[serde(default = "default_clone_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub drop_author: bool,
    #[serde(default)]
    pub drop_captions: bool,
}

impl Default for TelegramCloneOptions {
    fn default() -> Self {
        Self {
            delay_ms: default_clone_delay_ms(),
            batch_size: default_clone_batch_size(),
            limit: None,
            drop_author: false,
            drop_captions: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramCloneSession {
    pub id: String,
    pub source_chat_id: i64,
    pub source_chat_type: String,
    pub source_title: String,
    pub dest_chat_id: i64,
    pub dest_chat_type: String,
    pub dest_title: String,
    pub last_message_id: i32,
    pub total_collected: usize,
    pub cloned_count: usize,
    pub failed_count: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub options: TelegramCloneOptions,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramPersistentData {
    #[serde(default)]
    pub perf: TelegramPerfState,
    #[serde(default)]
    pub bandwidth: TelegramBandwidthState,
    #[serde(default)]
    pub sync: TelegramSyncState,
    #[serde(default)]
    pub accounts: Vec<TelegramAccountEntry>,
    #[serde(default)]
    pub clones: Vec<TelegramCloneSession>,
    #[serde(default)]
    pub takeout_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramStreamInfo {
    pub port: u16,
    pub token: String,
    pub base_url: String,
}

pub fn libreget_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("libreget")
}

pub fn persistent_state_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("libreget").join("telegram_plugin_state.json")
}

pub fn accounts_dir() -> PathBuf {
    libreget_data_dir().join("telegram_accounts")
}

pub fn backups_dir() -> PathBuf {
    libreget_data_dir().join("telegram_backups")
}

impl TelegramPersistentData {
    pub fn load() -> Self {
        let path = persistent_state_path();
        let mut data: Self = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        data.bandwidth.normalize_day();
        for clone in &mut data.clones {
            if clone.status == "running" {
                clone.status = "paused".to_string();
            }
        }
        data
    }

    pub fn save(&self) {
        let path = persistent_state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}

pub struct TelegramPluginState {
    pub telegram_session: TelegramSessionHandle,
    pub active_generic_downloads:
        Arc<tokio::sync::Mutex<HashMap<u64, (String, CancellationToken)>>>,
    pub active_ops: Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>>,
    pub persistent: Arc<tokio::sync::Mutex<TelegramPersistentData>>,
    pub stream_server: Arc<tokio::sync::Mutex<Option<TelegramStreamInfo>>>,
}

impl Default for TelegramPluginState {
    fn default() -> Self {
        Self {
            telegram_session: Arc::new(tokio::sync::Mutex::new(TelegramState::new())),
            active_generic_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            active_ops: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            persistent: Arc::new(tokio::sync::Mutex::new(TelegramPersistentData::load())),
            stream_server: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}
