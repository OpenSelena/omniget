//! Per-instance device identity: one Ed25519 key pair and one stable device id.
//!
//! The private key never leaves this module — not to the frontend, not to the
//! logs, not to `settings.json`. It lives in the same store as the session token
//! (OS keyring on macOS/Windows, encrypted file elsewhere, see `store.rs`), so
//! "sign out" and "forget this instance" already drop it.

use super::api::Api;
use super::{normalize_instance_url, store};
use base64::Engine;
use omnidisc_proto::rest::Device;
use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

const DEVICE_KEY: &str = "device-key";
const DEVICES_FILE: &str = "device-ids.json";

#[derive(Clone)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub seed: [u8; 32],
}

impl DeviceIdentity {
    pub fn public_key(&self) -> [u8; 32] {
        omnidisc_mls::MlsClient::new("0", &self.device_id, &self.seed)
            .map(|c| c.public_key())
            .unwrap_or([0u8; 32])
    }

    pub fn fingerprint(&self) -> String {
        omnidisc_mls::fingerprint(&self.public_key())
    }
}

#[derive(Serialize)]
pub struct DeviceFingerprint {
    pub device_id: String,
    pub fingerprint: String,
    pub public_key: String,
}

fn ids_path() -> Result<PathBuf, String> {
    Ok(store::base_dir()?.join(DEVICES_FILE))
}

fn read_ids() -> HashMap<String, String> {
    ids_path()
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_ids(map: &HashMap<String, String>) -> Result<(), String> {
    let path = ids_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("OmniDisc: could not create the device store: {}", e))?;
    }
    let bytes = serde_json::to_vec(map)
        .map_err(|e| format!("OmniDisc: could not encode the device store: {}", e))?;
    std::fs::write(&path, bytes)
        .map_err(|e| format!("OmniDisc: could not write the device store: {}", e))
}

/// Device ids must match the server's `[A-Za-z0-9_-]{8,64}`, so the UUID keeps
/// its hyphens and gains a prefix that makes it recognisable in a log line.
fn new_device_id() -> String {
    format!("od-{}", uuid::Uuid::new_v4())
}

pub fn hostname() -> Option<String> {
    for key in ["HOSTNAME", "COMPUTERNAME", "HOST"] {
        if let Some(h) = std::env::var(key)
            .ok()
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
        {
            return Some(h);
        }
    }
    #[cfg(unix)]
    if let Ok(raw) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = raw.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

pub fn device_name() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    let name = match hostname() {
        Some(host) => format!("{} · OmniGet on {}", host, os),
        None => format!("OmniGet on {}", os),
    };
    // The server caps device names at 64 characters and rejects longer ones;
    // a long hostname must not turn into a failed device registration.
    name.chars().take(64).collect()
}

/// Load this instance's identity, generating it on first use.
pub fn identity(base: &str) -> Result<DeviceIdentity, String> {
    let mut ids = read_ids();
    let device_id = match ids.get(base) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            let id = new_device_id();
            ids.insert(base.to_string(), id.clone());
            write_ids(&ids)?;
            id
        }
    };
    let account = store::secret_account(base, DEVICE_KEY);
    let seed = match store::load_secret(&account)? {
        Some(encoded) => decode_seed(&encoded)?,
        None => {
            let mut seed = [0u8; 32];
            rand::Rng::fill_bytes(&mut rand::rng(), &mut seed);
            store::save_secret(
                &account,
                &base64::engine::general_purpose::STANDARD.encode(seed),
            )?;
            seed
        }
    };
    Ok(DeviceIdentity { device_id, seed })
}

fn decode_seed(encoded: &str) -> Result<[u8; 32], String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|_| "OmniDisc: the stored device key is unreadable".to_string())?;
    if bytes.len() != 32 {
        return Err("OmniDisc: the stored device key has the wrong size".to_string());
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}

pub fn forget(base: &str) -> Result<(), String> {
    let mut ids = read_ids();
    if ids.remove(base).is_some() {
        write_ids(&ids)?;
    }
    store::delete_secret(&store::secret_account(base, DEVICE_KEY))
}

/// Publish the public half so other devices can add us to their MLS groups.
/// Called on login and on every gateway connect: it is an upsert, and it is what
/// binds the current session to this device on the server side.
pub async fn register(api: &Api, identity: &DeviceIdentity) -> Result<Device, String> {
    let public = base64::engine::general_purpose::STANDARD.encode(identity.public_key());
    let body = json!({ "ed25519_pub": public, "name": device_name() });
    let device: Device = api
        .send(
            Method::PUT,
            &format!("/api/users/@me/devices/{}", identity.device_id),
            &[],
            Some(body),
        )
        .await?;
    tracing::info!(
        "[omnidisc] device {} registered on {}",
        identity.device_id,
        api.base
    );
    Ok(device)
}

pub async fn ensure_registered(base: &str) -> Result<DeviceIdentity, String> {
    let identity = identity(base)?;
    let api = Api::authed(base)?;
    register(&api, &identity).await?;
    Ok(identity)
}

#[tauri::command]
pub async fn omnidisc_device_fingerprint(url: String) -> Result<DeviceFingerprint, String> {
    let base = normalize_instance_url(&url)?;
    let identity = identity(&base)?;
    let public = identity.public_key();
    Ok(DeviceFingerprint {
        device_id: identity.device_id,
        fingerprint: omnidisc_mls::fingerprint(&public),
        public_key: base64::engine::general_purpose::STANDARD.encode(public),
    })
}

#[tauri::command]
pub async fn omnidisc_list_user_devices(url: String, user_id: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let path = if user_id.trim().is_empty() || user_id == "@me" {
        "/api/users/@me/devices".to_string()
    } else {
        format!("/api/users/{}/devices", user_id)
    };
    let devices: Vec<Device> = api.send(Method::GET, &path, &[], None).await?;
    let enriched: Vec<Value> = devices
        .into_iter()
        .map(|d| {
            let fingerprint = base64::engine::general_purpose::STANDARD
                .decode(&d.ed25519_pub)
                .ok()
                .filter(|b| b.len() == 32)
                .map(|b| omnidisc_mls::fingerprint(&b));
            let mut value = serde_json::to_value(&d).unwrap_or(Value::Null);
            if let (Value::Object(map), Some(fp)) = (&mut value, fingerprint) {
                map.insert("fingerprint".into(), Value::String(fp));
            }
            value
        })
        .collect();
    Ok(Value::Array(enriched))
}

#[tauri::command]
pub async fn omnidisc_revoke_device(url: String, device_id: String) -> Result<(), String> {
    let base = normalize_instance_url(&url)?;
    let api = Api::authed(&base)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/users/@me/devices/{}", device_id),
        None,
    )
    .await?;
    let mine = identity(&base)?;
    if mine.device_id == device_id {
        forget(&base)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_ids_fit_the_server_rules() {
        let id = new_device_id();
        assert!(id.len() >= 8 && id.len() <= 64, "{id}");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{id}"
        );
        assert_ne!(id, new_device_id());
    }

    #[test]
    fn device_names_say_where_they_run() {
        let name = device_name();
        assert!(name.contains("OmniGet"), "{name}");
        assert!(name.len() <= 64, "{name}");
    }

    #[test]
    fn seeds_round_trip_and_bad_ones_are_refused() {
        let seed = [9u8; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(seed);
        assert_eq!(decode_seed(&encoded).expect("decode"), seed);
        assert!(decode_seed("not base64!!").is_err());
        assert!(decode_seed(&base64::engine::general_purpose::STANDARD.encode([1u8; 16])).is_err());
    }
}
