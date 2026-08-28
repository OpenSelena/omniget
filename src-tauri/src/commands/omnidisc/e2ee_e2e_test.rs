//! Live end-to-end check of milestone M4: device identity, MLS groups,
//! encrypted messages and encrypted attachments, against a real
//! omnidisc-server.
//!
//! ```text
//! OMNIDISC_TEST_URL=http://72.62.136.3:8080 \
//!   cargo test --lib omnidisc -- --ignored --nocapture
//! ```
//!
//! Each side gets its own `OMNIGET_OMNIDISC_SESSION_DIR`, because the session
//! store, the device key and the MLS state are all keyed by instance URL — two
//! accounts sharing one directory would share one device identity, which is
//! exactly the bug this test is meant to catch.

use super::api::Api;
use super::e2e_lock::SessionDirGuard;
use super::mls::{Decrypted, E2eePayload, MlsManager};
use super::{auth, device, mls, store, upload};
use base64::Engine;
use reqwest::Method;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

struct Side {
    name: String,
    dir: PathBuf,
    base: String,
    token: String,
    user_id: String,
    manager: MlsManager,
}

impl Side {
    /// Point every store lookup at this side's directory. The env var is read on
    /// each call, so switching sides is enough as long as nothing runs in
    /// parallel — and this test is deliberately sequential.
    fn activate(&self) {
        std::env::set_var(store::SESSION_DIR_ENV, &self.dir);
    }

    fn api(&self) -> Api {
        Api::with_token(self.base.clone(), self.token.clone()).expect("api")
    }
}

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(
        "{}{}{}",
        prefix,
        std::process::id() % 10_000,
        nanos % 1_000_000
    )
}

fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("omnidisc-e2ee-{}-{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("workspace");
    dir
}

async fn register_side(base: &str, name_prefix: &str) -> Side {
    let dir = workspace(name_prefix);
    std::env::set_var(store::SESSION_DIR_ENV, &dir);
    let username = unique(name_prefix);
    let user = auth::register(base, &username, "correct horse battery", None, None)
        .await
        .expect("register");
    let token = store::load_token(base).expect("store").expect("token");
    let user_id = user["id"].as_str().expect("user id").to_string();
    Side {
        name: username,
        dir,
        base: base.to_string(),
        token,
        user_id,
        manager: MlsManager::default(),
    }
}

/// Register the device and publish key packages, the way a gateway connect does.
async fn bring_online(side: &Side) {
    side.activate();
    let identity = device::ensure_registered(&side.base).await.expect("device");
    assert!(!identity.device_id.is_empty());
    assert_eq!(identity.fingerprint().split('-').count(), 8);
    let api = side.api();
    let handle = side.manager.session(&side.base).await.expect("session");
    let mut session = handle.lock().await;
    mls::top_up_key_packages(&api, &mut session)
        .await
        .expect("key packages");
}

async fn drain(side: &Side) -> Vec<Decrypted> {
    side.activate();
    let api = side.api();
    let handle = side.manager.session(&side.base).await.expect("session");
    let mut session = handle.lock().await;
    let mut out = Vec::new();
    mls::drain_inbox(&api, &mut session, Some(&mut out))
        .await
        .expect("drain inbox");
    out
}

fn sha256_of(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("read");
    Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[tokio::test]
#[ignore]
async fn two_devices_exchange_an_encrypted_message_and_an_encrypted_file() {
    let Ok(url) = std::env::var("OMNIDISC_TEST_URL") else {
        eprintln!("OMNIDISC_TEST_URL not set; skipping");
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();
    let base = super::normalize_instance_url(&url).expect("valid url");
    let _session_dir = SessionDirGuard::acquire();

    let alice = register_side(&base, "alicee").await;
    let bob = register_side(&base, "bobbbb").await;
    bring_online(&alice).await;
    bring_online(&bob).await;

    // Each side must have its own device identity, or the whole model collapses.
    alice.activate();
    let alice_device = device::identity(&base).expect("alice device");
    bob.activate();
    let bob_device = device::identity(&base).expect("bob device");
    assert_ne!(alice_device.device_id, bob_device.device_id);
    assert_ne!(alice_device.fingerprint(), bob_device.fingerprint());

    // The device roster is gated behind a shared context on the server, so it can
    // only be read once the DM exists — which is also the only moment the client
    // needs it.
    alice.activate();
    let dm: Value = alice
        .api()
        .send(
            Method::POST,
            "/api/users/@me/channels",
            &[],
            Some(json!({ "recipient_ids": [bob.user_id.clone()] })),
        )
        .await
        .expect("create dm");
    let dm_id = dm["id"].as_str().expect("dm id").to_string();

    // The public key the server hands out is the one this device really has.
    // Everything the client refuses in C2 hangs off this being true.
    let listed: Vec<Value> = alice
        .api()
        .send(
            Method::GET,
            &format!("/api/users/{}/devices", bob.user_id),
            &[],
            None,
        )
        .await
        .expect("bob devices");
    let seen = listed
        .iter()
        .find(|d| d["device_id"] == bob_device.device_id.as_str())
        .expect("bob's device is published");
    let published = base64::engine::general_purpose::STANDARD
        .decode(seen["ed25519_pub"].as_str().expect("ed25519_pub"))
        .expect("base64");
    assert_eq!(published, bob_device.public_key().to_vec());

    // ---------------------------------------------------------------- message
    let plaintext = "hello from the other side · 🔐";
    alice.activate();
    let sent = {
        let api = alice.api();
        let handle = alice.manager.session(&base).await.expect("session");
        let mut session = handle.lock().await;
        mls::send_encrypted(
            &api,
            &mut session,
            &dm_id,
            std::slice::from_ref(&bob.user_id),
            E2eePayload {
                v: 1,
                content: plaintext.to_string(),
                reply_to: None,
                nonce: Some("n-1".into()),
                files: vec![],
            },
        )
        .await
        .expect("send encrypted")
    };

    // What the server stored, and therefore what it dispatches, is ciphertext.
    assert_eq!(sent["content"], "");
    let ciphertext = sent["e2ee"]["ciphertext"]
        .as_str()
        .expect("the message carries an e2ee payload")
        .to_string();
    assert!(!ciphertext.contains("hello from the other side"));
    let raw = serde_json::to_string(&sent).expect("json");
    assert!(
        !raw.contains("hello from the other side"),
        "plaintext leaked into the message: {raw}"
    );

    let fetched: Vec<Value> = bob
        .api()
        .send(
            Method::GET,
            &format!("/api/channels/{dm_id}/messages"),
            &[("limit", "10".into())],
            None,
        )
        .await
        .expect("history");
    let stored = fetched
        .iter()
        .find(|m| m["id"] == sent["id"])
        .expect("the message is in the history");
    assert_eq!(stored["content"], "");
    assert_eq!(stored["e2ee"]["ciphertext"], ciphertext.as_str());

    let received = drain(&bob).await;
    let decrypted = received
        .iter()
        .find(|d| d.ciphertext == ciphertext)
        .expect("bob decrypted the message");
    assert_eq!(decrypted.payload.content, plaintext);
    assert_eq!(decrypted.channel_id, dm_id);
    assert_eq!(decrypted.sender_user_id, alice.user_id);
    assert_eq!(decrypted.sender_device_id, alice_device.device_id);

    // ------------------------------------------------------------------- file
    let scratch = workspace("payload");
    let source = scratch.join("five-mib.bin");
    let data: Vec<u8> = (0..(5 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
    std::fs::write(&source, &data).expect("write source");
    let source_sha = sha256_of(&source);

    alice.activate();
    let seen_progress = std::sync::Mutex::new(Vec::<u64>::new());
    let sink = |p: upload::UploadProgress| {
        if let Ok(mut v) = seen_progress.lock() {
            v.push(p.sent);
        }
    };
    let ready = upload::upload_file(&base, &dm_id, &source, true, &sink)
        .await
        .expect("upload");
    assert_eq!(ready.size, data.len() as u64);
    assert_eq!(ready.sha256, source_sha);
    assert!(
        seen_progress.lock().map(|v| v.len() > 1).unwrap_or(false),
        "the upload reported no progress"
    );
    let manifest = ready
        .manifest()
        .expect("an encrypted upload has a manifest");

    let sent_file = {
        let api = alice.api();
        let handle = alice.manager.session(&base).await.expect("session");
        let mut session = handle.lock().await;
        mls::send_encrypted(
            &api,
            &mut session,
            &dm_id,
            std::slice::from_ref(&bob.user_id),
            E2eePayload {
                v: 1,
                content: String::new(),
                reply_to: None,
                nonce: Some("n-2".into()),
                files: vec![manifest.clone()],
            },
        )
        .await
        .expect("send file message")
    };
    let file_ciphertext = sent_file["e2ee"]["ciphertext"]
        .as_str()
        .expect("ciphertext");

    let received = drain(&bob).await;
    let with_file = received
        .iter()
        .find(|d| d.ciphertext == file_ciphertext)
        .expect("bob decrypted the file message");
    let got = with_file
        .payload
        .files
        .first()
        .expect("the manifest survived");
    assert_eq!(got.name, "five-mib.bin");
    assert_eq!(got.size, data.len() as u64);
    assert_eq!(got.sha256, source_sha);
    assert_eq!(got.attachment_id, manifest.attachment_id);
    assert_eq!(got.file_id, manifest.file_id);
    assert_ne!(
        got.file_id, got.attachment_id,
        "the AAD id must not be the server's id"
    );

    // The blob the server holds is ciphertext, not the file.
    let http = super::api::http_client(std::time::Duration::from_secs(120)).expect("http");
    let downloaded_raw = http
        .get(&got.url)
        .send()
        .await
        .expect("fetch blob")
        .bytes()
        .await
        .expect("blob bytes");
    assert_ne!(
        downloaded_raw.len(),
        data.len(),
        "the stored blob is not padded ciphertext"
    );
    assert_ne!(
        &downloaded_raw[..64],
        &data[..64],
        "the server stored the file in the clear"
    );

    bob.activate();
    let into = workspace("bob-downloads");
    let saved = upload::fetch_attachment(
        &base,
        None,
        Some(got.clone()),
        &got.attachment_id,
        &got.name,
        Some(into.clone()),
    )
    .await
    .expect("download and decrypt");
    assert_eq!(sha256_of(std::path::Path::new(&saved.path)), source_sha);

    // A tampered manifest must not produce a file at all.
    let mut wrong = got.clone();
    wrong.sha256 = "0".repeat(64);
    assert!(
        upload::fetch_attachment(
            &base,
            None,
            Some(wrong),
            &got.attachment_id,
            &got.name,
            Some(into)
        )
        .await
        .is_err(),
        "a mismatched hash was accepted"
    );

    // ---------------------------------------------------------------- cleanup
    for side in [&alice, &bob] {
        side.activate();
        let _ = auth::logout(&base).await;
        let _ = std::fs::remove_dir_all(&side.dir);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    eprintln!("alice={} bob={} dm={}", alice.name, bob.name, dm_id);
}
