//! MLS wiring: key-package stock, group lifecycle, envelope inbox and the
//! encrypted message payload.
//!
//! Everything that touches key material stays here. What crosses to the frontend
//! is decrypted text and file metadata — never a key, never the group state.
//!
//! Two rules from spike S-05 shape the code and are easy to break by accident:
//! state is written **before** a commit or a message goes out (losing the epoch
//! secrets of a commit you already sent bricks the group for you), and a commit
//! is merged **only after** the server accepted it (409 means someone else won
//! the epoch, so we clear, catch up and retry).

use super::api::{Api, ERR_BAD_REQUEST, ERR_NOT_FOUND};
use super::device::{self, DeviceIdentity};
use super::store;
use base64::Engine;
use omnidisc_mls::{ClaimedDevice, CommitOutput, DeviceRef, Incoming, MlsClient, CIPHERSUITE_ID};
use omnidisc_proto::channel::{Channel, ChannelType};
use omnidisc_proto::gateway::{MlsEnvelope, MlsEnvelopeKind};
use omnidisc_proto::rest::{
    ClaimedKeyPackages, Device, KeyPackageCount, MlsCommitResponse, MlsInboxResponse,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const EVENT_DECRYPTED: &str = "omnidisc://decrypted";
pub const ERR_NO_GROUP_YET: &str = "ERR_E2EE_NOT_READY";
pub const ERR_E2EE: &str = "ERR_E2EE";
/// The server said something about identity that the client could not confirm
/// on its own. It is never retried and never downgraded: MLS exists precisely
/// so a compromised server cannot get past this.
pub const ERR_E2EE_UNTRUSTED: &str = "ERR_E2EE_UNTRUSTED";

const KEY_PACKAGE_FLOOR: u64 = 20;
const COMMIT_RETRIES: u32 = 3;
const INBOX_PAGE: u32 = 100;
const HISTORY_CACHE_MAX: usize = 4_000;
const STATE_KEY_ACCOUNT: &str = "mls-state-key";
const PAYLOAD_VERSION: u32 = 1;
/// How stale a cached device roster may be before an unrecognised sender is
/// worth another request.
const ROSTER_REFRESH: std::time::Duration = std::time::Duration::from_secs(60);

/// What a decrypted OmniDisc message carries. The `files` entries hold the key
/// material of every attachment, which is exactly why the whole struct is
/// re-serialised without them before anything reaches the webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eePayload {
    #[serde(default = "one")]
    pub v: u32,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileManifest>,
}

fn one() -> u32 {
    PAYLOAD_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifest {
    pub attachment_id: String,
    /// Identifier bound into the chunk AAD at encryption time. It is NOT the
    /// attachment id: the file is encrypted before the server assigns one, and
    /// binding the AAD to something the server picks would let the server
    /// influence the ciphertext's domain separation.
    #[serde(default)]
    pub file_id: String,
    /// Signed media URL captured at upload time. The server never links an
    /// attachment to an MLS message (it cannot read the message), so this is the
    /// only handle the receiver has — and it carries the server's 24 h signature
    /// TTL with it.
    #[serde(default)]
    pub url: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    pub size: u64,
    pub sha256: String,
    pub key: String,
    pub nonce: String,
}

impl E2eePayload {
    /// The view the frontend gets: same message, no key material.
    pub fn public_view(&self) -> Value {
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({
                    "attachment_id": f.attachment_id,
                    "name": f.name,
                    "mime": f.mime,
                    "size": f.size,
                })
            })
            .collect();
        json!({
            "content": self.content,
            "reply_to": self.reply_to,
            "nonce": self.nonce,
            "files": files,
        })
    }
}

pub struct MlsManager {
    sessions: Mutex<HashMap<String, Arc<Mutex<Session>>>>,
}

impl Default for MlsManager {
    fn default() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }
}

impl MlsManager {
    pub async fn session(&self, base: &str) -> Result<Arc<Mutex<Session>>, String> {
        if let Some(existing) = self.sessions.lock().await.get(base) {
            return Ok(existing.clone());
        }
        let session = Session::open(base).await?;
        let handle = Arc::new(Mutex::new(session));
        self.sessions
            .lock()
            .await
            .insert(base.to_string(), handle.clone());
        Ok(handle)
    }

    pub async fn forget(&self, base: &str) {
        self.sessions.lock().await.remove(base);
    }
}

/// The voice room key for a channel on one instance. `establish` is the list of
/// people to build the group with when the channel is encrypted but nobody has
/// sent a message in it yet; without it we only reuse a group that exists.
/// `None` at any step means the call runs in the clear and the UI says so.
pub async fn voice_key_for(
    manager: &MlsManager,
    base: &str,
    channel_id: &str,
    establish: Option<&[String]>,
) -> Option<(u64, [u8; 32])> {
    let handle = manager.session(base).await.ok()?;
    let mut session = handle.lock().await;
    if !session.client.has_group(&group_id_for(channel_id)) {
        let recipients = establish?;
        let api = Api::authed(base).ok()?;
        if let Err(e) = ensure_group(&api, &mut session, channel_id, recipients).await {
            tracing::warn!("[omnidisc] no MLS group for the call in {}: {}", channel_id, e);
            return None;
        }
    }
    session.voice_key(channel_id)
}

pub struct Session {
    base: String,
    identity: DeviceIdentity,
    state_key: [u8; 32],
    client: MlsClient,
    /// Decrypted payloads keyed by the SHA-256 of their ciphertext. MLS ratchets
    /// forward, so a message can only be decrypted once — without this, scrolling
    /// back through history would show empty bubbles forever.
    history: HashMap<String, String>,
    history_order: Vec<String>,
    last_envelope_id: Option<String>,
    /// Devices each user has published, as fetched from the server, with the
    /// moment they were fetched. Sender attribution is checked against this and
    /// never against the identity string inside the MLS credential, which its
    /// owner writes freely.
    rosters: HashMap<String, (std::time::Instant, Vec<DeviceRef>)>,
}

fn state_path(base: &str) -> Result<PathBuf, String> {
    let digest = Sha256::digest(base.as_bytes());
    let name = digest[..8].iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(store::base_dir()?.join("mls").join(format!("{name}.state")))
}

fn history_path(base: &str) -> Result<PathBuf, String> {
    let path = state_path(base)?;
    Ok(path.with_extension("history"))
}

fn e2ee(e: impl std::fmt::Display) -> String {
    tracing::warn!("[omnidisc] e2ee: {}", e);
    ERR_E2EE.to_string()
}

/// A refusal is not a failure to be retried: it means the server said something
/// about identity we could not confirm, and the user has to be told that rather
/// than shown a generic "encryption hiccup".
fn e2ee_mls(e: omnidisc_mls::MlsError) -> String {
    match e {
        omnidisc_mls::MlsError::Untrusted(_) => {
            tracing::warn!("[omnidisc] refused an untrusted identity: {}", e);
            ERR_E2EE_UNTRUSTED.to_string()
        }
        other => e2ee(other),
    }
}

/// One MLS group per private channel, named after the channel so both sides
/// derive the same id without a lookup (S-05 gotcha 10).
pub fn group_id_for(channel_id: &str) -> String {
    format!("od-{}", channel_id)
}

pub fn channel_of_group(group_id: &str) -> Option<String> {
    group_id.strip_prefix("od-").map(str::to_string)
}

fn write_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("OmniDisc: could not create the MLS state dir: {}", e))?;
    }
    let tmp = path.with_extension("tmp");
    store::write_private(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("OmniDisc: could not replace the MLS state: {}", e))
}

impl Session {
    async fn open(base: &str) -> Result<Self, String> {
        let identity = device::identity(base)?;
        let account = store::secret_account(base, STATE_KEY_ACCOUNT);
        let state_key = match store::load_secret(&account)? {
            Some(encoded) => decode_key(&encoded)?,
            None => {
                let key = omnidisc_mls::new_state_key();
                store::save_secret(
                    &account,
                    &base64::engine::general_purpose::STANDARD.encode(key),
                )?;
                key
            }
        };
        let user_id = current_user_id(base).await?;
        let path = state_path(base)?;
        let client = match std::fs::read(&path) {
            Ok(blob) => match omnidisc_mls::decrypt_state(&state_key, &blob)
                .and_then(|plain| MlsClient::restore(&user_id, &identity.device_id, &identity.seed, &plain))
            {
                Ok(client) => client,
                Err(err) => {
                    // A state we cannot read is a state we cannot use. Starting
                    // over loses the history, which is the honest outcome — the
                    // alternative is a client that silently fails every send.
                    tracing::warn!("[omnidisc] MLS state unusable, starting fresh: {}", err);
                    MlsClient::new(&user_id, &identity.device_id, &identity.seed).map_err(e2ee)?
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                MlsClient::new(&user_id, &identity.device_id, &identity.seed).map_err(e2ee)?
            }
            Err(e) => return Err(format!("OmniDisc: could not read the MLS state: {}", e)),
        };
        let mut session = Self {
            base: base.to_string(),
            identity,
            state_key,
            client,
            history: HashMap::new(),
            history_order: Vec::new(),
            last_envelope_id: None,
            rosters: HashMap::new(),
        };
        session.load_history();
        Ok(session)
    }

    pub fn device_id(&self) -> &str {
        &self.identity.device_id
    }

    fn save(&self) -> Result<(), String> {
        let plain = self.client.export_state();
        let blob = omnidisc_mls::encrypt_state(&self.state_key, &plain).map_err(e2ee)?;
        write_atomic(&state_path(&self.base)?, &blob)
    }

    fn load_history(&mut self) {
        let Ok(path) = history_path(&self.base) else { return };
        let Ok(blob) = std::fs::read(&path) else { return };
        let Ok(plain) = omnidisc_mls::decrypt_state(&self.state_key, &blob) else {
            tracing::warn!("[omnidisc] the decrypted-message cache is unreadable; dropping it");
            return;
        };
        let Ok(entries) = serde_json::from_slice::<Vec<(String, String)>>(&plain) else { return };
        for (hash, payload) in entries {
            self.history_order.push(hash.clone());
            self.history.insert(hash, payload);
        }
    }

    fn save_history(&self) {
        let Ok(path) = history_path(&self.base) else { return };
        let entries: Vec<(&String, &String)> = self
            .history_order
            .iter()
            .filter_map(|h| self.history.get_key_value(h))
            .collect();
        let Ok(plain) = serde_json::to_vec(&entries) else { return };
        let Ok(blob) = omnidisc_mls::encrypt_state(&self.state_key, &plain) else { return };
        if let Err(e) = write_atomic(&path, &blob) {
            tracing::warn!("[omnidisc] could not persist the decrypted-message cache: {}", e);
        }
    }

    /// The cache holds the full payload, manifests included — a download that
    /// happens days later needs the file key, and this file is encrypted at rest
    /// with the same keyring-held key as the group state.
    pub fn remember(&mut self, ciphertext_b64: &str, payload: &E2eePayload) {
        let Ok(encoded) = serde_json::to_string(payload) else { return };
        let hash = hash_ciphertext(ciphertext_b64);
        if self.history.insert(hash.clone(), encoded).is_none() {
            self.history_order.push(hash);
        }
        while self.history_order.len() > HISTORY_CACHE_MAX {
            let oldest = self.history_order.remove(0);
            self.history.remove(&oldest);
        }
    }

    fn payload_of(&self, ciphertext_b64: &str) -> Option<E2eePayload> {
        let raw = self.history.get(&hash_ciphertext(ciphertext_b64))?;
        serde_json::from_str(raw).ok()
    }

    /// The voice room key for a channel, or `None` when this channel has no MLS
    /// group. `None` is not an error: guild channels without the E2EE flag are
    /// deliberately not encrypted, and the UI has to say which one is in effect.
    ///
    /// The exporter context is the channel id this client asked to call in, not
    /// the room name the server answered with: a hostile server that hands each
    /// participant a different room name would otherwise split the call into
    /// people who cannot hear each other.
    pub fn voice_key(&self, channel_id: &str) -> Option<(u64, [u8; 32])> {
        voice_key_of(&self.client, channel_id)
    }

    pub fn recall(&self, ciphertext_b64: &str) -> Option<Value> {
        self.payload_of(ciphertext_b64).map(|p| p.public_view())
    }

    pub fn manifest_for(&self, ciphertext_b64: &str, attachment_id: &str) -> Option<FileManifest> {
        self.payload_of(ciphertext_b64)?
            .files
            .into_iter()
            .find(|f| f.attachment_id == attachment_id)
    }
}

/// The voice key of a channel, derived from the channel id and nothing the
/// server said. Free-standing so the binding itself can be tested without a
/// session, a keyring and an instance behind it.
fn voice_key_of(client: &MlsClient, channel_id: &str) -> Option<(u64, [u8; 32])> {
    let group_id = group_id_for(channel_id);
    if !client.has_group(&group_id) {
        return None;
    }
    let epoch = client.epoch(&group_id)?;
    match client.voice_key(&group_id, channel_id.as_bytes()) {
        Ok(key) => Some((epoch, key)),
        Err(e) => {
            tracing::warn!("[omnidisc] could not derive the voice key for {}: {}", group_id, e);
            None
        }
    }
}

fn hash_ciphertext(ciphertext_b64: &str) -> String {
    let digest = Sha256::digest(ciphertext_b64.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_key(encoded: &str) -> Result<[u8; 32], String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|_| "OmniDisc: the stored MLS key is unreadable".to_string())?;
    if bytes.len() != 32 {
        return Err("OmniDisc: the stored MLS key has the wrong size".to_string());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

async fn current_user_id(base: &str) -> Result<String, String> {
    let api = Api::authed(base)?;
    let me: Value = api.send(Method::GET, "/api/users/@me", &[], None).await?;
    me.get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ERR_E2EE.to_string())
}

// ---------------------------------------------------------------------------
// Key packages
// ---------------------------------------------------------------------------

pub async fn top_up_key_packages(api: &Api, session: &mut Session) -> Result<(), String> {
    let count: KeyPackageCount = api
        .send(Method::GET, "/api/mls/key-packages/count", &[], None)
        .await?;
    if count.unclaimed >= KEY_PACKAGE_FLOOR && count.last_resort {
        return Ok(());
    }
    let missing = KEY_PACKAGE_FLOOR.saturating_sub(count.unclaimed) as usize;
    let blobs = session
        .client
        .key_packages(missing, !count.last_resort)
        .map_err(e2ee)?;
    if blobs.is_empty() {
        return Ok(());
    }
    session.save()?;
    let last_resort_index = if count.last_resort { usize::MAX } else { blobs.len() - 1 };
    let packages: Vec<Value> = blobs
        .iter()
        .enumerate()
        .map(|(i, blob)| {
            json!({
                "ciphersuite": CIPHERSUITE_ID,
                "blob": base64::engine::general_purpose::STANDARD.encode(blob),
                "last_resort": i == last_resort_index,
            })
        })
        .collect();
    let _: KeyPackageCount = api
        .send(
            Method::PUT,
            "/api/mls/key-packages",
            &[],
            Some(json!({ "key_packages": packages })),
        )
        .await?;
    tracing::info!("[omnidisc] published {} key packages", packages.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

/// The devices a user has published, as the server reports them. This is the
/// only handle the client has on "which key really belongs to that device", so
/// every key package and every sender is checked against it.
async fn device_roster(api: &Api, user_id: &str) -> Result<Vec<DeviceRef>, String> {
    let user_id = super::api::path_id(user_id)?;
    let devices: Vec<Device> = api
        .send(Method::GET, &format!("/api/users/{}/devices", user_id), &[], None)
        .await?;
    Ok(devices
        .into_iter()
        .filter(|d| d.revoked_at.is_none())
        .filter_map(|d| {
            let key = base64::engine::general_purpose::STANDARD
                .decode(&d.ed25519_pub)
                .ok()
                .filter(|b| b.len() == 32)?;
            Some(DeviceRef::new(d.user_id.to_string(), d.device_id, key))
        })
        .collect())
}

/// Claim one key package per device of each user, and bind each one to the
/// device it must belong to. A server that substitutes a package of its own
/// fails the binding inside `add_members`, and the whole commit is dropped:
/// adding an unverifiable device would hand it every later message.
async fn claim_devices(api: &Api, user_ids: &[String]) -> Result<Vec<ClaimedDevice>, String> {
    let mut claimed = Vec::new();
    for user_id in user_ids {
        let packages: ClaimedKeyPackages = match api
            .send(
                Method::GET,
                &format!("/api/mls/key-packages/{}", super::api::path_id(user_id)?),
                &[],
                None,
            )
            .await
        {
            Ok(p) => p,
            Err(e) if e.starts_with(ERR_NOT_FOUND) => continue,
            Err(e) => return Err(e),
        };
        if packages.key_packages.is_empty() {
            continue;
        }
        let roster = device_roster(api, user_id).await?;
        for kp in packages.key_packages {
            let Ok(blob) = base64::engine::general_purpose::STANDARD.decode(&kp.blob) else {
                tracing::warn!("[omnidisc] a claimed key package was not valid base64");
                return Err(ERR_E2EE_UNTRUSTED.to_string());
            };
            let Some(device) = roster.iter().find(|d| d.device_id == kp.device_id).cloned() else {
                tracing::warn!(
                    "[omnidisc] a key package was claimed for {}, a device {} never published",
                    kp.device_id,
                    user_id
                );
                return Err(ERR_E2EE_UNTRUSTED.to_string());
            };
            claimed.push(ClaimedDevice { device, key_package: blob });
        }
    }
    Ok(claimed)
}

/// Post a staged commit, handling the server's one-commit-per-epoch rule. On
/// 409 someone else's commit landed first: drop ours, catch up on the inbox and
/// stage it again against the new epoch.
async fn commit_with_retry(
    api: &Api,
    session: &mut Session,
    group_id: &str,
    mut output: CommitOutput,
    added: &[String],
    removed: &[String],
    key_packages: &[ClaimedDevice],
) -> Result<u64, String> {
    for attempt in 0..COMMIT_RETRIES {
        session.save()?;
        let mut body = json!({
            "epoch": output.epoch,
            "commit": base64::engine::general_purpose::STANDARD.encode(&output.commit),
            "added_devices": added,
            "removed_devices": removed,
        });
        if let Some(welcome) = &output.welcome {
            body["welcome"] =
                Value::String(base64::engine::general_purpose::STANDARD.encode(welcome));
        }
        let result: Result<MlsCommitResponse, String> = api
            .send(
                Method::POST,
                &format!("/api/mls/groups/{}/commit", group_id),
                &[],
                Some(body),
            )
            .await;
        match result {
            Ok(response) => {
                session.client.merge_pending(group_id).map_err(e2ee)?;
                session.save()?;
                return Ok(response.epoch);
            }
            Err(err) if err.contains("epoch_conflict") && attempt + 1 < COMMIT_RETRIES => {
                tracing::info!(
                    "[omnidisc] commit for {} lost the epoch race; catching up",
                    group_id
                );
                session.client.clear_pending(group_id).map_err(e2ee)?;
                session.save()?;
                drain_inbox(api, session, None).await?;
                if !removed.is_empty() {
                    output = session.client.remove_devices(group_id, removed).map_err(e2ee)?;
                } else if !key_packages.is_empty() {
                    output = session.client.add_members(group_id, key_packages).map_err(e2ee_mls)?;
                } else {
                    return Err(format!("{}:epoch_conflict", ERR_BAD_REQUEST));
                }
            }
            Err(err) => {
                let _ = session.client.clear_pending(group_id);
                session.save()?;
                return Err(err);
            }
        }
    }
    Err(format!("{}:epoch_conflict", ERR_BAD_REQUEST))
}

/// Make sure this channel has a group we are a member of, creating it and adding
/// every recipient's devices when it does not exist yet.
pub async fn ensure_group(
    api: &Api,
    session: &mut Session,
    channel_id: &str,
    recipient_user_ids: &[String],
) -> Result<String, String> {
    let group_id = group_id_for(channel_id);
    if session.client.has_group(&group_id) {
        return Ok(group_id);
    }
    // The server may already know a group we have not been welcomed into yet;
    // draining the inbox is what turns that Welcome into a usable group.
    drain_inbox(api, session, None).await?;
    if session.client.has_group(&group_id) {
        return Ok(group_id);
    }
    let existing: Result<Value, String> = api
        .send(
            Method::GET,
            &format!("/api/mls/groups/{}", group_id),
            &[],
            None,
        )
        .await;
    match existing {
        Ok(_) => {
            return Err(ERR_NO_GROUP_YET.to_string());
        }
        Err(e) if e.starts_with(ERR_NOT_FOUND) => {}
        Err(e) => return Err(e),
    }

    session.client.create_group(&group_id).map_err(e2ee)?;
    session.save()?;
    let created: Result<Value, String> = api
        .send(
            Method::POST,
            "/api/mls/groups",
            &[],
            Some(json!({ "channel_id": channel_id, "group_id": group_id })),
        )
        .await;
    if let Err(err) = created {
        session.client.drop_group(&group_id).map_err(e2ee)?;
        session.save()?;
        if err.contains("group_exists") {
            return Err(ERR_NO_GROUP_YET.to_string());
        }
        return Err(err);
    }

    let claimed = claim_devices(api, recipient_user_ids).await?;
    if claimed.is_empty() {
        // The group is registered and usable; the other side simply has no
        // device online yet, and will be added by the next commit.
        return Ok(group_id);
    }
    let devices: Vec<String> = claimed.iter().map(|c| c.device.device_id.clone()).collect();
    let output = session.client.add_members(&group_id, &claimed).map_err(e2ee_mls)?;
    commit_with_retry(api, session, &group_id, output, &devices, &[], &claimed).await?;
    Ok(group_id)
}

/// Add devices of the given users that are not in the group yet — how a friend's
/// new device (or our own second device) starts receiving messages.
pub async fn sync_members(
    api: &Api,
    session: &mut Session,
    group_id: &str,
    user_ids: &[String],
) -> Result<bool, String> {
    if !session.client.has_group(group_id) {
        return Ok(false);
    }
    let known = session.client.member_device_ids(group_id);
    let claimed: Vec<ClaimedDevice> = claim_devices(api, user_ids)
        .await?
        .into_iter()
        .filter(|c| !known.contains(&c.device.device_id))
        .collect();
    if claimed.is_empty() {
        return Ok(false);
    }
    let devices: Vec<String> = claimed.iter().map(|c| c.device.device_id.clone()).collect();
    let output = session.client.add_members(group_id, &claimed).map_err(e2ee_mls)?;
    commit_with_retry(api, session, group_id, output, &devices, &[], &claimed).await?;
    Ok(true)
}

pub async fn remove_device_everywhere(
    api: &Api,
    session: &mut Session,
    device_id: &str,
) -> Result<(), String> {
    if device_id == session.identity.device_id {
        return Ok(());
    }
    let groups = session.client.group_ids();
    let removed = vec![device_id.to_string()];
    for group_id in groups {
        if !session.client.member_device_ids(&group_id).iter().any(|d| d == device_id) {
            continue;
        }
        let output = match session.client.remove_devices(&group_id, &removed) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!("[omnidisc] could not stage a removal in {}: {}", group_id, e);
                continue;
            }
        };
        if let Err(e) =
            commit_with_retry(api, session, &group_id, output, &[], &removed, &[]).await
        {
            tracing::warn!("[omnidisc] could not remove {} from {}: {}", device_id, group_id, e);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Inbox
// ---------------------------------------------------------------------------

pub struct Decrypted {
    pub channel_id: String,
    pub group_id: String,
    pub ciphertext: String,
    pub sender_user_id: String,
    pub sender_device_id: String,
    /// Whether the sending leaf's signature key is one the claimed device
    /// actually published. False means "someone in this group signed as them",
    /// which the UI has to say out loud instead of putting a name on it.
    pub sender_verified: bool,
    pub payload: E2eePayload,
}

/// Drain every envelope addressed to this device, strictly in id order. Welcome
/// and Commit come before the Application messages of their epoch, so a single
/// ordered pass is enough — out-of-order processing is what produces
/// `WrongEpoch` and a group that never recovers.
pub async fn drain_inbox(
    api: &Api,
    session: &mut Session,
    mut sink: Option<&mut Vec<Decrypted>>,
) -> Result<(), String> {
    loop {
        let mut query: Vec<(&str, String)> = vec![("limit", INBOX_PAGE.to_string())];
        if let Some(after) = &session.last_envelope_id {
            query.push(("after", after.clone()));
        }
        let page: MlsInboxResponse = api.send(Method::GET, "/api/mls/inbox", &query, None).await?;
        if page.envelopes.is_empty() {
            return Ok(());
        }
        let mut envelopes = page.envelopes;
        envelopes.sort_by_key(|e| e.id.0);
        let mut acks: Vec<String> = Vec::with_capacity(envelopes.len());
        for envelope in &envelopes {
            acks.push(envelope.id.to_string());
            session.last_envelope_id = Some(envelope.id.to_string());
            match apply_envelope(api, session, envelope).await {
                Ok(Some(decrypted)) => {
                    if let Some(sink) = sink.as_deref_mut() {
                        sink.push(decrypted);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    // One unreadable envelope must not stall the whole mailbox:
                    // it is acked and reported, and the rest still lands.
                    tracing::warn!(
                        "[omnidisc] envelope {} ({}) could not be processed: {}",
                        envelope.id,
                        envelope.kind.as_str(),
                        err
                    );
                }
            }
        }
        session.save()?;
        session.save_history();
        if !acks.is_empty() {
            if let Err(e) = api
                .send_empty(
                    Method::POST,
                    "/api/mls/inbox/ack",
                    Some(json!({ "envelope_ids": acks })),
                )
                .await
            {
                tracing::warn!("[omnidisc] could not ack envelopes: {}", e);
            }
        }
        if !page.has_more {
            return Ok(());
        }
    }
}

/// Who is allowed to have welcomed us into `channel_id`'s group.
///
/// The server picks the group id and the Welcome, so on its own a Welcome
/// proves nothing: it has to name a channel we are really a recipient of, and
/// it has to come from a device one of that channel's recipients published.
/// Anything else is a server trying to seat itself in an encrypted room.
async fn welcome_senders(
    api: &Api,
    session: &mut Session,
    channel_id: &str,
) -> Result<Vec<DeviceRef>, String> {
    let channel: Channel = api
        .send(
            Method::GET,
            &format!("/api/channels/{}", super::api::path_id(channel_id)?),
            &[],
            None,
        )
        .await?;
    if !matches!(channel.kind, ChannelType::Dm | ChannelType::GroupDm) {
        return Err(format!("{} is not an encrypted channel", channel_id));
    }
    let me = session.client.user_id().to_string();
    let recipients: Vec<String> = channel.recipient_ids.iter().map(|r| r.to_string()).collect();
    if !recipients.iter().any(|r| *r == me) {
        return Err(format!("this account is not a recipient of {}", channel_id));
    }
    let mut allowed = Vec::new();
    for user_id in &recipients {
        let roster = device_roster(api, user_id).await?;
        session
            .rosters
            .insert(user_id.clone(), (std::time::Instant::now(), roster.clone()));
        allowed.extend(roster);
    }
    Ok(allowed)
}

/// True when the sending leaf's key is one the claimed device published. A miss
/// refetches the roster, because a peer's new device is a normal event and a
/// stale cache must not be reported as an impersonation — but no more often than
/// `ROSTER_REFRESH`, so a stream of forged senders cannot turn into one request
/// per message.
async fn sender_is_known(
    api: &Api,
    session: &mut Session,
    user_id: &str,
    device_id: &str,
    key: &[u8],
) -> bool {
    let matches = |roster: &Vec<DeviceRef>| {
        roster
            .iter()
            .any(|d| d.user_id == user_id && d.device_id == device_id && d.signature_key == key)
    };
    if let Some((fetched, roster)) = session.rosters.get(user_id) {
        if matches(roster) {
            return true;
        }
        if fetched.elapsed() < ROSTER_REFRESH {
            return false;
        }
    }
    match device_roster(api, user_id).await {
        Ok(roster) => {
            let ok = matches(&roster);
            session
                .rosters
                .insert(user_id.to_string(), (std::time::Instant::now(), roster));
            ok
        }
        Err(e) => {
            tracing::warn!("[omnidisc] could not fetch the devices of {}: {}", user_id, e);
            false
        }
    }
}

async fn apply_envelope(
    api: &Api,
    session: &mut Session,
    envelope: &MlsEnvelope,
) -> Result<Option<Decrypted>, String> {
    let blob = base64::engine::general_purpose::STANDARD
        .decode(&envelope.payload)
        .map_err(|_| "the payload was not valid base64".to_string())?;
    match envelope.kind {
        MlsEnvelopeKind::Welcome => {
            if session.client.has_group(&envelope.group_id) {
                return Err(format!(
                    "a Welcome for {}, a group this device is already in",
                    envelope.group_id
                ));
            }
            let channel_id = channel_of_group(&envelope.group_id)
                .ok_or_else(|| "unrecognised group id".to_string())?;
            let allowed = welcome_senders(api, session, &channel_id).await?;
            let group_id = session
                .client
                .join_welcome(&blob, &envelope.group_id, &allowed)
                .map_err(|e| e.to_string())?;
            tracing::info!("[omnidisc] joined MLS group {}", group_id);
            Ok(None)
        }
        MlsEnvelopeKind::Commit | MlsEnvelopeKind::Proposal | MlsEnvelopeKind::Application => {
            if !session.client.has_group(&envelope.group_id) {
                return Err("no local group for this envelope".to_string());
            }
            match session
                .client
                .process(&envelope.group_id, &blob)
                .map_err(|e| e.to_string())?
            {
                Incoming::Application { user_id, device_id, signature_key, plaintext } => {
                    let payload: E2eePayload = serde_json::from_slice(&plaintext)
                        .map_err(|e| format!("the plaintext was not an OmniDisc payload: {e}"))?;
                    session.remember(&envelope.payload, &payload);
                    let channel_id = channel_of_group(&envelope.group_id)
                        .ok_or_else(|| "unrecognised group id".to_string())?;
                    let sender_verified =
                        sender_is_known(api, session, &user_id, &device_id, &signature_key).await;
                    if !sender_verified {
                        tracing::warn!(
                            "[omnidisc] {} in {} signs as {}:{} with a key that device never published",
                            envelope.id,
                            envelope.group_id,
                            user_id,
                            device_id
                        );
                    }
                    Ok(Some(Decrypted {
                        channel_id,
                        group_id: envelope.group_id.clone(),
                        ciphertext: envelope.payload.clone(),
                        sender_user_id: user_id,
                        sender_device_id: device_id,
                        sender_verified,
                        payload,
                    }))
                }
                Incoming::Commit { removed_me, .. } => {
                    if removed_me {
                        tracing::info!(
                            "[omnidisc] this device was removed from {}",
                            envelope.group_id
                        );
                        session.client.drop_group(&envelope.group_id).map_err(|e| e.to_string())?;
                    }
                    Ok(None)
                }
                Incoming::Proposal => Ok(None),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sending
// ---------------------------------------------------------------------------

pub async fn send_encrypted(
    api: &Api,
    session: &mut Session,
    channel_id: &str,
    recipient_user_ids: &[String],
    payload: E2eePayload,
) -> Result<Value, String> {
    let group_id = ensure_group(api, session, channel_id, recipient_user_ids).await?;
    let _ = sync_members(api, session, &group_id, recipient_user_ids).await;
    let plaintext = serde_json::to_vec(&payload).map_err(e2ee)?;
    let (ciphertext, epoch) = session.client.encrypt(&group_id, &plaintext).map_err(e2ee)?;
    // The ratchet already moved; if the process died here the message would be
    // undecryptable for us, so the state goes to disk before the request.
    session.save()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&ciphertext);
    session.remember(&encoded, &payload);
    session.save_history();
    let mut body = json!({ "epoch": epoch, "blob": encoded });
    if let Some(nonce) = &payload.nonce {
        body["nonce"] = Value::String(nonce.clone());
    }
    api.send(
        Method::POST,
        &format!("/api/mls/groups/{}/messages", group_id),
        &[],
        Some(body),
    )
    .await
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

use tauri::Emitter;

fn emit_decrypted(app: &tauri::AppHandle, base: &str, items: Vec<Decrypted>) {
    for item in items {
        let _ = app.emit(
            EVENT_DECRYPTED,
            json!({
                "url": base,
                "channel_id": item.channel_id,
                "group_id": item.group_id,
                "ciphertext": item.ciphertext,
                "sender_user_id": item.sender_user_id,
                "sender_device_id": item.sender_device_id,
                "sender_verified": item.sender_verified,
                "payload": item.payload.public_view(),
            }),
        );
    }
}

/// Register the device, keep the key-package stock topped up and drain the
/// mailbox. Called on every gateway connect, and again on each `MLS_ENVELOPE`.
pub async fn sync(
    app: &tauri::AppHandle,
    manager: &MlsManager,
    base: &str,
    top_up: bool,
) -> Result<(), String> {
    let identity = device::ensure_registered(base).await?;
    let _ = identity;
    let api = Api::authed(base)?;
    let handle = manager.session(base).await?;
    let mut session = handle.lock().await;
    if top_up {
        if let Err(e) = top_up_key_packages(&api, &mut session).await {
            tracing::warn!("[omnidisc] could not top up key packages: {}", e);
        }
    }
    let mut decrypted = Vec::new();
    drain_inbox(&api, &mut session, Some(&mut decrypted)).await?;
    // A commit in the mailbox may have moved the epoch of the very channel we
    // are talking in. The key ring has to follow it, or the call goes silent
    // for whoever merged first.
    let room_key = match super::voice::channel_in_call_on(app, base).await {
        Some(channel_id) => session.voice_key(&channel_id),
        None => None,
    };
    drop(session);
    if let Some((epoch, key)) = room_key {
        super::voice::push_room_key(app, epoch, key).await;
    }
    emit_decrypted(app, base, decrypted);
    Ok(())
}

/// Gateway hook. READY tops up the key-package stock and drains the backlog;
/// an `MLS_ENVELOPE` only drains; `DEVICE_REVOKED` also has to issue the commit
/// that removes the device from every group, because the server cannot.
pub fn on_dispatch(app: &tauri::AppHandle, url: &str, t: &str, d: &Value) {
    let (top_up, revoked) = match t {
        "READY" | "RESUMED" => (true, None),
        "MLS_ENVELOPE" => (false, None),
        "DEVICE_REVOKED" => (
            false,
            d.get("device_id").and_then(Value::as_str).map(str::to_string),
        ),
        _ => return,
    };
    use tauri::Manager;
    let manager = app.state::<crate::AppState>().omnidisc_mls.clone();
    let handle = app.clone();
    let base = url.to_string();
    tauri::async_runtime::spawn(async move {
        if let Some(device_id) = revoked {
            match (Api::authed(&base), manager.session(&base).await) {
                (Ok(api), Ok(session)) => {
                    let mut guard = session.lock().await;
                    if let Err(e) = remove_device_everywhere(&api, &mut guard, &device_id).await {
                        tracing::warn!("[omnidisc] could not evict a revoked device: {}", e);
                    }
                }
                _ => tracing::warn!("[omnidisc] no session to evict a revoked device from"),
            }
        }
        if let Err(e) = sync(&handle, &manager, &base, top_up).await {
            tracing::warn!("[omnidisc] MLS sync failed: {}", e);
        }
    });
}

#[tauri::command]
pub async fn omnidisc_mls_sync(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    url: String,
    top_up: Option<bool>,
) -> Result<(), String> {
    let base = super::normalize_instance_url(&url)?;
    sync(&app, &state.omnidisc_mls, &base, top_up.unwrap_or(true)).await
}

#[derive(Serialize)]
pub struct GroupStatus {
    pub ready: bool,
    pub group_id: String,
    pub epoch: Option<u64>,
    pub members: Vec<GroupMemberView>,
}

#[derive(Serialize)]
pub struct GroupMemberView {
    pub user_id: String,
    pub device_id: String,
    pub fingerprint: String,
    pub is_me: bool,
}

/// What the padlock in the UI is allowed to claim. `ready == false` means the
/// group does not exist yet, and the UI must not promise encryption.
#[tauri::command]
pub async fn omnidisc_mls_status(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    channel_id: String,
) -> Result<GroupStatus, String> {
    let base = super::normalize_instance_url(&url)?;
    let group_id = group_id_for(&channel_id);
    let handle = state.omnidisc_mls.session(&base).await?;
    let session = handle.lock().await;
    let members = session
        .client
        .members(&group_id)
        .into_iter()
        .map(|m| GroupMemberView {
            user_id: m.user_id,
            device_id: m.device_id,
            fingerprint: m.fingerprint,
            is_me: m.is_me,
        })
        .collect();
    Ok(GroupStatus {
        ready: session.client.has_group(&group_id),
        epoch: session.client.epoch(&group_id),
        group_id,
        members,
    })
}

/// Scrollback: MLS ratchets forward, so old ciphertext can only come from the
/// local cache. Anything not cached stays honestly blank in the UI.
#[tauri::command]
pub async fn omnidisc_mls_recall(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    ciphertexts: Vec<String>,
) -> Result<HashMap<String, Value>, String> {
    let base = super::normalize_instance_url(&url)?;
    let handle = state.omnidisc_mls.session(&base).await?;
    let session = handle.lock().await;
    let mut out = HashMap::new();
    for ciphertext in ciphertexts {
        if let Some(payload) = session.recall(&ciphertext) {
            out.insert(ciphertext, payload);
        }
    }
    Ok(out)
}

/// A device of ours was revoked: drop it from every group we are in, since the
/// server cannot issue that commit itself.
#[tauri::command]
pub async fn omnidisc_mls_device_revoked(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    device_id: String,
) -> Result<(), String> {
    let base = super::normalize_instance_url(&url)?;
    let api = Api::authed(&base)?;
    let handle = state.omnidisc_mls.session(&base).await?;
    let mut session = handle.lock().await;
    remove_device_everywhere(&api, &mut session, &device_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_ids_round_trip_and_fit_the_server_rules() {
        let id = group_id_for("1234567890123456789");
        assert_eq!(id, "od-1234567890123456789");
        assert!(id.len() >= 8 && id.len() <= 128);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')));
        assert_eq!(channel_of_group(&id).as_deref(), Some("1234567890123456789"));
        assert_eq!(channel_of_group("nope"), None);
    }

    #[test]
    fn the_public_view_never_carries_key_material() {
        let payload = E2eePayload {
            v: 1,
            content: "here is the file".into(),
            reply_to: None,
            nonce: Some("n-1".into()),
            files: vec![FileManifest {
                attachment_id: "42".into(),
                file_id: "local-1".into(),
                url: "https://media.example/attachments/42/cat.png?sig=x".into(),
                name: "cat.png".into(),
                mime: Some("image/png".into()),
                size: 1234,
                sha256: "abc".into(),
                key: "SUPERSECRETKEY".into(),
                nonce: "SUPERSECRETNONCE".into(),
            }],
        };
        let view = serde_json::to_string(&payload.public_view()).expect("json");
        assert!(view.contains("cat.png"));
        assert!(view.contains("here is the file"));
        assert!(!view.contains("SUPERSECRETKEY"), "{view}");
        assert!(!view.contains("SUPERSECRETNONCE"), "{view}");
        assert!(!view.contains("sha256"), "{view}");
        assert!(!view.contains("sig=x"), "{view}");
    }

    #[test]
    fn payloads_round_trip_through_json() {
        let raw = r#"{"v":1,"content":"hi","files":[]}"#;
        let payload: E2eePayload = serde_json::from_str(raw).expect("parse");
        assert_eq!(payload.content, "hi");
        assert!(payload.files.is_empty());
        assert!(payload.reply_to.is_none());
        let minimal: E2eePayload = serde_json::from_str("{}").expect("parse");
        assert_eq!(minimal.v, PAYLOAD_VERSION);
    }

    #[test]
    fn ciphertext_hashes_are_stable_and_distinct() {
        let a = hash_ciphertext("AAAA");
        assert_eq!(a, hash_ciphertext("AAAA"));
        assert_ne!(a, hash_ciphertext("AAAB"));
        assert_eq!(a.len(), 64);
    }

    /// The server picks the LiveKit room name, so it must not reach the key
    /// derivation: two participants handed different room names would otherwise
    /// derive different keys and hear silence.
    #[test]
    fn the_voice_key_is_bound_to_the_channel_and_not_to_anything_the_server_says() {
        use omnidisc_mls::{ClaimedDevice, DeviceRef, MlsClient};
        let channel = "1234567890123456789";
        let group = group_id_for(channel);
        let mut alice = MlsClient::new("1001", "desktop-a", &[91u8; 32]).expect("alice");
        let mut bob = MlsClient::new("2002", "phone-bbb", &[92u8; 32]).expect("bob");
        alice.create_group(&group).expect("group");
        let bob_kp = ClaimedDevice {
            device: DeviceRef::new(bob.user_id(), bob.device_id(), bob.public_key().to_vec()),
            key_package: bob.key_packages(1, false).expect("kp").remove(0),
        };
        let out = alice.add_members(&group, &[bob_kp]).expect("add");
        alice.merge_pending(&group).expect("merge");
        let alice_ref =
            DeviceRef::new(alice.user_id(), alice.device_id(), alice.public_key().to_vec());
        bob.join_welcome(&out.welcome.clone().expect("welcome"), &group, &[alice_ref])
            .expect("join");

        let (epoch_a, key_a) = voice_key_of(&alice, channel).expect("alice key");
        let (epoch_b, key_b) = voice_key_of(&bob, channel).expect("bob key");
        assert_eq!(epoch_a, epoch_b);
        assert_eq!(key_a, key_b, "both sides must derive the same key from the channel alone");
        assert!(voice_key_of(&alice, "9999999999999999999").is_none());
        assert_ne!(
            key_a,
            alice.voice_key(&group, b"livekit-room-the-server-picked").expect("raw"),
            "the channel id is what the exporter is bound to"
        );
    }

    #[test]
    fn state_keys_are_validated_on_the_way_in() {
        let key = [3u8; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        assert_eq!(decode_key(&encoded).expect("decode"), key);
        assert!(decode_key("...").is_err());
        assert!(decode_key(&base64::engine::general_purpose::STANDARD.encode([1u8; 8])).is_err());
    }
}
