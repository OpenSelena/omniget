pub mod words;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use iroh::{Endpoint, NodeAddr, NodeId};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

const ALPN: &[u8] = b"omniget/p2p/1";
const CHUNK_SIZE: usize = 64 * 1024;

pub struct P2pDownloader;

impl P2pDownloader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformDownloader for P2pDownloader {
    fn name(&self) -> &str {
        "p2p"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Some(code) = url.strip_prefix("p2p:") {
            return words::is_valid_code(code);
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let code = url
            .strip_prefix("p2p:")
            .ok_or_else(|| anyhow!("Invalid P2P URL: {}", url))?;

        if !words::is_valid_code(code) {
            anyhow::bail!("Invalid share code: {}", code);
        }

        let title = format!("P2P Transfer ({})", &code[..code.len().min(30)]);

        Ok(MediaInfo {
            title,
            author: "P2P Transfer".to_string(),
            platform: "p2p".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![VideoQuality {
                label: "Original".to_string(),
                width: 0,
                height: 0,
                url: url.to_string(),
                format: "p2p".to_string(),
            }],
            media_type: MediaType::Video,
            file_size_bytes: None,
        })
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<f64>,
    ) -> anyhow::Result<DownloadResult> {
        let url = match info.available_qualities.first() {
            Some(q) => &q.url,
            None => anyhow::bail!("No URL found in MediaInfo"),
        };

        let code = url
            .strip_prefix("p2p:")
            .ok_or_else(|| anyhow!("Invalid P2P URL"))?;

        let _ = progress.send(-2.0).await;

        let (node_id_str, _words) = parse_share_code(code)?;

        let node_id: NodeId = node_id_str
            .parse()
            .map_err(|e| anyhow!("Invalid node ID in share code: {}", e))?;

        tracing::info!("[p2p] connecting to sender: {}", node_id.fmt_short());

        let ep = Endpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| anyhow!("Failed to create endpoint: {}", e))?;

        let addr = NodeAddr::new(node_id);

        let conn = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            ep.connect(addr, ALPN),
        )
        .await
        .map_err(|_| anyhow!("Connection timed out (60s). The sender may be offline."))?
        .map_err(|e| anyhow!("Connection failed: {}", e))?;

        tracing::info!("[p2p] connected to sender");

        let (mut send, mut recv) = conn.open_bi().await
            .map_err(|e| anyhow!("Failed to open stream: {}", e))?;

        send.write_all(b"REQUEST").await?;
        send.flush().await?;

        let mut header_buf = vec![0u8; 8192];
        let n = recv.read(&mut header_buf).await?
            .ok_or_else(|| anyhow!("Sender closed connection before sending file info"))?;
        let header_str = String::from_utf8_lossy(&header_buf[..n]);

        let parts: Vec<&str> = header_str.splitn(3, '\n').collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid file header from sender");
        }

        let file_name = parts[0].to_string();
        let file_size: u64 = parts[1].parse().unwrap_or(0);

        tracing::info!("[p2p] receiving: {} ({} bytes)", file_name, file_size);
        let _ = progress.send(0.0).await;

        send.write_all(b"ACCEPT").await?;
        send.flush().await?;

        let sanitized = sanitize_filename::sanitize(&file_name);
        let output_path = opts.output_dir.join(&sanitized);
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = File::create(&output_path).await?;
        let mut received: u64 = 0;
        let mut buf = vec![0u8; CHUNK_SIZE];

        loop {
            if opts.cancel_token.is_cancelled() {
                let _ = tokio::fs::remove_file(&output_path).await;
                conn.close(1u8.into(), b"cancelled");
                ep.close().await;
                anyhow::bail!("Download cancelled");
            }

            match recv.read(&mut buf).await? {
                Some(0) | None => break,
                Some(n) => {
                    file.write_all(&buf[..n]).await?;
                    received += n as u64;
                    if file_size > 0 {
                        let pct = (received as f64 / file_size as f64) * 100.0;
                        let _ = progress.send(pct).await;
                    }
                }
            }
        }

        file.flush().await?;
        drop(file);

        let _ = progress.send(100.0).await;
        conn.close(0u8.into(), b"done");
        ep.close().await;

        tracing::info!("[p2p] download complete: {}", output_path.display());

        Ok(DownloadResult {
            file_path: output_path,
            file_size_bytes: received,
            duration_seconds: 0.0,
            torrent_id: None,
        })
    }
}

pub struct P2pSendSession {
    pub code: String,
    pub file_path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub cancel_token: CancellationToken,
    pub progress: Arc<tokio::sync::Mutex<f64>>,
    pub status: Arc<tokio::sync::Mutex<String>>,
    pub sent_bytes: Arc<tokio::sync::Mutex<u64>>,
    pub paused: Arc<std::sync::atomic::AtomicBool>,
    endpoint: Endpoint,
}

pub async fn start_send(
    file_path: PathBuf,
    cancel_token: CancellationToken,
) -> anyhow::Result<P2pSendSession> {
    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| anyhow!("File not found: {}", e))?;

    if !metadata.is_file() {
        anyhow::bail!("Path is not a file: {}", file_path.display());
    }

    let file_size = metadata.len();
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let ep = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .map_err(|e| anyhow!("Failed to create iroh endpoint: {}", e))?;

    let node_id = ep.node_id();
    let word_code = words::generate_code();
    let code = format!("{}@{}", word_code, node_id);

    tracing::info!(
        "[p2p] iroh endpoint created: node_id={}, code={}",
        node_id.fmt_short(),
        code
    );

    Ok(P2pSendSession {
        code,
        file_path,
        file_name,
        file_size,
        cancel_token,
        progress: Arc::new(tokio::sync::Mutex::new(0.0)),
        status: Arc::new(tokio::sync::Mutex::new("waiting".to_string())),
        sent_bytes: Arc::new(tokio::sync::Mutex::new(0)),
        paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        endpoint: ep,
    })
}

pub async fn run_sender(session: &P2pSendSession) -> anyhow::Result<()> {
    let cancel = session.cancel_token.clone();

    *session.status.lock().await = "waiting_for_receiver".to_string();

    tracing::info!(
        "[p2p] waiting for receiver connection... code: {}",
        session.code
    );

    let incoming = tokio::select! {
        result = session.endpoint.accept() => {
            result.ok_or_else(|| anyhow!("Endpoint closed while waiting for receiver"))?
        }
        _ = cancel.cancelled() => {
            anyhow::bail!("Send cancelled while waiting for receiver");
        }
    };

    let mut incoming_conn = incoming.accept()
        .map_err(|e| anyhow!("Failed to accept connection: {}", e))?;

    let alpn = incoming_conn.alpn().await?;
    if alpn.as_slice() != ALPN {
        anyhow::bail!("Unknown ALPN: {:?}", alpn);
    }

    let conn = incoming_conn.await
        .map_err(|e| anyhow!("Connection handshake failed: {}", e))?;

    let remote = conn.remote_node_id()
        .map(|id| id.fmt_short())
        .unwrap_or_else(|_| "unknown".to_string());
    tracing::info!("[p2p] receiver connected: {}", remote);

    *session.status.lock().await = "connected".to_string();

    let (mut send, mut recv) = conn.accept_bi().await
        .map_err(|e| anyhow!("Failed to accept stream: {}", e))?;

    let mut req_buf = vec![0u8; 64];
    let n = recv.read(&mut req_buf).await?
        .ok_or_else(|| anyhow!("Receiver closed stream before request"))?;
    let request = String::from_utf8_lossy(&req_buf[..n]);

    if !request.starts_with("REQUEST") {
        anyhow::bail!("Unexpected request from receiver: {}", request);
    }

    let header = format!("{}\n{}\n", session.file_name, session.file_size);
    send.write_all(header.as_bytes()).await?;
    send.flush().await?;

    let mut accept_buf = vec![0u8; 64];
    let n = recv.read(&mut accept_buf).await?
        .ok_or_else(|| anyhow!("Receiver closed before accepting"))?;
    let accept = String::from_utf8_lossy(&accept_buf[..n]);

    if !accept.starts_with("ACCEPT") {
        anyhow::bail!("Receiver rejected transfer: {}", accept);
    }

    *session.status.lock().await = "transferring".to_string();
    tracing::info!(
        "[p2p] transferring: {} ({} bytes)",
        session.file_name,
        session.file_size
    );

    let mut file = File::open(&session.file_path).await?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut sent: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            conn.close(1u8.into(), b"cancelled");
            anyhow::bail!("Send cancelled during transfer");
        }

        while session.paused.load(std::sync::atomic::Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if cancel.is_cancelled() {
                conn.close(1u8.into(), b"cancelled");
                anyhow::bail!("Send cancelled while paused");
            }
        }

        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        send.write_all(&buf[..n]).await?;
        sent += n as u64;

        *session.sent_bytes.lock().await = sent;
        if session.file_size > 0 {
            *session.progress.lock().await = (sent as f64 / session.file_size as f64) * 100.0;
        }
    }

    send.finish()
        .map_err(|e| anyhow!("Failed to finish stream: {}", e))?;

    *session.progress.lock().await = 100.0;
    *session.status.lock().await = "complete".to_string();

    tracing::info!("[p2p] transfer complete: {} bytes sent", sent);

    conn.close(0u8.into(), b"done");
    session.endpoint.close().await;

    Ok(())
}

fn parse_share_code(code: &str) -> anyhow::Result<(String, String)> {
    let parts: Vec<&str> = code.splitn(2, '@').collect();
    if parts.len() != 2 {
        anyhow::bail!("Share code must contain words@node_id, got: {}", code);
    }
    Ok((parts[1].to_string(), parts[0].to_string()))
}
