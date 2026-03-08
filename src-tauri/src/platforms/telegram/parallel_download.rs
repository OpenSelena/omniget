use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use grammers_client::Client;
use grammers_client::grammers_tl_types as tl;
use grammers_client::types::Downloadable;
use grammers_mtsender::InvocationError;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const PART_SIZE: i32 = 1024 * 1024;

const MAX_RETRIES: u32 = 5;
const RETRY_BASE_DELAY_MS: u64 = 1000;
const FLOOD_WAIT_CODE: i32 = 420;

pub struct MediaLocation {
    pub location: tl::enums::InputFileLocation,
    pub size: u64,
    pub dc_id: i32,
}

pub fn media_to_location(media: &tl::enums::MessageMedia) -> Option<MediaLocation> {
    match media {
        tl::enums::MessageMedia::Document(doc_media) => {
            let doc = match doc_media.document.as_ref()? {
                tl::enums::Document::Document(d) => d,
                tl::enums::Document::Empty(_) => return None,
            };
            let location = tl::enums::InputFileLocation::InputDocumentFileLocation(
                tl::types::InputDocumentFileLocation {
                    id: doc.id,
                    access_hash: doc.access_hash,
                    file_reference: doc.file_reference.clone(),
                    thumb_size: String::new(),
                },
            );
            Some(MediaLocation {
                location,
                size: doc.size as u64,
                dc_id: doc.dc_id,
            })
        }
        tl::enums::MessageMedia::Photo(photo_media) => {
            let photo = match photo_media.photo.as_ref()? {
                tl::enums::Photo::Photo(p) => p,
                tl::enums::Photo::Empty(_) => return None,
            };
            let largest = photo
                .sizes
                .iter()
                .filter_map(|s| match s {
                    tl::enums::PhotoSize::Size(ps) => Some(ps),
                    _ => None,
                })
                .max_by_key(|ps| ps.size)?;

            let location = tl::enums::InputFileLocation::InputPhotoFileLocation(
                tl::types::InputPhotoFileLocation {
                    id: photo.id,
                    access_hash: photo.access_hash,
                    file_reference: photo.file_reference.clone(),
                    thumb_size: largest.r#type.clone(),
                },
            );
            Some(MediaLocation {
                location,
                size: largest.size as u64,
                dc_id: photo.dc_id,
            })
        }
        _ => None,
    }
}

fn extract_messages(result: tl::enums::messages::Messages) -> Vec<tl::enums::Message> {
    match result {
        tl::enums::messages::Messages::Messages(m) => m.messages,
        tl::enums::messages::Messages::Slice(m) => m.messages,
        tl::enums::messages::Messages::ChannelMessages(m) => m.messages,
        tl::enums::messages::Messages::NotModified(_) => vec![],
    }
}

pub async fn fetch_raw_media(
    client: &Client,
    chat_id: i64,
    access_hash: i64,
    is_channel: bool,
    message_id: i32,
) -> anyhow::Result<(tl::enums::MessageMedia, i32)> {
    let msg_input = vec![tl::enums::InputMessage::Id(tl::types::InputMessageId {
        id: message_id,
    })];

    let raw_messages = if is_channel {
        let request = tl::functions::channels::GetMessages {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
            id: msg_input,
        };
        let result = client
            .invoke(&request)
            .await
            .map_err(|e| anyhow::anyhow!("channels.getMessages: {}", e))?;
        extract_messages(result)
    } else {
        let request = tl::functions::messages::GetMessages { id: msg_input };
        let result = client
            .invoke(&request)
            .await
            .map_err(|e| anyhow::anyhow!("messages.getMessages: {}", e))?;
        extract_messages(result)
    };

    let raw_msg = raw_messages
        .into_iter()
        .find_map(|m| match m {
            tl::enums::Message::Message(msg) => Some(msg),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Message {} not found", message_id))?;

    let raw_media = raw_msg
        .media
        .ok_or_else(|| anyhow::anyhow!("Message {} has no media", message_id))?;

    Ok((raw_media, raw_msg.date))
}

fn best_threads(file_size: u64, max: usize) -> usize {
    let threads = if file_size < 1024 * 1024 {
        1
    } else if file_size < 5 * 1024 * 1024 {
        2
    } else if file_size < 20 * 1024 * 1024 {
        4
    } else if file_size < 50 * 1024 * 1024 {
        8
    } else {
        max
    };
    threads.min(max)
}

fn write_at_offset(file: &std::fs::File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_all_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut written = 0usize;
        while written < buf.len() {
            let n = file.seek_write(&buf[written..], offset + written as u64)?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write data",
                ));
            }
            written += n;
        }
        Ok(())
    }
}

async fn invoke_get_file(
    client: &Client,
    dc_id: i32,
    request: &tl::functions::upload::GetFile,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut last_err = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay = RETRY_BASE_DELAY_MS * (1u64 << (attempt - 1).min(4));
            tracing::warn!(
                "[tg-dl] retry {}/{} in {}ms (dc={})",
                attempt, MAX_RETRIES, delay, dc_id
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        let result = client.invoke_in_dc(dc_id, request).await;

        match result {
            Ok(response) => match response {
                tl::enums::upload::File::File(f) => return Ok(f.bytes),
                tl::enums::upload::File::CdnRedirect(_) => {
                    return Err(anyhow::anyhow!("CDN redirect not supported"));
                }
            },
            Err(InvocationError::Rpc(ref err)) if err.code == FLOOD_WAIT_CODE => {
                let wait_secs = err.value.unwrap_or(5) as u64;
                tracing::warn!(
                    "[tg-dl] FLOOD_WAIT_{}, sleeping {}s (dc={})",
                    wait_secs, wait_secs, dc_id
                );
                tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                continue;
            }
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        }
    }

    Err(anyhow::anyhow!(
        "GetFile failed after {} retries (dc={}): {}",
        MAX_RETRIES,
        dc_id,
        last_err
            .map(|e| e.to_string())
            .unwrap_or_default()
    ))
}

pub async fn download_parallel(
    client: &Client,
    media: &MediaLocation,
    output_path: &Path,
    progress_tx: mpsc::Sender<f64>,
    cancel_token: &CancellationToken,
    max_threads: usize,
) -> anyhow::Result<u64> {
    let total_size = media.size;
    let dc_id = media.dc_id;

    tracing::info!(
        "[tg-dl] starting download: size={}, dc={}, path={}",
        total_size, dc_id, output_path.display()
    );

    if total_size < 5 * 1024 * 1024 {
        return download_sequential(
            client, media, output_path, progress_tx, cancel_token,
        )
        .await;
    }

    let threads = best_threads(total_size, max_threads);
    let num_parts = total_size.div_ceil(PART_SIZE as u64);
    tracing::info!(
        "[tg-dl] parallel: parts={}, threads={}, dc={}",
        num_parts, threads, dc_id
    );

    let path_for_create = output_path.to_path_buf();
    let sz = total_size;
    let file = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
        let f = std::fs::File::create(path_for_create)?;
        f.set_len(sz)?;
        Ok(f)
    })
    .await
    .map_err(|e| anyhow::anyhow!("File create panicked: {}", e))??;
    let file = Arc::new(file);

    let semaphore = Arc::new(Semaphore::new(threads));
    let downloaded = Arc::new(AtomicU64::new(0));
    let mut join_set = JoinSet::new();

    for part_idx in 0..num_parts {
        let offset = part_idx * PART_SIZE as u64;
        let expected_len = std::cmp::min(PART_SIZE as u64, total_size - offset);

        let client = client.clone();
        let location = media.location.clone();
        let file = Arc::clone(&file);
        let semaphore = Arc::clone(&semaphore);
        let downloaded = Arc::clone(&downloaded);
        let progress_tx = progress_tx.clone();
        let cancel = cancel_token.clone();

        join_set.spawn(async move {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|e| anyhow::anyhow!("Semaphore closed: {}", e))?;

            if cancel.is_cancelled() {
                return Err(anyhow::anyhow!("Download cancelled"));
            }

            let request = tl::functions::upload::GetFile {
                precise: true,
                cdn_supported: false,
                location: location.clone(),
                offset: offset as i64,
                limit: PART_SIZE,
            };

            let bytes = invoke_get_file(&client, dc_id, &request).await?;

            if (bytes.len() as u64) != expected_len {
                tracing::warn!(
                    "[tg-dl] part {} got {} bytes, expected {}",
                    part_idx, bytes.len(), expected_len
                );
            }

            let chunk_len = bytes.len() as u64;
            let file_ref = Arc::clone(&file);
            let write_offset = offset;
            tokio::task::spawn_blocking(move || write_at_offset(&file_ref, &bytes, write_offset))
                .await
                .map_err(|e| anyhow::anyhow!("Write panicked: {}", e))?
                .map_err(|e| anyhow::anyhow!("Write at offset {} failed: {}", offset, e))?;

            let prev = downloaded.fetch_add(chunk_len, Ordering::Relaxed);
            let new_total = prev + chunk_len;
            if total_size > 0 {
                let percent = (new_total as f64 / total_size as f64) * 100.0;
                let _ = progress_tx.send(percent.min(100.0)).await;
            }

            Ok::<u64, anyhow::Error>(chunk_len)
        });
    }

    let mut total_downloaded: u64 = 0;
    while let Some(result) = join_set.join_next().await {
        if cancel_token.is_cancelled() {
            join_set.abort_all();
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(anyhow::anyhow!("Download cancelled"));
        }
        match result {
            Ok(Ok(bytes)) => total_downloaded += bytes,
            Ok(Err(e)) => {
                join_set.abort_all();
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(e);
            }
            Err(e) => {
                join_set.abort_all();
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(anyhow::anyhow!("Download task panicked: {}", e));
            }
        }
    }

    tracing::info!(
        "[tg-dl] parallel download complete: {} bytes, dc={}",
        total_downloaded, dc_id
    );
    Ok(total_downloaded)
}

async fn download_sequential(
    client: &Client,
    media: &MediaLocation,
    output_path: &Path,
    progress_tx: mpsc::Sender<f64>,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let total_size = media.size;
    let dc_id = media.dc_id;

    let mut file = tokio::fs::File::create(output_path).await?;
    let mut offset: i64 = 0;
    let mut downloaded: u64 = 0;

    loop {
        if cancel_token.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(anyhow::anyhow!("Download cancelled"));
        }

        let request = tl::functions::upload::GetFile {
            precise: true,
            cdn_supported: false,
            location: media.location.clone(),
            offset,
            limit: PART_SIZE,
        };

        let bytes = invoke_get_file(client, dc_id, &request).await?;

        if bytes.is_empty() {
            break;
        }

        file.write_all(&bytes).await?;
        downloaded += bytes.len() as u64;
        offset += bytes.len() as i64;

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            let _ = progress_tx.send(percent.min(100.0)).await;
        }

        if (bytes.len() as i32) < PART_SIZE {
            break;
        }
    }

    file.flush().await?;
    tracing::info!(
        "[tg-dl] sequential download complete: {} bytes, dc={}",
        downloaded, dc_id
    );
    Ok(downloaded)
}

pub async fn download_with_iter(
    client: &Client,
    downloadable: &impl Downloadable,
    total_size: u64,
    output_path: &Path,
    progress_tx: mpsc::Sender<f64>,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    tracing::info!(
        "[tg-dl] fallback iter_download: size={}, path={}",
        total_size, output_path.display()
    );

    let mut download_iter = client.iter_download(downloadable);
    let mut file = tokio::fs::File::create(output_path).await?;
    let mut downloaded: u64 = 0;

    loop {
        if cancel_token.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(output_path).await;
            return Err(anyhow::anyhow!("Download cancelled"));
        }

        match download_iter.next().await {
            Ok(Some(chunk)) => {
                file.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;

                if total_size > 0 {
                    let percent = (downloaded as f64 / total_size as f64) * 100.0;
                    let _ = progress_tx.send(percent.min(100.0)).await;
                }
            }
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = tokio::fs::remove_file(output_path).await;
                return Err(anyhow::anyhow!("iter_download failed: {}", e));
            }
        }
    }

    file.flush().await?;
    tracing::info!("[tg-dl] iter_download complete: {} bytes", downloaded);
    Ok(downloaded)
}
