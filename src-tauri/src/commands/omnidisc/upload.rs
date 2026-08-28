//! Resumable uploads (tus 1.0.0) and attachment downloads.
//!
//! For an E2EE channel the file is encrypted to a temp file first and the blob
//! that leaves the machine is already ciphertext — the server stores size, id,
//! owner and expiry, nothing else (ADR 0014 §3). The key travels in the MLS
//! message, so it never reaches the frontend or the network in the clear.

use super::api::{http_client, Api, ERR_BAD_REQUEST, ERR_SERVER, ERR_UNREACHABLE};
use super::mls::FileManifest;
use super::{normalize_instance_url, store};
use base64::Engine;
use omnidisc_mls::FileSecret;
use omnidisc_proto::gateway::InstanceInfo;
use omnidisc_proto::message::Attachment;
use reqwest::Method;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub const EVENT_UPLOAD: &str = "omnidisc://upload";
pub const ERR_TOO_LARGE: &str = "ERR_UPLOAD_TOO_LARGE";
pub const ERR_CANCELLED: &str = "ERR_UPLOAD_CANCELLED";
pub const ERR_UPLOAD: &str = "ERR_UPLOAD";
pub const ERR_UNKNOWN_UPLOAD: &str = "ERR_UNKNOWN_UPLOAD";
pub const ERR_ATTACHMENT_ORIGIN: &str = "ERR_ATTACHMENT_ORIGIN";
pub const ERR_ATTACHMENT_TOO_LARGE: &str = "ERR_ATTACHMENT_TOO_LARGE";

/// Room for the per-chunk AEAD tags and the manifest header, so an honest file
/// is never cut off by its own budget.
const DOWNLOAD_SLACK: u64 = 1024 * 1024;
/// Ceiling for a plain (non-E2EE) attachment whose size nobody stated.
const DOWNLOAD_FALLBACK_CAP: u64 = 2 * 1024 * 1024 * 1024;

const PATCH_CHUNK: usize = 4 * 1024 * 1024;
const RESUME_ATTEMPTS: u32 = 4;
const TUS_VERSION: &str = "1.0.0";

#[derive(Clone, Serialize)]
pub struct UploadProgress {
    pub id: String,
    pub url: String,
    pub channel_id: String,
    pub name: String,
    pub sent: u64,
    pub total: u64,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default)]
    pub encrypted: bool,
}

/// A finished upload waiting to be attached to a message. The secret stays on
/// this side of the bridge; the frontend only ever passes the upload id back.
#[derive(Clone)]
pub struct ReadyUpload {
    pub url: String,
    pub channel_id: String,
    pub attachment_id: String,
    pub file_id: String,
    pub url_signed: String,
    pub filename: String,
    pub mime: Option<String>,
    pub size: u64,
    pub sha256: String,
    pub secret: Option<FileSecret>,
}

impl ReadyUpload {
    pub fn manifest(&self) -> Option<FileManifest> {
        let secret = self.secret.as_ref()?;
        Some(FileManifest {
            attachment_id: self.attachment_id.clone(),
            file_id: self.file_id.clone(),
            url: self.url_signed.clone(),
            name: self.filename.clone(),
            mime: self.mime.clone(),
            size: self.size,
            sha256: self.sha256.clone(),
            key: base64::engine::general_purpose::STANDARD.encode(secret.key),
            nonce: base64::engine::general_purpose::STANDARD.encode(secret.nonce),
        })
    }
}

#[derive(Default)]
pub struct UploadManager {
    running: Mutex<HashMap<String, CancellationToken>>,
    ready: Mutex<HashMap<String, ReadyUpload>>,
}

impl UploadManager {
    /// Look up finished uploads without consuming them, so a failed send can be
    /// retried with the same files instead of asking for them again.
    pub async fn peek(&self, ids: &[String]) -> Result<Vec<ReadyUpload>, String> {
        let map = self.ready.lock().await;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let ready = map
                .get(id)
                .ok_or_else(|| format!("{}:{}", ERR_UNKNOWN_UPLOAD, id))?;
            out.push(ready.clone());
        }
        Ok(out)
    }

    pub async fn release(&self, ids: &[String]) {
        let mut map = self.ready.lock().await;
        for id in ids {
            map.remove(id);
        }
    }

    pub async fn cancel(&self, id: &str) {
        if let Some(token) = self.running.lock().await.remove(id) {
            token.cancel();
        }
        self.ready.lock().await.remove(id);
    }
}

fn tmp_dir() -> Result<PathBuf, String> {
    let dir = store::base_dir()?.join("uploads");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("OmniDisc: could not create the upload workspace: {}", e))?;
    Ok(dir)
}

fn guess_mime(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        "zip" => "application/zip",
        _ => return None,
    };
    Some(mime.to_string())
}

fn meta_pair(key: &str, value: &str) -> String {
    format!("{} {}", key, base64::engine::general_purpose::STANDARD.encode(value))
}

pub async fn instance_limits(base: &str) -> Result<InstanceInfo, String> {
    let api = Api::public(base)?;
    api.send(Method::GET, "/api/instance", &[], None).await
}

struct Tus {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Tus {
    fn new(base: &str) -> Result<Self, String> {
        let token = store::load_token(base)?
            .ok_or_else(|| super::api::ERR_NO_SESSION.to_string())?;
        Ok(Self {
            http: http_client(Duration::from_secs(120))?,
            base: base.to_string(),
            token,
        })
    }

    async fn create(&self, total: u64, meta: &str) -> Result<String, String> {
        let response = self
            .http
            .post(format!("{}/api/uploads", self.base))
            .bearer_auth(&self.token)
            .header("tus-resumable", TUS_VERSION)
            .header("upload-length", total.to_string())
            .header("upload-metadata", meta)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("[omnidisc] upload create failed: {}", e);
                ERR_UNREACHABLE.to_string()
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
            return Err(ERR_TOO_LARGE.to_string());
        }
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let code = serde_json::from_str::<omnidisc_proto::rest::ApiError>(&text)
                .map(|e| e.code)
                .unwrap_or_default();
            tracing::warn!("[omnidisc] upload create -> {} {}", status.as_u16(), text);
            return Err(super::api::map_error(status, &code));
        }
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("[omnidisc] upload create answered without a Location header");
                ERR_SERVER.to_string()
            })?;
        Ok(location.rsplit('/').next().unwrap_or_default().to_string())
    }

    async fn offset(&self, id: &str) -> Result<u64, String> {
        let response = self
            .http
            .head(format!("{}/api/uploads/{}", self.base, id))
            .bearer_auth(&self.token)
            .header("tus-resumable", TUS_VERSION)
            .send()
            .await
            .map_err(|_| ERR_UNREACHABLE.to_string())?;
        if !response.status().is_success() {
            return Err(ERR_UPLOAD.to_string());
        }
        response
            .headers()
            .get("upload-offset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| ERR_SERVER.to_string())
    }

    /// Returns the new offset, plus the attachment once the last chunk lands.
    async fn patch(
        &self,
        id: &str,
        offset: u64,
        body: Vec<u8>,
    ) -> Result<(u64, Option<Attachment>), String> {
        let response = self
            .http
            .patch(format!("{}/api/uploads/{}", self.base, id))
            .bearer_auth(&self.token)
            .header("tus-resumable", TUS_VERSION)
            .header("upload-offset", offset.to_string())
            .header("content-type", "application/offset+octet-stream")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("[omnidisc] upload chunk failed: {}", e);
                ERR_UNREACHABLE.to_string()
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
            return Err(ERR_TOO_LARGE.to_string());
        }
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let code = serde_json::from_str::<omnidisc_proto::rest::ApiError>(&text)
                .map(|e| e.code)
                .unwrap_or_default();
            tracing::warn!("[omnidisc] upload chunk -> {} {}", status.as_u16(), text);
            return Err(super::api::map_error(status, &code));
        }
        let next = response
            .headers()
            .get("upload-offset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(offset + 1);
        if status == reqwest::StatusCode::OK {
            let text = response.text().await.unwrap_or_default();
            let attachment = serde_json::from_str::<Attachment>(&text)
                .map_err(|e| {
                    tracing::warn!("[omnidisc] upload finished with an unreadable body: {}", e);
                    ERR_SERVER.to_string()
                })?;
            return Ok((next, Some(attachment)));
        }
        Ok((next, None))
    }

    async fn terminate(&self, id: &str) {
        let _ = self
            .http
            .delete(format!("{}/api/uploads/{}", self.base, id))
            .bearer_auth(&self.token)
            .header("tus-resumable", TUS_VERSION)
            .send()
            .await;
    }
}

struct Job {
    id: String,
    base: String,
    channel_id: String,
    name: String,
    mime: Option<String>,
    encrypt: bool,
    source: PathBuf,
}

/// Progress sink. The Tauri command emits an event; the integration test just
/// counts, which is what lets the upload path be exercised without a webview.
pub type Progress<'a> = &'a (dyn Fn(UploadProgress) + Send + Sync);

fn emit(sink: Progress<'_>, progress: UploadProgress) {
    sink(progress);
}

fn progress_of(job: &Job, sent: u64, total: u64, state: &'static str) -> UploadProgress {
    UploadProgress {
        id: job.id.clone(),
        url: job.base.clone(),
        channel_id: job.channel_id.clone(),
        name: job.name.clone(),
        sent,
        total,
        state,
        error: None,
        attachment_id: None,
        mime: job.mime.clone(),
        encrypted: job.encrypt,
    }
}

async fn run_job(
    app: tauri::AppHandle,
    manager: Arc<UploadManager>,
    job: Job,
    cancel: CancellationToken,
) {
    let id = job.id.clone();
    let handle = app.clone();
    let sink = move |p: UploadProgress| {
        let _ = handle.emit(EVENT_UPLOAD, p);
    };
    match upload(&sink, &job, &cancel).await {
        Ok(ready) => {
            let mut done = progress_of(&job, ready.size, ready.size, "done");
            done.attachment_id = Some(ready.attachment_id.clone());
            manager.ready.lock().await.insert(id.clone(), ready);
            emit(&sink, done);
        }
        Err(err) => {
            let state = if cancel.is_cancelled() { "cancelled" } else { "failed" };
            let mut failed = progress_of(&job, 0, 0, state);
            failed.error = Some(err);
            emit(&sink, failed);
        }
    }
    manager.running.lock().await.remove(&id);
}

/// The upload pipeline without Tauri: used by the integration test, which has no
/// app handle to emit into.
pub async fn upload_file(
    base: &str,
    channel_id: &str,
    path: &Path,
    encrypt: bool,
    progress: Progress<'_>,
) -> Result<ReadyUpload, String> {
    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        base: base.to_string(),
        channel_id: channel_id.to_string(),
        mime: guess_mime(path),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string()),
        encrypt,
        source: path.to_path_buf(),
    };
    upload(progress, &job, &CancellationToken::new()).await
}

async fn upload(
    app: Progress<'_>,
    job: &Job,
    cancel: &CancellationToken,
) -> Result<ReadyUpload, String> {
    let meta = std::fs::metadata(&job.source)
        .map_err(|e| format!("{}:{}", ERR_BAD_REQUEST, unreadable(&job.source, e)))?;
    let plain_size = meta.len();
    let limits = instance_limits(&job.base).await?;
    let max = limits.limits.max_upload_bytes.max(limits.max_upload_bytes);

    // Refuse early: encrypting a 3 GB file only to be told it is too large is
    // the kind of wasted wait that makes people distrust the whole feature.
    if plain_size > max {
        return Err(ERR_TOO_LARGE.to_string());
    }
    emit(app, progress_of(job, 0, plain_size, "preparing"));

    let (payload_path, wire_size, sha256, secret) = if job.encrypt {
        let secret = omnidisc_mls::new_file_secret();
        let file_id = job.id.clone();
        let target = tmp_dir()?.join(format!("{}.enc", job.id));
        let source = job.source.clone();
        let target_for_task = target.clone();
        let secret_for_task = secret.clone();
        let (size, sha) = tokio::task::spawn_blocking(move || {
            omnidisc_mls::encrypt_file(&source, &target_for_task, &secret_for_task, &file_id)
        })
        .await
        .map_err(|e| format!("{}:{}", ERR_UPLOAD, e))?
        .map_err(|e| {
            tracing::warn!("[omnidisc] could not encrypt an attachment: {}", e);
            ERR_UPLOAD.to_string()
        })?;
        let wire = omnidisc_mls::encrypted_size(size);
        (target, wire, sha, Some(secret))
    } else {
        (job.source.clone(), plain_size, String::new(), None)
    };

    let cleanup = |path: &PathBuf| {
        if job.encrypt {
            let _ = std::fs::remove_file(path);
        }
    };

    if wire_size > max {
        cleanup(&payload_path);
        return Err(ERR_TOO_LARGE.to_string());
    }

    let mut metadata = vec![
        meta_pair("filename", &job.name),
        meta_pair("channel_id", &job.channel_id),
    ];
    if let Some(mime) = &job.mime {
        metadata.push(meta_pair("filetype", mime));
    }
    if job.encrypt {
        metadata.push(meta_pair("encrypted", "true"));
    }

    let tus = match Tus::new(&job.base) {
        Ok(t) => t,
        Err(e) => {
            cleanup(&payload_path);
            return Err(e);
        }
    };
    let upload_id = match tus.create(wire_size, &metadata.join(",")).await {
        Ok(id) => id,
        Err(e) => {
            cleanup(&payload_path);
            return Err(e);
        }
    };

    let result = push_chunks(app, job, &tus, &upload_id, &payload_path, wire_size, cancel).await;
    cleanup(&payload_path);
    drop_staged(&job.source);
    let attachment = match result {
        Ok(a) => a,
        Err(e) => {
            tus.terminate(&upload_id).await;
            return Err(e);
        }
    };

    Ok(ReadyUpload {
        url: job.base.clone(),
        channel_id: job.channel_id.clone(),
        attachment_id: attachment.id.to_string(),
        file_id: job.id.clone(),
        url_signed: attachment.url.clone(),
        filename: job.name.clone(),
        mime: job.mime.clone(),
        size: plain_size,
        sha256,
        secret,
    })
}

async fn push_chunks(
    app: Progress<'_>,
    job: &Job,
    tus: &Tus,
    upload_id: &str,
    path: &Path,
    total: u64,
    cancel: &CancellationToken,
) -> Result<Attachment, String> {
    use std::io::{Read, Seek, SeekFrom};

    let mut offset = 0u64;
    let mut attempts = 0u32;
    emit(app, progress_of(job, 0, total, "uploading"));
    loop {
        if cancel.is_cancelled() {
            return Err(ERR_CANCELLED.to_string());
        }
        if offset >= total {
            return Err(ERR_SERVER.to_string());
        }
        let want = ((total - offset) as usize).min(PATCH_CHUNK);
        let path = path.to_path_buf();
        let at = offset;
        let chunk = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<u8>> {
            let mut file = std::fs::File::open(&path)?;
            file.seek(SeekFrom::Start(at))?;
            let mut buf = vec![0u8; want];
            file.read_exact(&mut buf)?;
            Ok(buf)
        })
        .await
        .map_err(|e| format!("{}:{}", ERR_UPLOAD, e))?
        .map_err(|e| format!("{}:{}", ERR_UPLOAD, e))?;

        let sent = tokio::select! {
            _ = cancel.cancelled() => return Err(ERR_CANCELLED.to_string()),
            r = tus.patch(upload_id, offset, chunk) => r,
        };
        match sent {
            Ok((_, Some(attachment))) => {
                emit(app, progress_of(job, total, total, "uploading"));
                return Ok(attachment);
            }
            Ok((next, None)) => {
                attempts = 0;
                offset = next;
                emit(app, progress_of(job, offset, total, "uploading"));
            }
            Err(err) if err == ERR_UNREACHABLE && attempts + 1 < RESUME_ATTEMPTS => {
                // A dropped connection is not a failed upload: ask the server
                // where it actually stopped and carry on from there.
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(500 * u64::from(attempts))).await;
                match tus.offset(upload_id).await {
                    Ok(server_offset) => {
                        offset = server_offset;
                        emit(app, progress_of(job, offset, total, "resuming"));
                    }
                    Err(_) => continue,
                }
            }
            Err(err) => return Err(err),
        }
    }
}

#[tauri::command]
pub async fn omnidisc_instance_limits(url: String) -> Result<InstanceInfo, String> {
    let base = normalize_instance_url(&url)?;
    instance_limits(&base).await
}

/// Pasted or dropped bytes have no path on disk. Staging them under the upload
/// workspace gives the uploader a file to stream, and it is deleted with the
/// rest of the job.
#[tauri::command]
pub async fn omnidisc_stage_file(name: String, bytes: Vec<u8>) -> Result<String, String> {
    let dir = tmp_dir()?.join("staged");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("OmniDisc: could not create the upload workspace: {}", e))?;
    let safe = sanitize_filename::sanitize(&name);
    let safe = if safe.trim().is_empty() { "pasted".to_string() } else { safe };
    let path = dir.join(format!("{}-{}", uuid::Uuid::new_v4(), safe));
    std::fs::write(&path, bytes)
        .map_err(|e| format!("OmniDisc: could not stage the file: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

#[derive(Serialize)]
pub struct StartedUpload {
    pub id: String,
    pub size: u64,
    pub name: String,
}

#[tauri::command]
pub async fn omnidisc_upload_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    url: String,
    channel_id: String,
    path: String,
    encrypt: bool,
) -> Result<StartedUpload, String> {
    let base = normalize_instance_url(&url)?;
    let source = PathBuf::from(&path);
    if !source.is_file() {
        return Err(format!("{}:file_missing", ERR_BAD_REQUEST));
    }
    // Answer "too big" before the chip ever appears: a limit discovered halfway
    // through an upload reads as a bug, not as a rule.
    let size = std::fs::metadata(&source)
        .map_err(|e| format!("{}:{}", ERR_BAD_REQUEST, unreadable(&source, e)))?
        .len();
    let limits = instance_limits(&base).await?;
    if size > limits.limits.max_upload_bytes.max(limits.max_upload_bytes) {
        return Err(ERR_TOO_LARGE.to_string());
    }
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        base,
        channel_id,
        mime: guess_mime(&source),
        name,
        encrypt,
        source,
    };
    let id = job.id.clone();
    let cancel = CancellationToken::new();
    state
        .omnidisc_uploads
        .running
        .lock()
        .await
        .insert(id.clone(), cancel.clone());
    let manager = state.omnidisc_uploads.clone();
    let handle = app.clone();
    let name = job.name.clone();
    tauri::async_runtime::spawn(async move {
        run_job(handle, manager, job, cancel).await;
    });
    Ok(StartedUpload { id, size, name })
}

#[tauri::command]
pub async fn omnidisc_upload_cancel(
    state: tauri::State<'_, crate::AppState>,
    id: String,
) -> Result<(), String> {
    state.omnidisc_uploads.cancel(&id).await;
    Ok(())
}

#[derive(Serialize)]
pub struct DownloadedAttachment {
    pub path: String,
    pub name: String,
}

/// Fetch an attachment to the user's download folder. For an E2EE message the
/// key comes from the local decrypted-message cache, and the SHA-256 written by
/// the sender is checked before the file is handed over.
#[tauri::command]
pub async fn omnidisc_download_attachment(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    attachment_url: Option<String>,
    attachment_id: String,
    filename: String,
    ciphertext: Option<String>,
) -> Result<DownloadedAttachment, String> {
    let base = normalize_instance_url(&url)?;
    let manifest = match &ciphertext {
        Some(ct) => {
            let session = state.omnidisc_mls.session(&base).await?;
            let guard = session.lock().await;
            guard.manifest_for(ct, &attachment_id)
        }
        None => None,
    };
    if ciphertext.is_some() && manifest.is_none() {
        return Err(super::mls::ERR_NO_GROUP_YET.to_string());
    }
    let downloaded =
        fetch_attachment(&base, attachment_url, manifest, &attachment_id, &filename, None).await?;
    let _ = crate::commands::downloads::reveal_file(downloaded.path.clone()).await;
    Ok(downloaded)
}

/// Stream a response straight to disk, aborting the moment it exceeds `budget`.
/// Buffering the body first would let one reply from a hostile server exhaust
/// the app's memory, and the body is attacker-controlled by definition here.
async fn download_to(source: &url::Url, target: &Path, budget: u64) -> Result<(), String> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let http = http_client(Duration::from_secs(300))?;
    let response = http.get(source.clone()).send().await.map_err(|e| {
        tracing::warn!("[omnidisc] attachment download failed: {}", e);
        ERR_UNREACHABLE.to_string()
    })?;
    if !response.status().is_success() {
        return Err(super::api::map_error(response.status(), ""));
    }
    if response.content_length().is_some_and(|len| len > budget) {
        return Err(ERR_ATTACHMENT_TOO_LARGE.to_string());
    }
    let mut file = tokio::fs::File::create(target)
        .await
        .map_err(|e| format!("OmniDisc: could not write the download: {}", e))?;
    let mut stream = response.bytes_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            tracing::warn!("[omnidisc] attachment download interrupted: {}", e);
            ERR_UNREACHABLE.to_string()
        })?;
        written = written.saturating_add(chunk.len() as u64);
        if written > budget {
            drop(file);
            let _ = tokio::fs::remove_file(target).await;
            tracing::warn!("[omnidisc] an attachment kept sending past its stated size");
            return Err(ERR_ATTACHMENT_TOO_LARGE.to_string());
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("OmniDisc: could not write the download: {}", e))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("OmniDisc: could not write the download: {}", e))?;
    Ok(())
}

/// The origins an attachment may legitimately come from: the instance itself
/// and the media host it advertises. The URL in a manifest is written by
/// whoever sent the message, so without this an attachment is an arbitrary
/// fetch made by this app, with this app's network position.
fn attachment_origins(base: &str, limits: &InstanceInfo) -> Vec<url::Origin> {
    let mut origins = Vec::new();
    if let Ok(parsed) = url::Url::parse(base) {
        origins.push(parsed.origin());
    }
    if let Ok(media) = url::Url::parse(&limits.media_url) {
        origins.push(media.origin());
    }
    origins
}

fn origin_allowed(source: &url::Url, allowed: &[url::Origin]) -> bool {
    matches!(source.scheme(), "http" | "https") && allowed.contains(&source.origin())
}

/// A download is bounded by what the sender said the file is, plus the AEAD
/// overhead — never by whatever the server decides to keep sending.
fn download_budget(manifest: Option<&FileManifest>, instance_max: u64) -> u64 {
    match manifest {
        Some(m) => omnidisc_mls::encrypted_size(m.size).saturating_add(DOWNLOAD_SLACK),
        None => instance_max.max(DOWNLOAD_FALLBACK_CAP),
    }
}

/// Download, decrypt and verify. `into` overrides the destination folder, which
/// the integration test uses to stay out of the developer's Downloads.
pub async fn fetch_attachment(
    base: &str,
    attachment_url: Option<String>,
    manifest: Option<FileManifest>,
    attachment_id: &str,
    filename: &str,
    into: Option<PathBuf>,
) -> Result<DownloadedAttachment, String> {
    let name = sanitize_filename::sanitize(
        manifest.as_ref().map(|m| m.name.as_str()).unwrap_or(filename),
    );
    let name = if name.trim().is_empty() { "download".to_string() } else { name };
    let target_dir = match into {
        Some(dir) => dir,
        None => dirs::download_dir()
            .ok_or_else(|| "OmniDisc: could not find your downloads folder".to_string())?,
    };
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("OmniDisc: could not create the downloads folder: {}", e))?;
    let target = unique_path(&target_dir, &name);

    let source_url = manifest
        .as_ref()
        .map(|m| m.url.clone())
        .filter(|u| !u.is_empty())
        .or(attachment_url)
        .ok_or_else(|| format!("{}:no_attachment_url", ERR_BAD_REQUEST))?;
    let parsed = url::Url::parse(&source_url)
        .map_err(|_| format!("{}:bad_attachment_url", ERR_BAD_REQUEST))?;
    let limits = instance_limits(base).await?;
    if !origin_allowed(&parsed, &attachment_origins(base, &limits)) {
        tracing::warn!("[omnidisc] refused an attachment hosted outside {}", base);
        return Err(ERR_ATTACHMENT_ORIGIN.to_string());
    }
    let budget = download_budget(
        manifest.as_ref(),
        limits.limits.max_upload_bytes.max(limits.max_upload_bytes),
    );
    let tmp = tmp_dir()?.join(format!("{}.dl", attachment_id));
    let _ = std::fs::remove_file(&tmp);
    download_to(&parsed, &tmp, budget).await?;

    match manifest {
        Some(manifest) => {
            let secret = decode_secret(&manifest)?;
            let expected = manifest.sha256.clone();
            let size = manifest.size;
            let file_id = if manifest.file_id.is_empty() {
                manifest.attachment_id.clone()
            } else {
                manifest.file_id.clone()
            };
            let tmp_for_task = tmp.clone();
            let target_for_task = target.clone();
            let got = tokio::task::spawn_blocking(move || {
                omnidisc_mls::decrypt_file(
                    &tmp_for_task,
                    &target_for_task,
                    &secret,
                    &file_id,
                    size,
                )
            })
            .await
            .map_err(|e| format!("{}:{}", ERR_UPLOAD, e))?;
            let _ = std::fs::remove_file(&tmp);
            let got = got.map_err(|e| {
                tracing::warn!("[omnidisc] attachment failed to decrypt: {}", e);
                let _ = std::fs::remove_file(&target);
                "ERR_ATTACHMENT_CORRUPT".to_string()
            })?;
            if !expected.is_empty() && got != expected {
                let _ = std::fs::remove_file(&target);
                return Err("ERR_ATTACHMENT_CORRUPT".to_string());
            }
        }
        None => {
            std::fs::rename(&tmp, &target)
                .or_else(|_| std::fs::copy(&tmp, &target).map(|_| ()))
                .map_err(|e| format!("OmniDisc: could not write the download: {}", e))?;
            let _ = std::fs::remove_file(&tmp);
        }
    }

    let path = target.to_string_lossy().to_string();
    Ok(DownloadedAttachment { path, name })
}

fn decode_secret(manifest: &FileManifest) -> Result<FileSecret, String> {
    let engine = base64::engine::general_purpose::STANDARD;
    let key = engine
        .decode(&manifest.key)
        .ok()
        .filter(|b| b.len() == 32)
        .ok_or_else(|| "ERR_ATTACHMENT_CORRUPT".to_string())?;
    let nonce = engine
        .decode(&manifest.nonce)
        .ok()
        .filter(|b| b.len() == 16)
        .ok_or_else(|| "ERR_ATTACHMENT_CORRUPT".to_string())?;
    let mut secret = FileSecret { key: [0u8; 32], nonce: [0u8; 16] };
    secret.key.copy_from_slice(&key);
    secret.nonce.copy_from_slice(&nonce);
    Ok(secret)
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let ext = Path::new(name).extension().map(|e| e.to_string_lossy().to_string());
    for n in 2..1000 {
        let candidate = match &ext {
            Some(ext) => dir.join(format!("{stem} ({n}).{ext}")),
            None => dir.join(format!("{stem} ({n})")),
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(name)
}

/// A staged (pasted or dropped) source belongs to us, so it goes away with the
/// upload. A file the user picked from disk is theirs and is left alone.
fn drop_staged(source: &Path) {
    let Ok(staged) = tmp_dir().map(|d| d.join("staged")) else { return };
    if source.starts_with(&staged) {
        let _ = std::fs::remove_file(source);
    }
}

fn unreadable(path: &Path, e: std::io::Error) -> String {
    tracing::warn!("[omnidisc] cannot read {}: {}", path.display(), e);
    "file_missing".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(size: u64, url: &str) -> FileManifest {
        FileManifest {
            attachment_id: "42".into(),
            file_id: "f".into(),
            url: url.into(),
            name: "cat.png".into(),
            mime: None,
            size,
            sha256: String::new(),
            key: String::new(),
            nonce: String::new(),
        }
    }

    fn limits(media_url: &str) -> InstanceInfo {
        InstanceInfo {
            name: "test".into(),
            version: "0".into(),
            media_url: media_url.into(),
            sfu_url: String::new(),
            max_upload_bytes: 100 * 1024 * 1024,
            streaming: Default::default(),
            limits: Default::default(),
            registration_open: false,
        }
    }

    /// The attachment URL is written by whoever sent the message, so without an
    /// origin check a message is a way to make this app fetch anything at all —
    /// including hosts only this machine can reach.
    #[test]
    fn attachments_only_come_from_the_instance_or_its_media_host() {
        let base = "https://chat.example.org";
        let allowed = attachment_origins(base, &limits("https://media.example.org"));
        let ok = |u: &str| origin_allowed(&url::Url::parse(u).expect("url"), &allowed);
        assert!(ok("https://chat.example.org/api/uploads/42"));
        assert!(ok("https://media.example.org/attachments/42/cat.png?sig=x"));
        assert!(!ok("https://evil.example.net/attachments/42/cat.png"));
        assert!(!ok("http://chat.example.org/api/uploads/42"), "scheme is part of the origin");
        assert!(!ok("https://chat.example.org:8443/api/uploads/42"), "port is part of the origin");
        assert!(!ok("http://127.0.0.1:9200/_search"));
        assert!(!ok("file:///etc/passwd"));
    }

    /// `response.bytes()` used to buffer whatever the server sent; the budget is
    /// what turns "the sender said 4 MB" into a hard stop.
    #[test]
    fn downloads_are_budgeted_by_what_the_sender_declared() {
        let four_mb = 4 * 1024 * 1024;
        let budget = download_budget(Some(&manifest(four_mb, "")), u64::MAX);
        assert!(budget >= omnidisc_mls::encrypted_size(four_mb));
        assert!(budget < four_mb + 2 * DOWNLOAD_SLACK);
        // An empty file still gets its header and tag through.
        assert!(download_budget(Some(&manifest(0, "")), 0) >= omnidisc_mls::encrypted_size(0));
        // Nothing declared: the instance cap, never unbounded.
        assert_eq!(download_budget(None, 0), DOWNLOAD_FALLBACK_CAP);
        assert_eq!(download_budget(None, u64::MAX), u64::MAX);
    }

    #[test]
    fn metadata_is_base64_per_tus() {
        let pair = meta_pair("filename", "cat.png");
        assert_eq!(pair, "filename Y2F0LnBuZw==");
        let (key, value) = pair.split_once(' ').expect("pair");
        assert_eq!(key, "filename");
        let decoded = base64::engine::general_purpose::STANDARD.decode(value).expect("b64");
        assert_eq!(decoded, b"cat.png");
    }

    #[test]
    fn mime_guesses_cover_what_the_ui_renders_inline() {
        assert_eq!(guess_mime(Path::new("a/b/cat.PNG")).as_deref(), Some("image/png"));
        assert_eq!(guess_mime(Path::new("clip.mp4")).as_deref(), Some("video/mp4"));
        assert_eq!(guess_mime(Path::new("song.flac")).as_deref(), Some("audio/flac"));
        assert_eq!(guess_mime(Path::new("archive.7z")), None);
        assert_eq!(guess_mime(Path::new("noext")), None);
    }

    #[test]
    fn downloads_never_overwrite_an_existing_file() {
        let dir = std::env::temp_dir().join(format!("omnidisc-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        assert_eq!(unique_path(&dir, "cat.png"), dir.join("cat.png"));
        std::fs::write(dir.join("cat.png"), b"x").expect("write");
        assert_eq!(unique_path(&dir, "cat.png"), dir.join("cat (2).png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secrets_are_only_accepted_at_the_right_size() {
        let engine = base64::engine::general_purpose::STANDARD;
        let good = FileManifest {
            attachment_id: "1".into(),
            file_id: "1".into(),
            url: String::new(),
            name: "a".into(),
            mime: None,
            size: 1,
            sha256: String::new(),
            key: engine.encode([1u8; 32]),
            nonce: engine.encode([2u8; 16]),
        };
        assert!(decode_secret(&good).is_ok());
        let short = FileManifest { key: engine.encode([1u8; 16]), ..good.clone() };
        assert!(decode_secret(&short).is_err());
        let bad = FileManifest { nonce: "!!!".into(), ..good };
        assert!(decode_secret(&bad).is_err());
    }
}
