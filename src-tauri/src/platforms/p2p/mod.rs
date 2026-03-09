pub mod hole_punch;
pub mod protocol;
pub mod stun;
pub mod words;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

use protocol::{
    hash_code, read_data_frame, read_message, write_data_frame, write_message, CHUNK_SIZE,
    DISCOVERY_PORT, Message,
};

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
            .ok_or_else(|| anyhow::anyhow!("Invalid P2P URL: {}", url))?;

        let parsed = words::parse_code(code)
            .ok_or_else(|| anyhow::anyhow!("Invalid share code: {}", code))?;

        let label = if parsed.remote_endpoint.is_some() {
            "remote"
        } else {
            "local"
        };
        let title = format!("P2P Transfer ({}, {})", parsed.words, label);

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
            .ok_or_else(|| anyhow::anyhow!("Invalid P2P URL"))?;

        let parsed = words::parse_code(code)
            .ok_or_else(|| anyhow::anyhow!("Invalid share code"))?;

        let _ = progress.send(-2.0).await;

        if let Some(endpoint) = parsed.remote_endpoint {
            tracing::info!("[p2p] trying remote connection to {}", endpoint);
            match try_remote_download(
                &parsed.words,
                endpoint,
                &opts.output_dir,
                &progress,
                &opts.cancel_token,
            )
            .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("[p2p] remote failed: {}, falling back to LAN", e);
                }
            }
        }

        let expected_hash = hash_code(&parsed.words);

        let sender_info = tokio::select! {
            result = discover_sender(&expected_hash, std::time::Duration::from_secs(60)) => {
                result?
            }
            _ = opts.cancel_token.cancelled() => {
                anyhow::bail!("Transfer cancelled while searching for sender");
            }
        };

        tracing::info!(
            "[p2p] found sender at {}:{}",
            sender_info.addr,
            sender_info.tcp_port,
        );

        let _ = progress.send(-1.0).await;

        let sender_addr = SocketAddr::new(sender_info.addr, sender_info.tcp_port);
        let mut stream = tokio::select! {
            result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                TcpStream::connect(sender_addr),
            ) => {
                match result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => anyhow::bail!("Failed to connect to sender: {}", e),
                    Err(_) => anyhow::bail!("Connection to sender timed out"),
                }
            }
            _ = opts.cancel_token.cancelled() => {
                anyhow::bail!("Transfer cancelled during connection");
            }
        };

        tcp_receive(&mut stream, &expected_hash, &opts.output_dir, &progress, &opts.cancel_token)
            .await
    }
}

async fn try_remote_download(
    code: &str,
    endpoint: SocketAddr,
    output_dir: &std::path::Path,
    progress_tx: &mpsc::Sender<f64>,
    cancel: &CancellationToken,
) -> anyhow::Result<DownloadResult> {
    let socket = hole_punch::punch_to_sender(
        code,
        endpoint,
        std::time::Duration::from_secs(15),
        cancel,
    )
    .await?;

    let _ = progress_tx.send(-1.0).await;

    hole_punch::receive_file_udp(&socket, endpoint, output_dir, progress_tx, cancel).await
}

async fn tcp_receive(
    stream: &mut TcpStream,
    expected_hash: &str,
    output_dir: &std::path::Path,
    progress: &mpsc::Sender<f64>,
    cancel: &CancellationToken,
) -> anyhow::Result<DownloadResult> {
    write_message(
        stream,
        &Message::Hello {
            code_hash: expected_hash.to_string(),
        },
    )
    .await?;

    let (filename, file_size) = match read_message(stream).await? {
        Message::FileInfo {
            name,
            size,
            file_count: _,
        } => (name, size),
        Message::Error { message } => anyhow::bail!("Sender error: {}", message),
        other => anyhow::bail!("Unexpected message: {:?}", other),
    };

    tracing::info!(
        "[p2p] receiving file: {} ({} bytes)",
        filename,
        file_size
    );

    write_message(stream, &Message::Accept).await?;

    let _ = progress.send(0.0).await;

    let safe_name = sanitize_filename::sanitize(&filename);
    let mut output_path = output_dir.join(&safe_name);

    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    if output_path.exists() {
        let stem = output_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = output_path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let parent = output_path.parent().unwrap().to_path_buf();
        let mut n = 1u32;
        loop {
            let candidate = parent.join(format!("{} ({}){}", stem, n, ext));
            if !candidate.exists() {
                output_path = candidate;
                break;
            }
            n += 1;
        }
    }

    let file = File::create(&output_path).await?;
    let mut writer = BufWriter::new(file);
    let mut received: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            drop(writer);
            let _ = tokio::fs::remove_file(&output_path).await;
            anyhow::bail!("Transfer cancelled");
        }

        let data = read_data_frame(stream).await?;
        if data.is_empty() {
            break;
        }

        writer.write_all(&data).await?;
        received += data.len() as u64;

        if file_size > 0 {
            let pct = (received as f64 / file_size as f64) * 100.0;
            let _ = progress.send(pct).await;
        }
    }

    writer.flush().await?;
    drop(writer);

    match read_message(stream).await {
        Ok(Message::Done) => {}
        _ => tracing::warn!("[p2p] did not receive Done message"),
    }

    let _ = progress.send(100.0).await;

    tracing::info!(
        "[p2p] transfer complete: {} ({} bytes)",
        safe_name,
        received
    );

    Ok(DownloadResult {
        file_path: output_path,
        file_size_bytes: received,
        duration_seconds: 0.0,
        torrent_id: None,
    })
}

struct SenderInfo {
    addr: std::net::IpAddr,
    tcp_port: u16,
}

async fn discover_sender(
    expected_hash: &str,
    timeout: std::time::Duration,
) -> anyhow::Result<SenderInfo> {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)).await {
        Ok(s) => s,
        Err(_) => {
            UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to bind UDP socket: {}", e))?
        }
    };

    socket.set_broadcast(true)?;

    let mut buf = [0u8; 2048];

    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline - tokio::time::Instant::now();
        if remaining.is_zero() {
            anyhow::bail!("Sender not found on local network (timed out after {:?}). Make sure both devices are on the same network.", timeout);
        }

        let result = tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await;

        match result {
            Ok(Ok((len, src_addr))) => {
                if let Some(packet) = protocol::parse_discovery_packet(&buf[..len]) {
                    if packet.code_hash == expected_hash {
                        return Ok(SenderInfo {
                            addr: src_addr.ip(),
                            tcp_port: packet.tcp_port,
                        });
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("[p2p] UDP recv error: {}", e);
            }
            Err(_) => {
                anyhow::bail!("Sender not found on local network (timed out). Make sure both devices are on the same network.");
            }
        }
    }
}

enum Connection {
    Tcp(TcpStream),
    Udp(SocketAddr),
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
    pub udp_socket: Option<Arc<UdpSocket>>,
    pub public_endpoint: Option<SocketAddr>,
}

pub async fn start_send(
    file_path: PathBuf,
    cancel_token: CancellationToken,
) -> anyhow::Result<P2pSendSession> {
    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| anyhow::anyhow!("File not found: {}", e))?;

    if !metadata.is_file() {
        anyhow::bail!("Path is not a file: {}", file_path.display());
    }

    let file_size = metadata.len();
    let file_name = protocol::safe_filename(&file_path);
    let base_code = words::generate_code();

    let udp_socket = UdpSocket::bind("0.0.0.0:0").await.ok();
    let (public_endpoint, udp_arc) = match udp_socket {
        Some(sock) => {
            let arc = Arc::new(sock);
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                stun::discover_public_endpoint(&arc),
            )
            .await
            {
                Ok(Ok(ep)) => (Some(ep), Some(arc)),
                Ok(Err(e)) => {
                    tracing::warn!("[p2p] STUN failed: {}", e);
                    (None, Some(arc))
                }
                Err(_) => {
                    tracing::warn!("[p2p] STUN timed out");
                    (None, Some(arc))
                }
            }
        }
        None => (None, None),
    };

    let code = match &public_endpoint {
        Some(ep) => format!("{}@{}", base_code, ep),
        None => base_code,
    };

    tracing::info!(
        "[p2p] session created: code={}, endpoint={:?}",
        code,
        public_endpoint
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
        udp_socket: udp_arc,
        public_endpoint,
    })
}

pub async fn run_sender(session: &P2pSendSession) -> anyhow::Result<()> {
    let cancel = session.cancel_token.clone();

    let tcp_listener = TcpListener::bind("0.0.0.0:0").await?;
    let tcp_port = tcp_listener.local_addr()?.port();

    tracing::info!(
        "[p2p] sender ready — code: {}, TCP port: {}, file: {} ({} bytes)",
        session.code,
        tcp_port,
        session.file_name,
        session.file_size
    );

    *session.status.lock().await = "waiting_for_receiver".to_string();

    let code_words = session.code.split('@').next().unwrap_or(&session.code);
    let expected_hash = hash_code(code_words);

    let discovery_packet = protocol::encode_discovery_packet(
        code_words,
        tcp_port,
        &session.file_name,
        session.file_size,
    );

    let broadcast_cancel = cancel.clone();
    let broadcast_handle = tokio::spawn(async move {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("[p2p] failed to bind broadcast socket: {}", e);
                return;
            }
        };
        if let Err(e) = socket.set_broadcast(true) {
            tracing::error!("[p2p] failed to set broadcast: {}", e);
            return;
        }

        let broadcast_addr: SocketAddr =
            format!("255.255.255.255:{}", DISCOVERY_PORT).parse().unwrap();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                    if let Err(e) = socket.send_to(&discovery_packet, broadcast_addr).await {
                        tracing::debug!("[p2p] broadcast send error (non-fatal): {}", e);
                    }
                }
                _ = broadcast_cancel.cancelled() => {
                    break;
                }
            }
        }
    });

    let connection = if let Some(ref udp) = session.udp_socket {
        let udp_clone = udp.clone();
        let hash_clone = expected_hash.clone();
        let cancel_clone = cancel.clone();

        tokio::select! {
            result = tcp_listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        tracing::info!("[p2p] TCP receiver connected from {}", addr);
                        Connection::Tcp(stream)
                    }
                    Err(e) => {
                        broadcast_handle.abort();
                        anyhow::bail!("TCP accept failed: {}", e);
                    }
                }
            }
            result = hole_punch::wait_for_punch(&udp_clone, &hash_clone, &cancel_clone) => {
                match result {
                    Ok(peer) => {
                        tracing::info!("[p2p] UDP receiver punched from {}", peer);
                        Connection::Udp(peer)
                    }
                    Err(e) => {
                        broadcast_handle.abort();
                        return Err(e);
                    }
                }
            }
            result = tokio::time::timeout(std::time::Duration::from_secs(600), std::future::pending::<()>()) => {
                let _ = result;
                broadcast_handle.abort();
                anyhow::bail!("No receiver connected within 10 minutes");
            }
            _ = cancel.cancelled() => {
                broadcast_handle.abort();
                anyhow::bail!("Send cancelled while waiting for receiver");
            }
        }
    } else {
        tokio::select! {
            result = tokio::time::timeout(
                std::time::Duration::from_secs(600),
                tcp_listener.accept(),
            ) => {
                match result {
                    Ok(Ok((stream, addr))) => {
                        tracing::info!("[p2p] receiver connected from {}", addr);
                        Connection::Tcp(stream)
                    }
                    Ok(Err(e)) => {
                        broadcast_handle.abort();
                        anyhow::bail!("TCP accept failed: {}", e);
                    }
                    Err(_) => {
                        broadcast_handle.abort();
                        anyhow::bail!("No receiver connected within 10 minutes");
                    }
                }
            }
            _ = cancel.cancelled() => {
                broadcast_handle.abort();
                anyhow::bail!("Send cancelled while waiting for receiver");
            }
        }
    };

    broadcast_handle.abort();

    match connection {
        Connection::Tcp(mut stream) => {
            tcp_send(&mut stream, session).await
        }
        Connection::Udp(peer) => {
            let socket = session.udp_socket.as_ref().unwrap();
            *session.status.lock().await = "transferring".to_string();
            hole_punch::send_file_udp(
                socket,
                peer,
                &session.file_path,
                &session.file_name,
                session.file_size,
                &session.progress,
                &session.sent_bytes,
                &session.cancel_token,
                &session.paused,
            )
            .await?;
            *session.status.lock().await = "complete".to_string();
            Ok(())
        }
    }
}

async fn tcp_send(stream: &mut TcpStream, session: &P2pSendSession) -> anyhow::Result<()> {
    let cancel = &session.cancel_token;

    let expected_hash = {
        let code_words = session.code.split('@').next().unwrap_or(&session.code);
        hash_code(code_words)
    };

    match read_message(stream).await? {
        Message::Hello { code_hash } => {
            if code_hash != expected_hash {
                write_message(
                    stream,
                    &Message::Error {
                        message: "Invalid code".to_string(),
                    },
                )
                .await?;
                anyhow::bail!("Receiver sent wrong code hash");
            }
        }
        other => {
            anyhow::bail!("Expected Hello, got {:?}", other);
        }
    }

    write_message(
        stream,
        &Message::FileInfo {
            name: session.file_name.clone(),
            size: session.file_size,
            file_count: 1,
        },
    )
    .await?;

    match read_message(stream).await? {
        Message::Accept => {}
        Message::Reject => {
            anyhow::bail!("Receiver rejected the transfer");
        }
        other => {
            anyhow::bail!("Expected Accept/Reject, got {:?}", other);
        }
    }

    *session.status.lock().await = "transferring".to_string();

    let mut file = File::open(&session.file_path).await?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut sent: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            anyhow::bail!("Send cancelled during transfer");
        }

        while session.paused.load(std::sync::atomic::Ordering::Relaxed) {
            if cancel.is_cancelled() {
                anyhow::bail!("Send cancelled while paused");
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        tokio::select! {
            result = write_data_frame(stream, &buf[..n]) => {
                result?;
            }
            _ = cancel.cancelled() => {
                anyhow::bail!("Send cancelled during chunk write");
            }
        }
        sent += n as u64;

        if session.file_size > 0 {
            let pct = (sent as f64 / session.file_size as f64) * 100.0;
            *session.progress.lock().await = pct;
            *session.sent_bytes.lock().await = sent;
        }
    }

    write_data_frame(stream, &[]).await?;
    write_message(stream, &Message::Done).await?;

    *session.progress.lock().await = 100.0;
    *session.status.lock().await = "complete".to_string();

    tracing::info!(
        "[p2p] TCP transfer complete: {} ({} bytes sent)",
        session.file_name,
        sent
    );

    Ok(())
}
