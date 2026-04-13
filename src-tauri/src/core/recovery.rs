use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const RECOVERY_FILE: &str = "recovery.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryItem {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub platform: String,
    pub output_dir: String,
    #[serde(default)]
    pub download_mode: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub format_id: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecoveryFile {
    #[serde(default)]
    items: Vec<RecoveryItem>,
}

static STORE: OnceLock<Mutex<HashMap<u64, RecoveryItem>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<u64, RecoveryItem>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn file_path() -> Option<PathBuf> {
    crate::core::paths::app_data_dir().map(|d| d.join(RECOVERY_FILE))
}

fn write_to_disk(items: &HashMap<u64, RecoveryItem>) {
    let Some(path) = file_path() else { return };
    let Some(parent) = path.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("[recovery] create_dir_all failed: {}", e);
        return;
    }
    let file_data = RecoveryFile {
        items: items.values().cloned().collect(),
    };
    let serialized = match serde_json::to_string_pretty(&file_data) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[recovery] serialize failed: {}", e);
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
        tracing::warn!("[recovery] write tmp failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("[recovery] rename failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn init_from_disk() {
    let Some(path) = file_path() else { return };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed: RecoveryFile = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[recovery] parse failed: {}", e);
            return;
        }
    };
    let mut guard = store().lock().unwrap();
    guard.clear();
    for item in parsed.items {
        guard.insert(item.id, item);
    }
}

pub fn persist(item: RecoveryItem) {
    let mut guard = store().lock().unwrap();
    guard.insert(item.id, item);
    write_to_disk(&guard);
}

pub fn remove(id: u64) {
    let mut guard = store().lock().unwrap();
    if guard.remove(&id).is_some() {
        write_to_disk(&guard);
    }
}

pub fn list() -> Vec<RecoveryItem> {
    let guard = store().lock().unwrap();
    guard.values().cloned().collect()
}

pub fn clear_all() {
    let mut guard = store().lock().unwrap();
    guard.clear();
    write_to_disk(&guard);
}
