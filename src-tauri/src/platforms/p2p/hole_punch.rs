use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::models::media::DownloadResult;
use super::protocol::hash_code;

const MAGIC: &[u8; 4] = b"OMNI";
const PKT_PUNCH: u8 = 0x10;
const PKT_PUNCH_ACK: u8 = 0x11;
const PKT_OFFER: u8 = 0x20;
const PKT_ACCEPT: u8 = 0x21;
const PKT_DATA: u8 = 0x30;
const PKT_ACK: u8 = 0x31;
const PKT_FIN: u8 = 0x40;
const PKT_FIN_ACK: u8 = 0x41;

const DATA_PAYLOAD: usize = 1400;
const WINDOW: u32 = 256;
const RETRANSMIT_MS: u64 = 300;
const MAX_PKT: usize = 1500;

fn build_simple(pkt_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(5 + payload.len());
    p.extend_from_slice(MAGIC);
    p.push(pkt_type);
    p.extend_from_slice(payload);
    p
}

fn build_data(seq: u32, data: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(9 + data.len());
    p.extend_from_slice(MAGIC);
    p.push(PKT_DATA);
    p.extend_from_slice(&seq.to_be_bytes());
    p.extend_from_slice(data);
    p
}

fn build_ack(cumulative_seq: u32) -> Vec<u8> {
    let mut p = Vec::with_capacity(9);
    p.extend_from_slice(MAGIC);
    p.push(PKT_ACK);
    p.extend_from_slice(&cumulative_seq.to_be_bytes());
    p
}

fn parse_pkt(data: &[u8]) -> Option<(u8, &[u8])> {
    if data.len() >= 5 && &data[0..4] == MAGIC {
        Some((data[4], &data[5..]))
    } else {
        None
    }
}

pub async fn wait_for_punch(
    socket: &Arc<UdpSocket>,
    expected_hash: &str,
    cancel: &CancellationToken,
) -> anyhow::Result<SocketAddr> {
    let mut buf = [0u8; MAX_PKT];
    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                let (len, src) = result?;
                if let Some((PKT_PUNCH, payload)) = parse_pkt(&buf[..len]) {
                    if let Ok(hash) = std::str::from_utf8(payload) {
                        if hash == expected_hash {
                            let ack = build_simple(PKT_PUNCH_ACK, expected_hash.as_bytes());
                            for _ in 0..3 {
                                let _ = socket.send_to(&ack, src).await;
                            }
                            tracing::info!("[p2p] punch received from {}", src);
                            return Ok(src);
                        }
                    }
                }
            }
            _ = cancel.cancelled() => {
                anyhow::bail!("Cancelled");
            }
        }
    }
}

pub async fn punch_to_sender(
    code: &str,
    sender_endpoint: SocketAddr,
    timeout: Duration,
    cancel: &CancellationToken,
) -> anyhow::Result<Arc<UdpSocket>> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let code_hash = hash_code(code);
    let punch = build_simple(PKT_PUNCH, code_hash.as_bytes());

    let deadline = Instant::now() + timeout;
    let mut buf = [0u8; MAX_PKT];

    loop {
        if Instant::now() > deadline {
            anyhow::bail!("Hole punch timed out after {:?}", timeout);
        }

        let _ = socket.send_to(&punch, sender_endpoint).await;

        tokio::select! {
            result = tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf)) => {
                if let Ok(Ok((len, _))) = result {
                    if let Some((PKT_PUNCH_ACK, payload)) = parse_pkt(&buf[..len]) {
                        if let Ok(hash) = std::str::from_utf8(payload) {
                            if hash == code_hash {
                                tracing::info!("[p2p] hole punch succeeded to {}", sender_endpoint);
                                return Ok(socket);
                            }
                        }
                    }
                }
            }
            _ = cancel.cancelled() => {
                anyhow::bail!("Cancelled during hole punch");
            }
        }
    }
}

pub async fn send_file_udp(
    socket: &Arc<UdpSocket>,
    peer: SocketAddr,
    file_path: &std::path::Path,
    file_name: &str,
    file_size: u64,
    progress: &Arc<tokio::sync::Mutex<f64>>,
    sent_bytes: &Arc<tokio::sync::Mutex<u64>>,
    cancel: &CancellationToken,
    paused: &std::sync::atomic::AtomicBool,
) -> anyhow::Result<()> {
    let mut offer_payload = Vec::with_capacity(8 + file_name.len());
    offer_payload.extend_from_slice(&file_size.to_be_bytes());
    offer_payload.extend_from_slice(file_name.as_bytes());
    let offer_pkt = build_simple(PKT_OFFER, &offer_payload);

    let mut buf = [0u8; MAX_PKT];
    let accepted = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            socket.send_to(&offer_pkt, peer).await?;
            match tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf)).await {
                Ok(Ok((len, _))) => {
                    if let Some((PKT_ACCEPT, _)) = parse_pkt(&buf[..len]) {
                        return Ok::<_, anyhow::Error>(true);
                    }
                }
                _ => continue,
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Accept timeout"))??;

    if !accepted {
        anyhow::bail!("Transfer rejected");
    }

    let mut file = File::open(file_path).await?;
    let mut next_seq: u32 = 0;
    let mut base_seq: u32 = 0;
    let mut window: BTreeMap<u32, (Vec<u8>, Instant)> = BTreeMap::new();
    let mut file_done = false;
    let mut chunk_buf = vec![0u8; DATA_PAYLOAD];

    loop {
        if cancel.is_cancelled() {
            anyhow::bail!("Send cancelled");
        }

        while paused.load(std::sync::atomic::Ordering::Relaxed) {
            if cancel.is_cancelled() {
                anyhow::bail!("Cancelled while paused");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        while !file_done && next_seq < base_seq + WINDOW {
            let n = file.read(&mut chunk_buf).await?;
            if n == 0 {
                file_done = true;
                break;
            }
            let pkt = build_data(next_seq, &chunk_buf[..n]);
            let _ = socket.send_to(&pkt, peer).await;
            window.insert(next_seq, (pkt, Instant::now()));
            next_seq += 1;
        }

        if file_done && window.is_empty() {
            break;
        }

        match tokio::time::timeout(
            Duration::from_millis(RETRANSMIT_MS),
            socket.recv_from(&mut buf),
        )
        .await
        {
            Ok(Ok((len, _))) => {
                if let Some((PKT_ACK, payload)) = parse_pkt(&buf[..len]) {
                    if payload.len() >= 4 {
                        let ack_seq =
                            u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        while base_seq <= ack_seq {
                            window.remove(&base_seq);
                            base_seq += 1;
                        }
                        let bytes_acked =
                            (base_seq as u64 * DATA_PAYLOAD as u64).min(file_size);
                        let pct = if file_size > 0 {
                            (bytes_acked as f64 / file_size as f64) * 100.0
                        } else {
                            100.0
                        };
                        *progress.lock().await = pct.min(100.0);
                        *sent_bytes.lock().await = bytes_acked;
                    }
                }
            }
            _ => {
                let now = Instant::now();
                for (_, (pkt, sent_at)) in &mut window {
                    if now.duration_since(*sent_at) >= Duration::from_millis(RETRANSMIT_MS) {
                        let _ = socket.send_to(pkt, peer).await;
                        *sent_at = now;
                    }
                }
            }
        }
    }

    let fin = build_simple(PKT_FIN, &[]);
    for _ in 0..10 {
        let _ = socket.send_to(&fin, peer).await;
        match tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf)).await {
            Ok(Ok((len, _))) => {
                if let Some((PKT_FIN_ACK, _)) = parse_pkt(&buf[..len]) {
                    break;
                }
            }
            _ => continue,
        }
    }

    *progress.lock().await = 100.0;
    *sent_bytes.lock().await = file_size;
    tracing::info!("[p2p] UDP send complete: {} bytes", file_size);
    Ok(())
}

pub async fn receive_file_udp(
    socket: &Arc<UdpSocket>,
    sender_addr: SocketAddr,
    output_dir: &std::path::Path,
    progress_tx: &mpsc::Sender<f64>,
    cancel: &CancellationToken,
) -> anyhow::Result<DownloadResult> {
    let mut buf = [0u8; MAX_PKT];

    let (file_name, file_size) = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (len, _) = socket.recv_from(&mut buf).await?;
            if let Some((PKT_OFFER, payload)) = parse_pkt(&buf[..len]) {
                if payload.len() >= 8 {
                    let size = u64::from_be_bytes(payload[0..8].try_into().unwrap());
                    let name = std::str::from_utf8(&payload[8..])
                        .unwrap_or("file")
                        .to_string();
                    return Ok::<_, anyhow::Error>((name, size));
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Offer timeout"))??;

    tracing::info!("[p2p] UDP offer: {} ({} bytes)", file_name, file_size);

    let accept = build_simple(PKT_ACCEPT, &[]);
    for _ in 0..3 {
        let _ = socket.send_to(&accept, sender_addr).await;
    }

    let _ = progress_tx.send(0.0).await;

    let safe_name = sanitize_filename::sanitize(&file_name);
    let mut output_path = output_dir.join(&safe_name);
    tokio::fs::create_dir_all(output_dir).await?;

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

    let mut expected_seq: u32 = 0;
    let mut ooo_buf: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    let mut received_bytes: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            drop(writer);
            let _ = tokio::fs::remove_file(&output_path).await;
            anyhow::bail!("Transfer cancelled");
        }

        let recv = tokio::time::timeout(Duration::from_secs(30), socket.recv_from(&mut buf)).await;

        match recv {
            Ok(Ok((len, _))) => {
                if let Some((pkt_type, payload)) = parse_pkt(&buf[..len]) {
                    match pkt_type {
                        PKT_DATA if payload.len() >= 4 => {
                            let seq = u32::from_be_bytes([
                                payload[0], payload[1], payload[2], payload[3],
                            ]);
                            let data = &payload[4..];

                            if seq == expected_seq {
                                writer.write_all(data).await?;
                                received_bytes += data.len() as u64;
                                expected_seq += 1;

                                while let Some(ooo_data) = ooo_buf.remove(&expected_seq) {
                                    writer.write_all(&ooo_data).await?;
                                    received_bytes += ooo_data.len() as u64;
                                    expected_seq += 1;
                                }
                            } else if seq > expected_seq && seq < expected_seq + WINDOW * 2 {
                                ooo_buf.entry(seq).or_insert_with(|| data.to_vec());
                            }

                            let ack = build_ack(expected_seq.saturating_sub(1));
                            let _ = socket.send_to(&ack, sender_addr).await;

                            if file_size > 0 {
                                let pct =
                                    (received_bytes as f64 / file_size as f64) * 100.0;
                                let _ = progress_tx.send(pct.min(100.0)).await;
                            }
                        }
                        PKT_FIN => {
                            let fin_ack = build_simple(PKT_FIN_ACK, &[]);
                            for _ in 0..3 {
                                let _ = socket.send_to(&fin_ack, sender_addr).await;
                            }
                            break;
                        }
                        PKT_OFFER => {
                            for _ in 0..3 {
                                let _ = socket.send_to(&accept, sender_addr).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("[p2p] UDP recv error: {}", e);
            }
            Err(_) => {
                anyhow::bail!("Transfer timed out (no data for 30s)");
            }
        }
    }

    writer.flush().await?;
    drop(writer);

    let _ = progress_tx.send(100.0).await;
    tracing::info!(
        "[p2p] UDP receive complete: {} ({} bytes)",
        safe_name,
        received_bytes
    );

    Ok(DownloadResult {
        file_path: output_path,
        file_size_bytes: received_bytes,
        duration_seconds: 0.0,
        torrent_id: None,
    })
}
