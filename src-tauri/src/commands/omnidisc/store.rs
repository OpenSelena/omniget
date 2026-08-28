//! Session tokens, one per instance URL.
//!
//! macOS and Windows keep them in the OS keyring (service
//! `wtf.tonho.omniget.omnidisc`, account = instance URL). Linux, and any
//! machine where the keyring call fails, falls back to an encrypted file under
//! `{data_dir}/omnidisc/`. WHY the fallback instead of the Secret Service on
//! Linux: that backend needs libdbus headers at build time and a running
//! daemon at runtime, and neither the CI image nor a headless box has them —
//! a hard dependency would break the Linux build. The file is AES-256-CBC +
//! HMAC-SHA256 with a random key stored next to it with 0600 permissions, so
//! the tokens never sit in plain text inside a backup or a synced folder. It
//! is a floor, not a keyring: anyone running as the same OS user can read the
//! key. Tokens never leave this module towards the frontend.
//!
//! `OMNIGET_OMNIDISC_SESSION_DIR` forces the file store into that directory
//! and bypasses the keyring: the integration test must not write into the
//! developer's real keychain (and a rebuilt test binary would trigger a
//! keychain access prompt that blocks forever).

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const SERVICE: &str = "wtf.tonho.omniget.omnidisc";
const SESSIONS_FILE: &str = "sessions.bin";
const KEY_FILE: &str = "session.key";
const IV_LEN: usize = 16;
const MAC_LEN: usize = 32;

type Enc = cbc::Encryptor<aes::Aes256>;
type Dec = cbc::Decryptor<aes::Aes256>;
type HmacSha256 = Hmac<Sha256>;

pub const SESSION_DIR_ENV: &str = "OMNIGET_OMNIDISC_SESSION_DIR";

fn forced_dir() -> Option<PathBuf> {
    std::env::var(SESSION_DIR_ENV)
        .ok()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
}

/// Directory the file store writes into. Also where the MLS state blobs live,
/// so the integration test's `OMNIGET_OMNIDISC_SESSION_DIR` moves every OmniDisc
/// secret at once instead of half of them.
pub fn base_dir() -> Result<PathBuf, String> {
    if let Some(dir) = forced_dir() {
        return Ok(dir);
    }
    let base = crate::core::paths::app_data_dir()
        .ok_or_else(|| "OmniDisc: could not resolve the app data directory".to_string())?;
    Ok(base.join("omnidisc"))
}

/// Secrets other than the session token (device key, MLS state key) share the
/// same store under a suffixed account so one instance's material stays together
/// and is dropped together on logout.
pub fn secret_account(url: &str, kind: &str) -> String {
    format!("{}#{}", url, kind)
}

pub fn save_secret(account: &str, value: &str) -> Result<(), String> {
    if let Some(dir) = forced_dir() {
        return FileStore::new(dir).set(account, value);
    }
    match keyring_set(account, value) {
        Some(Ok(())) => return Ok(()),
        Some(Err(e)) => tracing::warn!("[omnidisc] keyring write failed, using file store: {}", e),
        None => {}
    }
    FileStore::default_dir()?.set(account, value)
}

pub fn load_secret(account: &str) -> Result<Option<String>, String> {
    if let Some(dir) = forced_dir() {
        return FileStore::new(dir).get(account);
    }
    match keyring_get(account) {
        Some(Ok(found)) => {
            if found.is_some() {
                return Ok(found);
            }
        }
        Some(Err(e)) => tracing::warn!("[omnidisc] keyring read failed, using file store: {}", e),
        None => {}
    }
    FileStore::default_dir()?.get(account)
}

pub fn delete_secret(account: &str) -> Result<(), String> {
    if let Some(dir) = forced_dir() {
        return FileStore::new(dir).remove(account);
    }
    if let Some(Err(e)) = keyring_delete(account) {
        tracing::warn!("[omnidisc] keyring delete failed: {}", e);
    }
    FileStore::default_dir()?.remove(account)
}

pub fn save_token(url: &str, token: &str) -> Result<(), String> {
    save_secret(url, token)
}

pub fn load_token(url: &str) -> Result<Option<String>, String> {
    load_secret(url)
}

pub fn delete_token(url: &str) -> Result<(), String> {
    delete_secret(url)
}

#[cfg(any(target_os = "macos", windows))]
fn keyring_entry(url: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, url).map_err(|e| e.to_string())
}

#[cfg(any(target_os = "macos", windows))]
fn keyring_set(url: &str, token: &str) -> Option<Result<(), String>> {
    Some(keyring_entry(url).and_then(|e| e.set_password(token).map_err(|e| e.to_string())))
}

#[cfg(any(target_os = "macos", windows))]
fn keyring_get(url: &str) -> Option<Result<Option<String>, String>> {
    Some(keyring_entry(url).and_then(|e| match e.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }))
}

#[cfg(any(target_os = "macos", windows))]
fn keyring_delete(url: &str) -> Option<Result<(), String>> {
    Some(keyring_entry(url).and_then(|e| match e.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn keyring_set(_url: &str, _token: &str) -> Option<Result<(), String>> {
    None
}

#[cfg(not(any(target_os = "macos", windows)))]
fn keyring_get(_url: &str) -> Option<Result<Option<String>, String>> {
    None
}

#[cfg(not(any(target_os = "macos", windows)))]
fn keyring_delete(_url: &str) -> Option<Result<(), String>> {
    None
}

pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn default_dir() -> Result<Self, String> {
        let base = crate::core::paths::app_data_dir()
            .ok_or_else(|| "OmniDisc: could not resolve the app data directory".to_string())?;
        Ok(Self::new(base.join("omnidisc")))
    }

    pub fn get(&self, url: &str) -> Result<Option<String>, String> {
        Ok(self.load()?.remove(url))
    }

    pub fn set(&self, url: &str, token: &str) -> Result<(), String> {
        let mut map = self.load()?;
        map.insert(url.to_string(), token.to_string());
        self.save(&map)
    }

    pub fn remove(&self, url: &str) -> Result<(), String> {
        let mut map = self.load()?;
        if map.remove(url).is_some() {
            self.save(&map)?;
        }
        Ok(())
    }

    fn load(&self) -> Result<HashMap<String, String>, String> {
        let path = self.dir.join(SESSIONS_FILE);
        let blob = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => return Err(format!("OmniDisc: could not read session store: {}", e)),
        };
        let key = self.key(false)?;
        let Some(key) = key else {
            return Ok(HashMap::new());
        };
        let plain = decrypt(&key, &blob)?;
        serde_json::from_slice(&plain)
            .map_err(|e| format!("OmniDisc: session store is corrupt: {}", e))
    }

    fn save(&self, map: &HashMap<String, String>) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("OmniDisc: could not create session store dir: {}", e))?;
        let key = self
            .key(true)?
            .ok_or_else(|| "OmniDisc: could not create session key".to_string())?;
        let plain = serde_json::to_vec(map)
            .map_err(|e| format!("OmniDisc: could not encode session store: {}", e))?;
        let blob = encrypt(&key, &plain);
        write_private(&self.dir.join(SESSIONS_FILE), &blob)
    }

    fn key(&self, create: bool) -> Result<Option<[u8; 32]>, String> {
        let path = self.dir.join(KEY_FILE);
        match std::fs::read(&path) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(Some(key))
            }
            Ok(_) => Err("OmniDisc: session key file has the wrong size".to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !create {
                    return Ok(None);
                }
                let mut key = [0u8; 32];
                rand::Rng::fill_bytes(&mut rand::rng(), &mut key);
                write_private(&path, &key)?;
                Ok(Some(key))
            }
            Err(e) => Err(format!("OmniDisc: could not read session key: {}", e)),
        }
    }
}

/// Write a file only its owner can read. On unix the mode is part of the
/// `open` call: creating the file first and chmod'ing after leaves a window in
/// which the session key, the device key or the MLS state is world-readable.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let io = |e: std::io::Error| format!("OmniDisc: could not write {}: {}", path.display(), e);
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(dir) = path.parent() {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).mode(0o600);
        let mut file = match opts.clone().create_new(true).open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Re-tighten before reopening: an older build may have left it
                // readable, and truncating does not change the mode.
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                    .map_err(io)?;
                opts.truncate(true).open(path).map_err(io)?
            }
            Err(e) => return Err(io(e)),
        };
        file.write_all(bytes).map_err(io)?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes).map_err(io)
    }
}

fn derive(key: &[u8; 32], label: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(label);
    hasher.finalize().into()
}

fn encrypt(key: &[u8; 32], plain: &[u8]) -> Vec<u8> {
    let enc_key = derive(key, b"enc");
    let mac_key = derive(key, b"mac");
    let mut iv = [0u8; IV_LEN];
    rand::Rng::fill_bytes(&mut rand::rng(), &mut iv);
    let mut buf = vec![0u8; plain.len() + 16];
    buf[..plain.len()].copy_from_slice(plain);
    let ct_len = Enc::new(&enc_key.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
        .map(|ct| ct.len())
        .unwrap_or(0);
    buf.truncate(ct_len);
    let mut out = Vec::with_capacity(IV_LEN + buf.len() + MAC_LEN);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&buf);
    let mut mac = HmacSha256::new_from_slice(&mac_key).unwrap_or_else(|_| {
        HmacSha256::new_from_slice(b"omnidisc").expect("hmac accepts any key length")
    });
    mac.update(&out);
    out.extend_from_slice(&mac.finalize().into_bytes());
    out
}

fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < IV_LEN + MAC_LEN {
        return Err("OmniDisc: session store is truncated".to_string());
    }
    let (body, tag) = blob.split_at(blob.len() - MAC_LEN);
    let mac_key = derive(key, b"mac");
    let mut mac = HmacSha256::new_from_slice(&mac_key).unwrap_or_else(|_| {
        HmacSha256::new_from_slice(b"omnidisc").expect("hmac accepts any key length")
    });
    mac.update(body);
    mac.verify_slice(tag)
        .map_err(|_| "OmniDisc: session store failed integrity check".to_string())?;
    let (iv, ct) = body.split_at(IV_LEN);
    let enc_key = derive(key, b"enc");
    let mut iv_arr = [0u8; IV_LEN];
    iv_arr.copy_from_slice(iv);
    let mut buf = ct.to_vec();
    Dec::new(&enc_key.into(), &iv_arr.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map(|pt| pt.to_vec())
        .map_err(|_| "OmniDisc: session store could not be decrypted".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "omnidisc-store-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn file_store_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = FileStore::new(&dir);
        assert_eq!(store.get("https://a.example").unwrap(), None);
        store.set("https://a.example", "od1.secret-a").unwrap();
        store.set("https://b.example", "od1.secret-b").unwrap();
        assert_eq!(store.get("https://a.example").unwrap().as_deref(), Some("od1.secret-a"));
        assert_eq!(store.get("https://b.example").unwrap().as_deref(), Some("od1.secret-b"));
        store.remove("https://a.example").unwrap();
        assert_eq!(store.get("https://a.example").unwrap(), None);
        assert_eq!(store.get("https://b.example").unwrap().as_deref(), Some("od1.secret-b"));
        let raw = std::fs::read(dir.join(SESSIONS_FILE)).unwrap();
        assert!(!raw.windows(b"od1.secret-b".len()).any(|w| w == b"od1.secret-b"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampering_is_rejected() {
        let dir = temp_dir("tamper");
        let store = FileStore::new(&dir);
        store.set("https://a.example", "od1.secret").unwrap();
        let path = dir.join(SESSIONS_FILE);
        let mut raw = std::fs::read(&path).unwrap();
        let idx = IV_LEN + 1;
        raw[idx] ^= 0xff;
        std::fs::write(&path, raw).unwrap();
        assert!(store.get("https://a.example").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The session key is the key to every stored token: it must never exist on
    /// disk, not even for an instant, in a mode another local account can read.
    #[cfg(unix)]
    #[test]
    fn private_files_are_never_readable_by_anyone_else() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("private");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = dir.join("key.bin");
        write_private(&path, b"first").unwrap();
        let mode = |p: &std::path::Path| {
            std::fs::metadata(p).unwrap().permissions().mode() & 0o777
        };
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&dir), 0o700, "the folder around it has to be private too");

        // An overwrite keeps the mode, and a file an older build left open is
        // tightened before it is written again.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, b"second").unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn encrypt_decrypt_roundtrip_and_mac() {
        let key = [7u8; 32];
        let blob = encrypt(&key, b"hello");
        assert_eq!(decrypt(&key, &blob).unwrap(), b"hello");
        let other = [8u8; 32];
        assert!(decrypt(&other, &blob).is_err());
    }
}
