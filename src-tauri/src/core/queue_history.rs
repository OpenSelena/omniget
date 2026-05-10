use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::core::queue::QueueKind;

const HISTORY_FILE: &str = "download-history.json";
const MAX_HISTORY_ENTRIES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub url: String,
    pub platform: String,
    pub title: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_size_bytes: Option<u64>,
    #[serde(default)]
    pub total_bytes: Option<u64>,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub completed_at: i64,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub kind: Option<QueueKind>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct HistoryFile {
    #[serde(default)]
    entries: Vec<HistoryEntry>,
}

fn store() -> &'static Mutex<VecDeque<HistoryEntry>> {
    static STORE: OnceLock<Mutex<VecDeque<HistoryEntry>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn file_path() -> Option<PathBuf> {
    crate::core::paths::app_data_dir().map(|d| d.join(HISTORY_FILE))
}

fn write_to_disk(entries: &VecDeque<HistoryEntry>) {
    let Some(path) = file_path() else { return };
    let Some(parent) = path.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("[history] create_dir_all failed: {}", e);
        return;
    }
    let file_data = HistoryFile {
        entries: entries.iter().cloned().collect(),
    };
    let serialized = match serde_json::to_string_pretty(&file_data) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[history] serialize failed: {}", e);
            return;
        }
    };
    let tmp = path.with_extension("json.tmp");
    let write_result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(serialized.as_bytes())?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_result {
        tracing::warn!("[history] write tmp failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("[history] rename failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn init_from_disk() {
    let Some(path) = file_path() else { return };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed: HistoryFile = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[history] parse failed: {}", e);
            return;
        }
    };
    let mut guard = store().lock().unwrap();
    guard.clear();
    for entry in parsed.entries.into_iter().take(MAX_HISTORY_ENTRIES) {
        guard.push_back(entry);
    }
}

pub fn record(entry: HistoryEntry) {
    let mut guard = store().lock().unwrap();
    guard.retain(|e| e.id != entry.id);
    guard.push_back(entry);
    while guard.len() > MAX_HISTORY_ENTRIES {
        guard.pop_front();
    }
    write_to_disk(&guard);
}

pub fn list() -> Vec<HistoryEntry> {
    let guard = store().lock().unwrap();
    guard.iter().rev().cloned().collect()
}

pub fn remove(id: u64) {
    let mut guard = store().lock().unwrap();
    let before = guard.len();
    guard.retain(|e| e.id != id);
    if guard.len() != before {
        write_to_disk(&guard);
    }
}

pub fn clear_all() {
    let mut guard = store().lock().unwrap();
    guard.clear();
    write_to_disk(&guard);
}

pub fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
