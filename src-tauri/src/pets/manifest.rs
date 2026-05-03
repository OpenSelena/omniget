use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MANIFEST_VERSION: u32 = 1;
const MANIFEST_NAME: &str = "manifest.local.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalManifest {
    pub version: u32,
    pub last_synced_at: Option<String>,
    pub last_remote_manifest_at: Option<String>,
    pub pets: BTreeMap<String, LocalPetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPetEntry {
    pub slug: String,
    pub display_name: String,
    pub source: String,
    pub installed_at: String,
    #[serde(default)]
    pub vibes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub sprite_ext: String,
    #[serde(default)]
    pub file_sizes: BTreeMap<String, u64>,
    #[serde(default)]
    pub remote_checked_at: Option<String>,
}

impl Default for LocalManifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            last_synced_at: None,
            last_remote_manifest_at: None,
            pets: BTreeMap::new(),
        }
    }
}

pub fn pets_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("pets")
}

pub fn manifest_path(app_data_dir: &Path) -> PathBuf {
    pets_root(app_data_dir).join(MANIFEST_NAME)
}

pub fn pet_dir(app_data_dir: &Path, slug: &str) -> PathBuf {
    pets_root(app_data_dir).join(slug)
}

pub fn ensure_root(app_data_dir: &Path) -> std::io::Result<PathBuf> {
    let root = pets_root(app_data_dir);
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn load(app_data_dir: &Path) -> LocalManifest {
    let path = manifest_path(app_data_dir);
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    if raw.is_empty() {
        return LocalManifest::default();
    }
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_atomic(app_data_dir: &Path, manifest: &LocalManifest) -> std::io::Result<()> {
    let _ = ensure_root(app_data_dir)?;
    let final_path = manifest_path(app_data_dir);
    let tmp_path = final_path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp_path, bytes)?;
    if final_path.exists() {
        std::fs::remove_file(&final_path)?;
    }
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn discover_local_pets(
    app_data_dir: &Path,
    manifest: &mut LocalManifest,
) -> std::io::Result<()> {
    let root = pets_root(app_data_dir);
    let mut existing_slugs: Vec<String> = manifest.pets.keys().cloned().collect();
    if !root.exists() {
        return Ok(());
    }
    let entries = match std::fs::read_dir(&root) {
        Ok(it) => it,
        Err(_) => return Ok(()),
    };
    let mut seen_on_disk: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let slug = match entry.file_name().to_str().map(|s| s.to_string()) {
            Some(s) => s,
            None => continue,
        };
        let dir = entry.path();
        let webp = dir.join("spritesheet.webp");
        let png = dir.join("spritesheet.png");
        let ext = if webp.is_file() {
            Some("webp")
        } else if png.is_file() {
            Some("png")
        } else {
            None
        };
        let Some(ext) = ext else { continue };
        seen_on_disk.push(slug.clone());
        if manifest.pets.contains_key(&slug) {
            continue;
        }
        let pet_json = dir.join("pet.json");
        let display_name = std::fs::read_to_string(&pet_json)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| {
                v.get("displayName")
                    .or_else(|| v.get("display_name"))
                    .and_then(|s| s.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| slug.clone());
        manifest.pets.insert(
            slug.clone(),
            LocalPetEntry {
                slug: slug.clone(),
                display_name,
                source: "manual".to_string(),
                installed_at: now_iso(),
                vibes: Vec::new(),
                tags: Vec::new(),
                kind: None,
                sprite_ext: ext.to_string(),
                file_sizes: BTreeMap::new(),
                remote_checked_at: None,
            },
        );
    }
    existing_slugs.retain(|slug| !seen_on_disk.contains(slug));
    for orphan in existing_slugs {
        manifest.pets.remove(&orphan);
    }
    Ok(())
}
