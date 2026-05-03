use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use super::download::{
    extract_ext_from_url, fallback_bundle_url, fetch_bundle_zip, fetch_pet_json,
    fetch_spritesheet, RemoteManifest, RemotePet,
};
use super::manifest::{
    discover_local_pets, ensure_root, load, manifest_path, now_iso, pet_dir, save_atomic,
    LocalManifest, LocalPetEntry,
};

#[derive(Debug, Serialize, Clone)]
pub struct DiffReport {
    pub new_remote: Vec<String>,
    pub in_both: Vec<String>,
    pub only_local: Vec<String>,
    pub total_remote: u32,
    pub total_local: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstallReport {
    pub added: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<FailedInstall>,
    pub total_bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct FailedInstall {
    pub slug: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LocalIndex {
    pub pets: Vec<LocalPetEntry>,
    pub last_synced_at: Option<String>,
    pub total: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressEvent<'a> {
    pub slug: &'a str,
    pub current: u32,
    pub total: u32,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct StartedEvent {
    pub kind: String,
    pub expected: u32,
}

pub fn refresh_index(app_data_dir: &Path) -> std::io::Result<LocalIndex> {
    let _ = ensure_root(app_data_dir)?;
    let mut manifest = load(app_data_dir);
    discover_local_pets(app_data_dir, &mut manifest)?;
    save_atomic(app_data_dir, &manifest)?;
    let mut pets: Vec<LocalPetEntry> = manifest.pets.values().cloned().collect();
    pets.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    let total = pets.len();
    Ok(LocalIndex {
        pets,
        last_synced_at: manifest.last_synced_at.clone(),
        total,
    })
}

pub fn diff(local: &LocalIndex, remote: &RemoteManifest) -> DiffReport {
    let local_set: std::collections::HashSet<String> =
        local.pets.iter().map(|p| p.slug.clone()).collect();
    let remote_set: std::collections::HashSet<String> =
        remote.pets.iter().map(|p| p.slug.clone()).collect();
    let mut new_remote: Vec<String> = remote
        .pets
        .iter()
        .filter(|p| !local_set.contains(&p.slug))
        .map(|p| p.slug.clone())
        .collect();
    let mut in_both: Vec<String> = remote
        .pets
        .iter()
        .filter(|p| local_set.contains(&p.slug))
        .map(|p| p.slug.clone())
        .collect();
    let mut only_local: Vec<String> = local
        .pets
        .iter()
        .filter(|p| !remote_set.contains(&p.slug))
        .map(|p| p.slug.clone())
        .collect();
    new_remote.sort();
    in_both.sort();
    only_local.sort();
    DiffReport {
        new_remote,
        in_both,
        only_local,
        total_remote: remote.total,
        total_local: local.pets.len() as u32,
    }
}

async fn install_one(
    app_data_dir: Arc<std::path::PathBuf>,
    pet: RemotePet,
    overwrite: bool,
) -> Result<(String, u64, LocalPetEntry), FailedInstall> {
    let slug = pet.slug.clone();
    let dir = pet_dir(app_data_dir.as_path(), &slug);
    if dir.exists() && !overwrite {
        return Err(FailedInstall {
            slug,
            reason: "already installed".to_string(),
        });
    }
    let sprite_url = match pet.spritesheet_url.as_ref() {
        Some(u) => u.clone(),
        None => {
            return Err(FailedInstall {
                slug,
                reason: "missing spritesheet url".to_string(),
            });
        }
    };
    let pet_json_url = match pet.pet_json_url.as_ref() {
        Some(u) => u.clone(),
        None => {
            return Err(FailedInstall {
                slug,
                reason: "missing pet.json url".to_string(),
            });
        }
    };

    let pet_json_bytes = match fetch_pet_json(&pet_json_url).await {
        Ok(b) => b,
        Err(e) => {
            return Err(FailedInstall {
                slug,
                reason: format!("pet.json: {}", e),
            });
        }
    };
    if serde_json::from_slice::<serde_json::Value>(&pet_json_bytes).is_err() {
        return Err(FailedInstall {
            slug,
            reason: "pet.json invalid".to_string(),
        });
    }

    let sprite_bytes = match fetch_spritesheet(&sprite_url).await {
        Ok(b) => b,
        Err(e) => {
            return Err(FailedInstall {
                slug,
                reason: format!("sprite: {}", e),
            });
        }
    };

    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(FailedInstall {
            slug,
            reason: format!("mkdir: {}", e),
        });
    }
    let sprite_ext = extract_ext_from_url(&sprite_url);
    let sprite_path = dir.join(format!("spritesheet.{}", sprite_ext));
    let pet_json_path = dir.join("pet.json");
    let metadata_path = dir.join("metadata.json");
    if let Err(e) = atomic_write(&sprite_path, &sprite_bytes) {
        return Err(FailedInstall {
            slug,
            reason: format!("sprite write: {}", e),
        });
    }
    if let Err(e) = atomic_write(&pet_json_path, &pet_json_bytes) {
        return Err(FailedInstall {
            slug,
            reason: format!("pet.json write: {}", e),
        });
    }
    let metadata = serde_json::json!({
        "id": &slug,
        "slug": &slug,
        "displayName": &pet.display_name,
        "description": pet.description.clone().unwrap_or_default(),
        "spritesheetPath": format!("/pets/{}/spritesheet.{}", slug, sprite_ext),
        "petJsonPath": format!("/pets/{}/pet.json", slug),
        "kind": pet.kind.clone().unwrap_or_default(),
        "vibes": pet.vibes.clone(),
        "tags": pet.tags.clone(),
        "featured": pet.featured,
        "submittedBy": pet.submitted_by.clone(),
        "importedAt": now_iso(),
    });
    let metadata_bytes = serde_json::to_vec_pretty(&metadata).unwrap_or_default();
    let _ = atomic_write(&metadata_path, &metadata_bytes);

    let bytes_total = sprite_bytes.len() as u64 + pet_json_bytes.len() as u64;
    let mut sizes: BTreeMap<String, u64> = BTreeMap::new();
    sizes.insert("spritesheet".into(), sprite_bytes.len() as u64);
    sizes.insert("pet.json".into(), pet_json_bytes.len() as u64);
    let entry = LocalPetEntry {
        slug: slug.clone(),
        display_name: pet.display_name.clone(),
        source: "manifest".to_string(),
        installed_at: now_iso(),
        vibes: pet.vibes.clone(),
        tags: pet.tags.clone(),
        kind: pet.kind.clone(),
        sprite_ext: sprite_ext.to_string(),
        file_sizes: sizes,
        remote_checked_at: Some(now_iso()),
    };
    Ok((slug, bytes_total, entry))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("dat")
    ));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub async fn install_missing(
    app: AppHandle,
    app_data_dir: std::path::PathBuf,
    remote: RemoteManifest,
    requested: Option<Vec<String>>,
) -> InstallReport {
    let started = std::time::Instant::now();
    let _ = ensure_root(&app_data_dir);
    let manifest_arc: Arc<Mutex<LocalManifest>> = Arc::new(Mutex::new(load(&app_data_dir)));
    let app_data_arc = Arc::new(app_data_dir.clone());

    let local_known: std::collections::HashSet<String> = {
        let g = manifest_arc.lock().await;
        g.pets.keys().cloned().collect()
    };

    let target_pets: Vec<RemotePet> = remote
        .pets
        .into_iter()
        .filter(|p| {
            if local_known.contains(&p.slug) {
                return false;
            }
            match &requested {
                Some(list) => list.iter().any(|s| s == &p.slug),
                None => true,
            }
        })
        .collect();

    let total = target_pets.len() as u32;
    let _ = app.emit(
        "pets:sync:started",
        StartedEvent {
            kind: "incremental".to_string(),
            expected: total,
        },
    );

    let mut added: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut failed: Vec<FailedInstall> = Vec::new();
    let mut total_bytes: u64 = 0;

    let mut futures = FuturesUnordered::new();
    let max_parallel = 4;

    let mut iter = target_pets.into_iter();
    for _ in 0..max_parallel {
        if let Some(pet) = iter.next() {
            let app_data_arc = app_data_arc.clone();
            futures.push(install_one(app_data_arc, pet, false));
        }
    }

    let mut current: u32 = 0;
    while let Some(result) = futures.next().await {
        current += 1;
        match result {
            Ok((slug, bytes, entry)) => {
                total_bytes += bytes;
                let mut g = manifest_arc.lock().await;
                g.pets.insert(slug.clone(), entry);
                g.last_synced_at = Some(now_iso());
                let _ = save_atomic(&app_data_dir, &g);
                drop(g);
                let _ = app.emit(
                    "pets:sync:progress",
                    ProgressEvent {
                        slug: &slug,
                        current,
                        total,
                        bytes_downloaded: total_bytes,
                        bytes_total: 0,
                    },
                );
                added.push(slug);
            }
            Err(err) => {
                if err.reason == "already installed" {
                    skipped.push(err.slug);
                } else {
                    let _ = app.emit(
                        "pets:sync:error",
                        serde_json::json!({
                            "slug": err.slug,
                            "message": err.reason,
                            "retryable": false,
                        }),
                    );
                    failed.push(err);
                }
            }
        }
        if let Some(pet) = iter.next() {
            let app_data_arc = app_data_arc.clone();
            futures.push(install_one(app_data_arc, pet, false));
        }
    }

    {
        let mut g = manifest_arc.lock().await;
        g.last_synced_at = Some(now_iso());
        g.last_remote_manifest_at = remote.generated_at.clone();
        let _ = save_atomic(&app_data_dir, &g);
    }

    let report = InstallReport {
        added,
        skipped,
        failed,
        total_bytes,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    let _ = app.emit("pets:sync:finished", &report);
    report
}

pub async fn install_bundle(
    app: AppHandle,
    app_data_dir: std::path::PathBuf,
    remote: RemoteManifest,
) -> InstallReport {
    let started = std::time::Instant::now();
    let _ = ensure_root(&app_data_dir);
    let bundle_url = remote
        .all_pets_pack_path
        .clone()
        .unwrap_or_else(|| fallback_bundle_url().to_string());
    let _ = app.emit(
        "pets:sync:started",
        StartedEvent {
            kind: "bundle".to_string(),
            expected: remote.total,
        },
    );

    let zip_bytes = match fetch_bundle_zip(&bundle_url).await {
        Ok(b) => b,
        Err(e) => {
            let report = InstallReport {
                added: vec![],
                skipped: vec![],
                failed: vec![FailedInstall {
                    slug: "bundle".to_string(),
                    reason: format!("bundle download: {}", e),
                }],
                total_bytes: 0,
                duration_ms: started.elapsed().as_millis() as u64,
            };
            let _ = app.emit("pets:sync:finished", &report);
            return report;
        }
    };

    let mut manifest = load(&app_data_dir);
    let local_known: std::collections::HashSet<String> = manifest.pets.keys().cloned().collect();
    let mut added: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut failed: Vec<FailedInstall> = Vec::new();
    let mut total_bytes: u64 = 0;

    let result = (|| -> anyhow::Result<()> {
        let cursor = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut by_slug: std::collections::BTreeMap<String, Vec<(String, Vec<u8>)>> =
            std::collections::BTreeMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let raw_name = match entry.enclosed_name() {
                Some(n) => n.to_path_buf(),
                None => continue,
            };
            let parts: Vec<String> = raw_name
                .components()
                .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
                .collect();
            if parts.len() < 2 {
                continue;
            }
            let slug = parts[parts.len() - 2].clone();
            let file_name = parts[parts.len() - 1].clone();
            let mut bytes: Vec<u8> = Vec::new();
            entry.read_to_end(&mut bytes)?;
            by_slug.entry(slug).or_default().push((file_name, bytes));
        }

        for (slug, files) in by_slug {
            if local_known.contains(&slug) {
                skipped.push(slug);
                continue;
            }
            let dir = pet_dir(&app_data_dir, &slug);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                failed.push(FailedInstall {
                    slug: slug.clone(),
                    reason: format!("mkdir: {}", e),
                });
                continue;
            }
            let mut sprite_ext = "webp".to_string();
            let mut sizes: BTreeMap<String, u64> = BTreeMap::new();
            let mut display_name = slug.clone();
            let mut vibes: Vec<String> = Vec::new();
            let mut tags: Vec<String> = Vec::new();
            let mut kind: Option<String> = None;
            let mut wrote_sprite = false;
            for (name, bytes) in files {
                let lower = name.to_lowercase();
                let dest = dir.join(&name);
                if lower == "spritesheet.webp" || lower == "spritesheet.png" {
                    sprite_ext = if lower.ends_with(".png") { "png".into() } else { "webp".into() };
                    if atomic_write(&dest, &bytes).is_ok() {
                        sizes.insert("spritesheet".into(), bytes.len() as u64);
                        wrote_sprite = true;
                    }
                } else if lower == "pet.json" {
                    if atomic_write(&dest, &bytes).is_ok() {
                        sizes.insert("pet.json".into(), bytes.len() as u64);
                        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if let Some(name) = v.get("displayName").and_then(|s| s.as_str()) {
                                display_name = name.to_string();
                            }
                        }
                    }
                } else if lower == "metadata.json" {
                    if atomic_write(&dest, &bytes).is_ok() {
                        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if let Some(name) = v.get("displayName").and_then(|s| s.as_str()) {
                                display_name = name.to_string();
                            }
                            if let Some(arr) = v.get("vibes").and_then(|s| s.as_array()) {
                                vibes = arr
                                    .iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect();
                            }
                            if let Some(arr) = v.get("tags").and_then(|s| s.as_array()) {
                                tags = arr
                                    .iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect();
                            }
                            if let Some(k) = v.get("kind").and_then(|s| s.as_str()) {
                                kind = Some(k.to_string());
                            }
                        }
                    }
                } else {
                    let _ = atomic_write(&dest, &bytes);
                }
                total_bytes += bytes.len() as u64;
            }
            if !wrote_sprite {
                failed.push(FailedInstall {
                    slug: slug.clone(),
                    reason: "no spritesheet in bundle entry".to_string(),
                });
                continue;
            }
            let entry = LocalPetEntry {
                slug: slug.clone(),
                display_name,
                source: "bundle".to_string(),
                installed_at: now_iso(),
                vibes,
                tags,
                kind,
                sprite_ext,
                file_sizes: sizes,
                remote_checked_at: Some(now_iso()),
            };
            manifest.pets.insert(slug.clone(), entry);
            let _ = app.emit(
                "pets:sync:progress",
                ProgressEvent {
                    slug: &slug,
                    current: added.len() as u32 + 1,
                    total: remote.total,
                    bytes_downloaded: total_bytes,
                    bytes_total: 0,
                },
            );
            added.push(slug);
            let _ = save_atomic(&app_data_dir, &manifest);
        }
        Ok(())
    })();

    if let Err(e) = result {
        failed.push(FailedInstall {
            slug: "bundle".to_string(),
            reason: format!("zip: {}", e),
        });
    }

    manifest.last_synced_at = Some(now_iso());
    manifest.last_remote_manifest_at = remote.generated_at.clone();
    let _ = save_atomic(&app_data_dir, &manifest);

    let report = InstallReport {
        added,
        skipped,
        failed,
        total_bytes,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    let _ = app.emit("pets:sync:finished", &report);
    report
}

pub async fn force_refresh(
    app: AppHandle,
    app_data_dir: std::path::PathBuf,
    remote: RemoteManifest,
    slug: String,
) -> InstallReport {
    let started = std::time::Instant::now();
    let pet = remote.pets.into_iter().find(|p| p.slug == slug);
    let Some(pet) = pet else {
        let _ = app.emit(
            "pets:sync:error",
            serde_json::json!({
                "slug": slug,
                "message": "slug not in remote manifest",
                "retryable": false,
            }),
        );
        return InstallReport {
            added: vec![],
            skipped: vec![],
            failed: vec![FailedInstall {
                slug,
                reason: "not in remote manifest".to_string(),
            }],
            total_bytes: 0,
            duration_ms: started.elapsed().as_millis() as u64,
        };
    };
    let app_data_arc = Arc::new(app_data_dir.clone());
    let result = install_one(app_data_arc, pet, true).await;
    let mut manifest = load(&app_data_dir);
    let mut added = vec![];
    let mut failed = vec![];
    let mut total_bytes = 0u64;
    match result {
        Ok((s, bytes, entry)) => {
            total_bytes = bytes;
            manifest.pets.insert(s.clone(), entry);
            manifest.last_synced_at = Some(now_iso());
            let _ = save_atomic(&app_data_dir, &manifest);
            added.push(s);
        }
        Err(e) => failed.push(e),
    }
    let report = InstallReport {
        added,
        skipped: vec![],
        failed,
        total_bytes,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    let _ = app.emit("pets:sync:finished", &report);
    report
}

pub fn uninstall(app_data_dir: &Path, slug: &str) -> std::io::Result<()> {
    let dir = pet_dir(app_data_dir, slug);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    let mut manifest = load(app_data_dir);
    manifest.pets.remove(slug);
    save_atomic(app_data_dir, &manifest)?;
    Ok(())
}

pub fn manifest_file(app_data_dir: &Path) -> std::path::PathBuf {
    manifest_path(app_data_dir)
}
