use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CHANNELS_FILE: &str = "channels.json";
const MAX_SEEN_IDS: usize = 500;

fn default_true() -> bool {
    true
}

fn default_interval() -> u32 {
    60
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelFollow {
    pub id: String,
    pub url: String,
    pub title: String,
    pub added_at_ms: u64,
    #[serde(default)]
    pub last_checked_ms: Option<u64>,
    #[serde(default)]
    pub seen_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_download: bool,
    #[serde(default = "default_interval")]
    pub interval_minutes: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ChannelsFile {
    #[serde(default)]
    channels: Vec<ChannelFollow>,
}

static STORE: OnceLock<Mutex<HashMap<String, ChannelFollow>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, ChannelFollow>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn file_path() -> Option<PathBuf> {
    crate::core::paths::app_data_dir().map(|d| d.join(CHANNELS_FILE))
}

pub fn id_for_url(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.trim().hash(&mut hasher);
    format!("ch{:016x}", hasher.finish())
}

fn write_to_disk(channels: &HashMap<String, ChannelFollow>) {
    let Some(path) = file_path() else { return };
    let Some(parent) = path.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("[channels] create_dir_all failed: {}", e);
        return;
    }
    let file_data = ChannelsFile {
        channels: channels.values().cloned().collect(),
    };
    let serialized = match serde_json::to_string_pretty(&file_data) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[channels] serialize failed: {}", e);
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
        tracing::warn!("[channels] write tmp failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("[channels] rename failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn init_from_disk() {
    let Some(path) = file_path() else { return };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed: ChannelsFile = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[channels] parse failed: {}", e);
            return;
        }
    };
    let mut guard = store().lock().unwrap();
    guard.clear();
    for ch in parsed.channels {
        guard.insert(ch.id.clone(), ch);
    }
}

pub fn list() -> Vec<ChannelFollow> {
    let guard = store().lock().unwrap();
    let mut v: Vec<ChannelFollow> = guard.values().cloned().collect();
    v.sort_by(|a, b| a.added_at_ms.cmp(&b.added_at_ms));
    v
}

pub fn get(id: &str) -> Option<ChannelFollow> {
    store().lock().unwrap().get(id).cloned()
}

pub fn add(url: String, title: String) -> ChannelFollow {
    let id = id_for_url(&url);
    let mut guard = store().lock().unwrap();
    let entry = guard.entry(id.clone()).or_insert_with(|| ChannelFollow {
        id: id.clone(),
        url: url.trim().to_string(),
        title: title.clone(),
        added_at_ms: now_ms(),
        last_checked_ms: None,
        seen_ids: Vec::new(),
        enabled: true,
        auto_download: false,
        interval_minutes: default_interval(),
    });
    if !title.is_empty() {
        entry.title = title;
    }
    let result = entry.clone();
    write_to_disk(&guard);
    result
}

pub fn remove(id: &str) -> bool {
    let mut guard = store().lock().unwrap();
    if guard.remove(id).is_some() {
        write_to_disk(&guard);
        true
    } else {
        false
    }
}

pub fn update(
    id: &str,
    enabled: Option<bool>,
    auto_download: Option<bool>,
    interval_minutes: Option<u32>,
) -> Option<ChannelFollow> {
    let mut guard = store().lock().unwrap();
    let ch = guard.get_mut(id)?;
    if let Some(v) = enabled {
        ch.enabled = v;
    }
    if let Some(v) = auto_download {
        ch.auto_download = v;
    }
    if let Some(v) = interval_minutes {
        ch.interval_minutes = v.max(5);
    }
    let result = ch.clone();
    write_to_disk(&guard);
    Some(result)
}

// Records the poll result: marks checked-now and folds newly seen video ids
// into the channel's bounded seen set. Returns the ids that were NOT seen
// before (the genuinely new videos) so the caller can notify/auto-download.
pub fn record_poll(id: &str, fetched_ids: &[String]) -> Vec<String> {
    let mut guard = store().lock().unwrap();
    let Some(ch) = guard.get_mut(id) else {
        return Vec::new();
    };
    ch.last_checked_ms = Some(now_ms());

    // First successful poll establishes a baseline without flagging the whole
    // back catalogue as "new".
    let first_poll = ch.seen_ids.is_empty();
    let known: std::collections::HashSet<&String> = ch.seen_ids.iter().collect();
    let new_ids: Vec<String> = fetched_ids
        .iter()
        .filter(|fid| !known.contains(fid))
        .cloned()
        .collect();

    for fid in fetched_ids {
        if !ch.seen_ids.iter().any(|s| s == fid) {
            ch.seen_ids.push(fid.clone());
        }
    }
    if ch.seen_ids.len() > MAX_SEEN_IDS {
        let overflow = ch.seen_ids.len() - MAX_SEEN_IDS;
        ch.seen_ids.drain(0..overflow);
    }

    write_to_disk(&guard);
    if first_poll {
        Vec::new()
    } else {
        new_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_stable_and_trimmed() {
        assert_eq!(
            id_for_url("https://x.test/c"),
            id_for_url("  https://x.test/c  ")
        );
        assert_ne!(
            id_for_url("https://x.test/a"),
            id_for_url("https://x.test/b")
        );
        assert!(id_for_url("https://x.test/c").starts_with("ch"));
    }
}
