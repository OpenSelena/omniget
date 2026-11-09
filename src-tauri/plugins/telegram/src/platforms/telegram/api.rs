use std::path::{Path, PathBuf};
use std::time::Duration;

use grammers_client::grammers_tl_types as tl;
use grammers_client::types::Peer;
use grammers_client::Client;
use grammers_tl_types::Serializable;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::auth::TelegramSessionHandle;

/// Maps a MIME type to a file extension (with leading dot).
/// Covers the most common Telegram media MIME types.
fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        // Video
        "video/mp4" => ".mp4",
        "video/x-matroska" => ".mkv",
        "video/webm" => ".webm",
        "video/quicktime" => ".mov",
        "video/x-msvideo" => ".avi",
        "video/mpeg" => ".mpeg",
        "video/3gpp" => ".3gp",
        "video/x-flv" => ".flv",
        // Audio
        "audio/mpeg" | "audio/mp3" => ".mp3",
        "audio/ogg" => ".ogg",
        "audio/x-opus+ogg" => ".opus",
        "audio/flac" | "audio/x-flac" => ".flac",
        "audio/x-wav" | "audio/wav" => ".wav",
        "audio/aac" | "audio/x-aac" => ".aac",
        "audio/mp4" | "audio/x-m4a" => ".m4a",
        "audio/x-ms-wma" => ".wma",
        // Image
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "image/bmp" => ".bmp",
        "image/svg+xml" => ".svg",
        "image/tiff" => ".tiff",
        // Documents
        "application/pdf" => ".pdf",
        "application/zip" => ".zip",
        "application/x-rar-compressed" | "application/vnd.rar" => ".rar",
        "application/x-7z-compressed" => ".7z",
        "application/gzip" | "application/x-gzip" => ".gz",
        "application/x-tar" => ".tar",
        "application/json" => ".json",
        "application/xml" | "text/xml" => ".xml",
        "text/plain" => ".txt",
        "text/html" => ".html",
        "text/csv" => ".csv",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => ".docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => ".pptx",
        "application/msword" => ".doc",
        "application/vnd.ms-excel" => ".xls",
        "application/vnd.ms-powerpoint" => ".ppt",
        "application/x-python-script" | "text/x-python" => ".py",
        "application/javascript" | "text/javascript" => ".js",
        // Fallback: try to extract from the subtype
        other => {
            if let Some(sub) = other.split('/').nth(1) {
                // Common pattern: "video/mp4" → ".mp4"
                // But we only return static strings, so we use a catch-all
                match sub {
                    "mp4" => ".mp4",
                    "mpeg" => ".mpeg",
                    "ogg" => ".ogg",
                    "webm" => ".webm",
                    "flac" => ".flac",
                    "wav" => ".wav",
                    "jpeg" => ".jpg",
                    "png" => ".png",
                    "gif" => ".gif",
                    _ => "",
                }
            } else {
                ""
            }
        }
    }
}

/// Ensures a filename has a proper extension based on its MIME type.
/// If the file already has a known extension, keeps it.
/// If not, appends the MIME-derived extension.
fn ensure_extension(name: &str, mime_type: &str) -> String {
    let path = Path::new(name);
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        // Already has a recognized extension
        let known = matches!(
            ext_str.as_str(),
            "mp4"
                | "mkv"
                | "webm"
                | "mov"
                | "avi"
                | "mpeg"
                | "3gp"
                | "flv"
                | "mp3"
                | "ogg"
                | "opus"
                | "flac"
                | "wav"
                | "aac"
                | "m4a"
                | "wma"
                | "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "webp"
                | "bmp"
                | "svg"
                | "tiff"
                | "pdf"
                | "zip"
                | "rar"
                | "7z"
                | "gz"
                | "tar"
                | "json"
                | "xml"
                | "txt"
                | "html"
                | "csv"
                | "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "ppt"
                | "pptx"
                | "py"
                | "js"
                | "ts"
                | "rs"
                | "go"
                | "c"
                | "cpp"
                | "h"
        );
        if known {
            return name.to_string();
        }
    }

    // No extension or unknown extension — add from MIME type
    let ext = mime_to_ext(mime_type);
    if ext.is_empty() {
        name.to_string()
    } else {
        format!("{}{}", name, ext)
    }
}

fn parse_flood_wait(err: &str) -> Option<u64> {
    for pattern in &["FLOOD_WAIT_", "FLOOD_PREMIUM_WAIT_"] {
        if let Some(pos) = err.find(pattern) {
            let after = &err[pos + pattern.len()..];
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(secs) = digits.parse::<u64>() {
                return Some(secs);
            }
        }
    }
    None
}

async fn invoke_with_flood_wait<R>(
    client: &Client,
    request: &R,
) -> Result<R::Return, grammers_mtsender::InvocationError>
where
    R: grammers_tl_types::RemoteCall + Serializable,
{
    for attempt in 0..3u32 {
        match client.invoke(request).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let err_str = e.to_string();
                if let Some(secs) = parse_flood_wait(&err_str) {
                    let wait = secs + 1;
                    tracing::warn!(
                        "[tg-api] FLOOD_WAIT_{} on attempt {}, waiting {}s",
                        secs,
                        attempt + 1,
                        wait
                    );
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                return Err(e);
            }
        }
    }
    client.invoke(request).await
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    pub title: String,
    pub chat_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_hash: Option<i64>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TelegramWebpageInfo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<i32>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TelegramMediaItem {
    pub message_id: i32,
    pub file_name: String,
    pub file_size: u64,
    pub media_type: String,
    pub date: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webpage: Option<TelegramWebpageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouped_id: Option<i64>,
}

fn media_filter(media_type: Option<&str>) -> tl::enums::MessagesFilter {
    match media_type {
        Some("photo") => tl::enums::MessagesFilter::InputMessagesFilterPhotos,
        Some("video") => tl::enums::MessagesFilter::InputMessagesFilterVideo,
        Some("photo_video") => tl::enums::MessagesFilter::InputMessagesFilterPhotoVideo,
        Some("document") => tl::enums::MessagesFilter::InputMessagesFilterDocument,
        Some("audio") => tl::enums::MessagesFilter::InputMessagesFilterMusic,
        Some("webpage") => tl::enums::MessagesFilter::InputMessagesFilterUrl,
        Some("gif") => tl::enums::MessagesFilter::InputMessagesFilterGif,
        Some("round_video") => tl::enums::MessagesFilter::InputMessagesFilterRoundVideo,
        Some("round_voice") => tl::enums::MessagesFilter::InputMessagesFilterRoundVoice,
        Some("chat_photos") => tl::enums::MessagesFilter::InputMessagesFilterChatPhotos,
        _ => tl::enums::MessagesFilter::InputMessagesFilterEmpty,
    }
}

fn extract_raw_media_info_with_webpage(
    media: &tl::enums::MessageMedia,
    fix_extensions: bool,
) -> Option<(String, u64, String, Option<TelegramWebpageInfo>)> {
    match media {
        tl::enums::MessageMedia::Photo(photo_media) => {
            let photo = match photo_media.photo.as_ref()? {
                tl::enums::Photo::Photo(p) => p,
                tl::enums::Photo::Empty(_) => return None,
            };
            let name = format!("photo_{}.jpg", photo.id);
            let size = photo
                .sizes
                .iter()
                .filter_map(|s| match s {
                    tl::enums::PhotoSize::Size(ps) => Some(ps.size as u64),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            Some((name, size, "photo".to_string(), None))
        }
        tl::enums::MessageMedia::Document(doc_media) => {
            let doc = match doc_media.document.as_ref()? {
                tl::enums::Document::Document(d) => d,
                tl::enums::Document::Empty(_) => return None,
            };
            let raw_name = doc
                .attributes
                .iter()
                .find_map(|attr| {
                    if let tl::enums::DocumentAttribute::Filename(f) = attr {
                        Some(f.file_name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let ext = mime_to_ext(&doc.mime_type);
                    if ext.is_empty() {
                        format!("file_{}", doc.id)
                    } else {
                        format!("{}{}", doc.id, ext)
                    }
                });
            let name = if fix_extensions {
                ensure_extension(&raw_name, &doc.mime_type)
            } else {
                raw_name
            };
            let size = doc.size as u64;
            let is_animated = doc
                .attributes
                .iter()
                .any(|attr| matches!(attr, tl::enums::DocumentAttribute::Animated));
            let mt = if doc_media.round {
                "round_video"
            } else if doc_media.voice {
                "round_voice"
            } else if is_animated || doc.mime_type == "image/gif" {
                "gif"
            } else if doc.mime_type.starts_with("video/") {
                "video"
            } else if doc.mime_type.starts_with("audio/") {
                "audio"
            } else if doc.mime_type.starts_with("image/") {
                "photo"
            } else {
                "document"
            };
            Some((name, size, mt.to_string(), None))
        }
        tl::enums::MessageMedia::WebPage(wp_media) => match &wp_media.webpage {
            tl::enums::WebPage::Page(page) => {
                let name = page
                    .title
                    .clone()
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| page.url.clone());
                let info = TelegramWebpageInfo {
                    url: page.url.clone(),
                    site_name: page.site_name.clone(),
                    title: page.title.clone(),
                    description: page.description.clone(),
                    embed_url: page.embed_url.clone(),
                    embed_type: page.embed_type.clone(),
                    duration_sec: page.duration,
                };
                Some((name, 0, "webpage".to_string(), Some(info)))
            }
            _ => None,
        },
        _ => None,
    }
}

pub async fn list_chats(handle: &TelegramSessionHandle) -> anyhow::Result<Vec<TelegramChat>> {
    let _t = std::time::Instant::now();
    let guard = handle.lock().await;
    let client = guard
        .client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
        .clone();
    drop(guard);

    let fetch_dialogs = async {
        let mut dialogs = client.iter_dialogs();
        let mut chats = Vec::new();
        let mut peer_hashes = std::collections::HashMap::new();

        while let Some(dialog) = dialogs.next().await.map_err(|e| anyhow::anyhow!("{}", e))? {
            let peer = dialog.peer();
            let chat_type = match peer {
                Peer::User(_) => "private",
                Peer::Group(_) => "group",
                Peer::Channel(_) => "channel",
            };
            let title = peer.name().unwrap_or("Unknown").to_string();

            let peer_data = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                use grammers_client::session::defs::PeerRef;
                let peer_ref = PeerRef::from(peer);
                (peer_ref.id.bare_id(), peer_ref.auth.hash())
            }));

            let (id, access_hash) = match peer_data {
                Ok(data) => data,
                Err(_) => {
                    tracing::warn!(
                        "[tg-api] skipping peer with unsupported ID (title={:?})",
                        title
                    );
                    continue;
                }
            };

            peer_hashes.insert(id, access_hash);

            chats.push(TelegramChat {
                id,
                title,
                chat_type: chat_type.to_string(),
                peer_hash: Some(access_hash),
            });
        }

        Ok::<_, anyhow::Error>((chats, peer_hashes))
    };

    let (chats, peer_hashes) = tokio::time::timeout(Duration::from_secs(30), fetch_dialogs)
        .await
        .map_err(|_| anyhow::anyhow!("Loading chats timed out, please try again"))??;

    tracing::info!(
        "[tg-perf] list_chats completed in {:?}, loaded {} chats",
        _t.elapsed(),
        chats.len()
    );

    let mut guard = handle.lock().await;
    guard.peer_hashes.extend(peer_hashes);
    drop(guard);

    Ok(chats)
}

pub fn make_input_peer(chat_id: i64, chat_type: &str, access_hash: i64) -> tl::enums::InputPeer {
    match chat_type {
        "private" => tl::enums::InputPeer::User(tl::types::InputPeerUser {
            user_id: chat_id,
            access_hash,
        }),
        "group" => {
            if access_hash != 0 {
                tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
                    channel_id: chat_id,
                    access_hash,
                })
            } else {
                tl::enums::InputPeer::Chat(tl::types::InputPeerChat { chat_id })
            }
        }
        "channel" => tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
            channel_id: chat_id,
            access_hash,
        }),
        _ => tl::enums::InputPeer::Empty,
    }
}

async fn fetch_media_page(
    client: &Client,
    input_peer: &tl::enums::InputPeer,
    media_type: Option<&str>,
    query: &str,
    offset: i32,
    limit: i32,
    fix_extensions: bool,
) -> anyhow::Result<Vec<TelegramMediaItem>> {
    let filter = media_filter(media_type);
    let request = tl::functions::messages::Search {
        peer: input_peer.clone(),
        q: query.to_string(),
        from_id: None,
        saved_peer_id: None,
        saved_reaction: None,
        top_msg_id: None,
        filter,
        min_date: 0,
        max_date: 0,
        offset_id: offset,
        add_offset: 0,
        limit,
        max_id: 0,
        min_id: 0,
        hash: 0,
    };

    let result = invoke_with_flood_wait(client, &request)
        .await
        .map_err(|e| anyhow::anyhow!("messages.Search failed: {}", e))?;

    let messages = match result {
        tl::enums::messages::Messages::Messages(m) => m.messages,
        tl::enums::messages::Messages::Slice(m) => m.messages,
        tl::enums::messages::Messages::ChannelMessages(m) => m.messages,
        tl::enums::messages::Messages::NotModified(_) => vec![],
    };

    let mut items = Vec::new();
    for raw_msg in messages {
        let msg = match raw_msg {
            tl::enums::Message::Message(m) => m,
            _ => continue,
        };
        let caption = if msg.message.trim().is_empty() {
            None
        } else {
            Some(msg.message.clone())
        };
        let grouped_id = msg.grouped_id;
        if let Some(raw_media) = msg.media {
            if let Some((file_name, file_size, media_type_str, webpage)) =
                extract_raw_media_info_with_webpage(&raw_media, fix_extensions)
            {
                items.push(TelegramMediaItem {
                    message_id: msg.id,
                    file_name,
                    file_size,
                    media_type: media_type_str,
                    date: msg.date as i64,
                    webpage,
                    caption,
                    grouped_id,
                });
            }
        }
    }

    Ok(items)
}

fn merge_dedup_media(
    results: [anyhow::Result<Vec<TelegramMediaItem>>; 4],
    limit: usize,
) -> Vec<TelegramMediaItem> {
    let mut all_items = Vec::new();
    for items in results.into_iter().flatten() {
        all_items.extend(items);
    }
    let mut seen = std::collections::HashSet::new();
    all_items.retain(|item| seen.insert(item.message_id));
    all_items.sort_by_key(|b| std::cmp::Reverse(b.date));
    all_items.truncate(limit);
    all_items
}

pub async fn list_media(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    media_type: Option<&str>,
    offset: i32,
    limit: u32,
    fix_extensions: bool,
) -> anyhow::Result<Vec<TelegramMediaItem>> {
    let _t = std::time::Instant::now();
    let guard = handle.lock().await;
    let client = guard
        .client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
        .clone();
    let access_hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
    drop(guard);

    tracing::debug!(
        "[tg-api] list_media: chat_id={}, type={}, hash={}, filter={:?}",
        chat_id,
        chat_type,
        access_hash,
        media_type
    );

    let input_peer = make_input_peer(chat_id, chat_type, access_hash);

    let fetch = async {
        if media_type.is_none() {
            let (photos, videos, docs, audio) = tokio::join!(
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("photo"),
                    "",
                    offset,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("video"),
                    "",
                    offset,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("document"),
                    "",
                    offset,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("audio"),
                    "",
                    offset,
                    limit as i32,
                    fix_extensions
                ),
            );
            let items = merge_dedup_media([photos, videos, docs, audio], limit as usize);
            return Ok(items);
        }

        fetch_media_page(
            &client,
            &input_peer,
            media_type,
            "",
            offset,
            limit as i32,
            fix_extensions,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    };

    let items = tokio::time::timeout(Duration::from_secs(30), fetch)
        .await
        .map_err(|_| anyhow::anyhow!("Loading media timed out — please try again"))??;

    tracing::info!(
        "[tg-perf] list_media completed in {:?}, found {} items",
        _t.elapsed(),
        items.len()
    );
    Ok(items)
}

pub async fn search_media(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    query: &str,
    media_type: Option<&str>,
    limit: u32,
    fix_extensions: bool,
) -> anyhow::Result<Vec<TelegramMediaItem>> {
    let _t = std::time::Instant::now();
    let guard = handle.lock().await;
    let client = guard
        .client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
        .clone();
    let access_hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
    drop(guard);

    let input_peer = make_input_peer(chat_id, chat_type, access_hash);

    let fetch = async {
        if media_type.is_none() {
            let (photos, videos, docs, audio) = tokio::join!(
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("photo"),
                    query,
                    0,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("video"),
                    query,
                    0,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("document"),
                    query,
                    0,
                    limit as i32,
                    fix_extensions
                ),
                fetch_media_page(
                    &client,
                    &input_peer,
                    Some("audio"),
                    query,
                    0,
                    limit as i32,
                    fix_extensions
                ),
            );
            let items = merge_dedup_media([photos, videos, docs, audio], limit as usize);
            return Ok(items);
        }

        fetch_media_page(
            &client,
            &input_peer,
            media_type,
            query,
            0,
            limit as i32,
            fix_extensions,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    };

    let items = tokio::time::timeout(Duration::from_secs(30), fetch)
        .await
        .map_err(|_| anyhow::anyhow!("Search timed out — please try again"))??;

    tracing::info!(
        "[tg-perf] search_media completed in {:?}, found {} items for query={:?}",
        _t.elapsed(),
        items.len(),
        query
    );
    Ok(items)
}

pub async fn download_media(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_id: i32,
    output_path: &Path,
    progress_tx: mpsc::Sender<omniget_core::models::progress::ProgressUpdate>,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let _t = std::time::Instant::now();
    let guard = handle.lock().await;
    if guard.client.is_none() {
        tracing::warn!("[tg-perf] download_media: client is None (not authenticated)");
    }
    let client = guard
        .client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
        .clone();
    let access_hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
    drop(guard);

    let is_auth = client.is_authorized().await.unwrap_or(false);
    tracing::info!(
        "[tg-diag] download_media: is_authorized={}, chat_id={}, msg_id={}",
        is_auth,
        chat_id,
        message_id
    );
    if !is_auth {
        tracing::error!("[tg-diag] download_media: client not authorized");
    }

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel && access_hash == 0 {
        tracing::warn!(
            "[tg-diag] download_media: access_hash=0 for channel/supergroup chat_id={}, download will likely fail",
            chat_id
        );
    }

    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let tmp_path = PathBuf::from(format!("{}.tmp", output_path.display()));
    const MAX_REF_RETRIES: u32 = 2;

    for ref_attempt in 0..=MAX_REF_RETRIES {
        let (raw_media, msg_date) = super::parallel_download::fetch_raw_media(
            &client,
            chat_id,
            access_hash,
            is_channel,
            message_id,
        )
        .await?;

        let media = super::parallel_download::media_to_location(&raw_media)
            .ok_or_else(|| anyhow::anyhow!("Unsupported media type"))?;

        if ref_attempt == 0 {
            tracing::info!(
                "[tg-perf] download_media: size={}, dc={}",
                media.size,
                media.dc_id
            );
        }

        let result = super::parallel_download::download_file(
            &client,
            &media,
            &tmp_path,
            progress_tx.clone(),
            cancel_token,
        )
        .await;

        match result {
            Ok(downloaded) => {
                tokio::fs::rename(&tmp_path, output_path).await?;
                let ts = msg_date as i64;
                if ts > 0 {
                    let file_time = filetime::FileTime::from_unix_time(ts, 0);
                    if let Err(e) = filetime::set_file_mtime(output_path, file_time) {
                        tracing::warn!("[tg-api] failed to set file time: {}", e);
                    }
                }
                tracing::info!("[tg-perf] download_media completed in {:?}", _t.elapsed());
                return Ok(downloaded);
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                let err_lower = e.to_string().to_lowercase();
                if err_lower.contains("file_reference") && ref_attempt < MAX_REF_RETRIES {
                    tracing::warn!(
                        "[tg-diag] download_media: FILE_REFERENCE expired, re-fetching message ({}/{})",
                        ref_attempt + 1, MAX_REF_RETRIES
                    );
                    continue;
                }
                tracing::info!("[tg-perf] download_media failed in {:?}", _t.elapsed());
                return Err(e);
            }
        }
    }

    unreachable!()
}

fn is_retryable_error(err_str: &str) -> bool {
    let lower = err_str.to_lowercase();
    if lower.contains("rpc error 400")
        || lower.contains("rpc error 401")
        || lower.contains("rpc error 403")
        || lower.contains("rpc error 404")
    {
        return false;
    }
    let retryable = [
        "connection reset",
        "timed out",
        "connection refused",
        "broken pipe",
        "unexpected eof",
        "internal server error",
        "temporarily unavailable",
        "transport error",
        "network",
        "rpc error",
    ];
    retryable.iter().any(|p| lower.contains(p))
}

pub async fn download_media_with_retry(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_id: i32,
    output_path: &Path,
    progress_tx: mpsc::Sender<omniget_core::models::progress::ProgressUpdate>,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    let _t = std::time::Instant::now();
    const MAX_RETRIES: u32 = 5;
    const BASE_DELAY_SECS: u64 = 2;

    for attempt in 0..MAX_RETRIES {
        let tx = progress_tx.clone();
        let result = tokio::select! {
            r = download_media(handle, chat_id, chat_type, message_id, output_path, tx, cancel_token) => r,
            _ = cancel_token.cancelled() => return Err(anyhow::anyhow!("Download cancelled")),
        };

        match result {
            Ok(size) => {
                tracing::info!(
                    "[tg-perf] download_media_with_retry completed in {:?}",
                    _t.elapsed()
                );
                return Ok(size);
            }
            Err(e) => {
                let err_str = e.to_string();

                if parse_flood_wait(&err_str).is_some() {
                    // Flood wait is handled inside invoke_with_flood_wait,
                    // but if it bubbles up, retry
                    tracing::warn!(
                        "[tg-api] flood wait error on attempt {}, retrying: {}",
                        attempt + 1,
                        err_str
                    );
                } else if !is_retryable_error(&err_str) {
                    tracing::info!(
                        "[tg-perf] download_media_with_retry failed (non-retryable) in {:?}",
                        _t.elapsed()
                    );
                    return Err(e);
                }

                if attempt + 1 < MAX_RETRIES {
                    let delay = BASE_DELAY_SECS * 2u64.pow(attempt);
                    tracing::warn!(
                        "[tg-api] download attempt {} failed, retrying in {}s: {}",
                        attempt + 1,
                        delay,
                        err_str
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(delay)) => {},
                        _ = cancel_token.cancelled() => return Err(anyhow::anyhow!("Download cancelled")),
                    }
                } else {
                    tracing::info!(
                        "[tg-perf] download_media_with_retry failed (max retries) in {:?}",
                        _t.elapsed()
                    );
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TelegramSelf {
    pub user_id: i64,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub phone: Option<String>,
}

pub async fn get_self(handle: &TelegramSessionHandle) -> anyhow::Result<TelegramSelf> {
    let (client, fallback_phone) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        (client, guard.phone.clone())
    };

    let req = tl::functions::users::GetUsers {
        id: vec![tl::enums::InputUser::UserSelf],
    };
    let users = invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("users.getUsers failed: {}", e))?;

    if let Some(tl::enums::User::User(u)) = users.into_iter().next() {
        return Ok(TelegramSelf {
            user_id: u.id,
            username: u.username,
            first_name: u.first_name.unwrap_or_default(),
            last_name: u.last_name,
            phone: u
                .phone
                .or_else(|| (!fallback_phone.is_empty()).then(|| fallback_phone.clone())),
        });
    }

    let me = client.get_me().await?;
    Ok(TelegramSelf {
        user_id: me.bare_id(),
        username: me.username().map(str::to_string),
        first_name: me.first_name().unwrap_or_default().to_string(),
        last_name: me.last_name().map(str::to_string),
        phone: me
            .phone()
            .map(str::to_string)
            .or_else(|| (!fallback_phone.is_empty()).then_some(fallback_phone)),
    })
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TelegramGlobalSearchHit {
    pub message_id: i32,
    pub file_name: String,
    pub file_size: u64,
    pub media_type: String,
    pub date: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webpage: Option<TelegramWebpageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouped_id: Option<i64>,
    pub chat_id: i64,
    pub chat_type: String,
    pub chat_title: String,
}

async fn fetch_global_search_filter(
    client: &Client,
    query: &str,
    filter: tl::enums::MessagesFilter,
    limit: i32,
    fix_extensions: bool,
) -> anyhow::Result<(
    Vec<TelegramGlobalSearchHit>,
    std::collections::HashMap<i64, i64>,
)> {
    let request = tl::functions::messages::SearchGlobal {
        broadcasts_only: false,
        groups_only: false,
        users_only: false,
        folder_id: None,
        q: query.to_string(),
        filter,
        min_date: 0,
        max_date: 0,
        offset_rate: 0,
        offset_peer: tl::enums::InputPeer::Empty,
        offset_id: 0,
        limit,
    };

    let result = invoke_with_flood_wait(client, &request)
        .await
        .map_err(|e| anyhow::anyhow!("messages.searchGlobal failed: {}", e))?;

    let (messages, chats, users) = match result {
        tl::enums::messages::Messages::Messages(m) => (m.messages, m.chats, m.users),
        tl::enums::messages::Messages::Slice(m) => (m.messages, m.chats, m.users),
        tl::enums::messages::Messages::ChannelMessages(m) => (m.messages, m.chats, m.users),
        tl::enums::messages::Messages::NotModified(_) => (vec![], vec![], vec![]),
    };

    let mut peer_meta: std::collections::HashMap<i64, (String, String)> =
        std::collections::HashMap::new();
    let mut discovered_hashes: std::collections::HashMap<i64, i64> =
        std::collections::HashMap::new();

    for u in users {
        if let tl::enums::User::User(user) = u {
            let full_name = match (&user.first_name, &user.last_name) {
                (Some(f), Some(l)) if !l.is_empty() => format!("{} {}", f, l),
                (Some(f), _) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => user
                    .username
                    .clone()
                    .unwrap_or_else(|| format!("User {}", user.id)),
            };
            if let Some(hash) = user.access_hash {
                discovered_hashes.insert(user.id, hash);
            }
            peer_meta.insert(user.id, ("private".to_string(), full_name));
        }
    }

    for c in chats {
        match c {
            tl::enums::Chat::Chat(chat) => {
                discovered_hashes.insert(chat.id, 0);
                peer_meta.insert(chat.id, ("group".to_string(), chat.title));
            }
            tl::enums::Chat::Forbidden(chat) => {
                peer_meta.insert(chat.id, ("group".to_string(), chat.title));
            }
            tl::enums::Chat::Channel(ch) => {
                let ctype = if ch.megagroup { "group" } else { "channel" };
                if let Some(hash) = ch.access_hash {
                    discovered_hashes.insert(ch.id, hash);
                }
                peer_meta.insert(ch.id, (ctype.to_string(), ch.title));
            }
            tl::enums::Chat::ChannelForbidden(ch) => {
                let ctype = if ch.megagroup { "group" } else { "channel" };
                discovered_hashes.insert(ch.id, ch.access_hash);
                peer_meta.insert(ch.id, (ctype.to_string(), ch.title));
            }
            tl::enums::Chat::Empty(_) => {}
        }
    }

    let mut hits = Vec::new();
    for raw_msg in messages {
        let msg = match raw_msg {
            tl::enums::Message::Message(m) => m,
            _ => continue,
        };
        let raw_media = match msg.media {
            Some(ref m) => m,
            None => continue,
        };
        let (file_name, file_size, media_type, webpage) =
            match extract_raw_media_info_with_webpage(raw_media, fix_extensions) {
                Some(info) => info,
                None => continue,
            };

        let (chat_id, default_type) = match msg.peer_id {
            tl::enums::Peer::User(u) => (u.user_id, "private"),
            tl::enums::Peer::Chat(c) => (c.chat_id, "group"),
            tl::enums::Peer::Channel(ch) => (ch.channel_id, "channel"),
        };

        let (chat_type, chat_title) = peer_meta
            .get(&chat_id)
            .cloned()
            .unwrap_or_else(|| (default_type.to_string(), format!("Chat {}", chat_id)));

        let caption = if msg.message.trim().is_empty() {
            None
        } else {
            Some(msg.message.clone())
        };

        hits.push(TelegramGlobalSearchHit {
            message_id: msg.id,
            file_name,
            file_size,
            media_type,
            date: msg.date as i64,
            webpage,
            caption,
            grouped_id: msg.grouped_id,
            chat_id,
            chat_type,
            chat_title,
        });
    }

    Ok((hits, discovered_hashes))
}

pub async fn search_global_hits(
    handle: &TelegramSessionHandle,
    query: &str,
    limit: u32,
    fix_extensions: bool,
) -> anyhow::Result<Vec<TelegramGlobalSearchHit>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };

    let lim = limit.clamp(1, 100) as i32;
    let fetch = async {
        let (docs, videos, photos, audio, general) = tokio::join!(
            fetch_global_search_filter(
                &client,
                q,
                tl::enums::MessagesFilter::InputMessagesFilterDocument,
                lim,
                fix_extensions
            ),
            fetch_global_search_filter(
                &client,
                q,
                tl::enums::MessagesFilter::InputMessagesFilterVideo,
                lim,
                fix_extensions
            ),
            fetch_global_search_filter(
                &client,
                q,
                tl::enums::MessagesFilter::InputMessagesFilterPhotos,
                lim,
                fix_extensions
            ),
            fetch_global_search_filter(
                &client,
                q,
                tl::enums::MessagesFilter::InputMessagesFilterMusic,
                lim,
                fix_extensions
            ),
            fetch_global_search_filter(
                &client,
                q,
                tl::enums::MessagesFilter::InputMessagesFilterEmpty,
                lim,
                fix_extensions
            ),
        );

        let mut all_hits = Vec::new();
        let mut all_hashes = std::collections::HashMap::new();
        for res in [docs, videos, photos, audio, general].into_iter().flatten() {
            all_hits.extend(res.0);
            all_hashes.extend(res.1);
        }

        let mut seen = std::collections::HashSet::new();
        all_hits.retain(|hit| seen.insert((hit.chat_id, hit.message_id)));
        all_hits.sort_by_key(|b| std::cmp::Reverse(b.date));
        all_hits.truncate(limit as usize);

        Ok::<_, anyhow::Error>((all_hits, all_hashes))
    };

    let (hits, hashes) = tokio::time::timeout(Duration::from_secs(30), fetch)
        .await
        .map_err(|_| anyhow::anyhow!("Global search timed out — please try again"))??;

    if !hashes.is_empty() {
        let mut guard = handle.lock().await;
        guard.peer_hashes.extend(hashes);
    }

    Ok(hits)
}

pub async fn delete_messages(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_ids: Vec<i32>,
    revoke: bool,
) -> anyhow::Result<usize> {
    if message_ids.is_empty() {
        return Ok(0);
    }
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let count = message_ids.len();
    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel {
        let req = tl::functions::channels::DeleteMessages {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
            id: message_ids,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.deleteMessages failed: {}", e))?;
    } else {
        let req = tl::functions::messages::DeleteMessages {
            revoke,
            id: message_ids,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.deleteMessages failed: {}", e))?;
    }
    Ok(count)
}

fn next_random_id(seed: u64) -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    (nanos.wrapping_mul(6364136223846793005).wrapping_add(seed)) as i64
}

#[allow(clippy::too_many_arguments)]
pub async fn forward_messages(
    handle: &TelegramSessionHandle,
    from_chat_id: i64,
    from_chat_type: &str,
    to_chat_id: i64,
    to_chat_type: &str,
    message_ids: Vec<i32>,
    drop_author: bool,
    drop_captions: bool,
) -> anyhow::Result<()> {
    if message_ids.is_empty() {
        return Ok(());
    }
    let (client, from_hash, to_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let fh = guard.peer_hashes.get(&from_chat_id).copied().unwrap_or(0);
        let th = guard.peer_hashes.get(&to_chat_id).copied().unwrap_or(0);
        (client, fh, th)
    };

    let from_peer = make_input_peer(from_chat_id, from_chat_type, from_hash);
    let to_peer = make_input_peer(to_chat_id, to_chat_type, to_hash);
    let random_id: Vec<i64> = message_ids
        .iter()
        .enumerate()
        .map(|(idx, &mid)| next_random_id(((idx as u64) << 32) ^ (mid as u32 as u64)))
        .collect();

    let req = tl::functions::messages::ForwardMessages {
        silent: false,
        background: false,
        with_my_score: false,
        drop_author,
        drop_media_captions: drop_captions,
        noforwards: false,
        allow_paid_floodskip: false,
        from_peer,
        id: message_ids,
        random_id,
        to_peer,
        top_msg_id: None,
        reply_to: None,
        schedule_date: None,
        send_as: None,
        quick_reply_shortcut: None,
        video_timestamp: None,
        allow_paid_stars: None,
        suggested_post: None,
    };

    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("messages.forwardMessages failed: {}", e))?;
    Ok(())
}

pub async fn edit_caption(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_id: i32,
    caption: &str,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let peer = make_input_peer(chat_id, chat_type, access_hash);
    let req = tl::functions::messages::EditMessage {
        no_webpage: false,
        invert_media: false,
        peer,
        id: message_id,
        message: Some(caption.to_string()),
        media: None,
        reply_markup: None,
        entities: None,
        schedule_date: None,
        quick_reply_shortcut_id: None,
    };

    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("messages.editMessage failed: {}", e))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct TelegramFullChannelInfo {
    pub chat_id: i64,
    pub title: String,
    pub about: String,
    pub participants_count: Option<i32>,
    pub admins_count: Option<i32>,
    pub kicked_count: Option<i32>,
    pub banned_count: Option<i32>,
    pub online_count: Option<i32>,
    pub username: Option<String>,
    pub invite_link: Option<String>,
    pub slowmode_seconds: Option<i32>,
    pub linked_chat_id: Option<i64>,
    pub can_view_participants: bool,
    pub is_megagroup: bool,
    pub is_broadcast: bool,
}

pub async fn get_full_channel_info(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
) -> anyhow::Result<TelegramFullChannelInfo> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel {
        let req = tl::functions::channels::GetFullChannel {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
        };
        let res = invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.getFullChannel failed: {}", e))?;
        let tl::enums::messages::ChatFull::Full(full) = res;

        let mut title = String::new();
        let mut username = None;
        let mut is_megagroup = chat_type == "group";
        let mut is_broadcast = chat_type == "channel";
        let mut ch_participants = None;

        for c in &full.chats {
            if let tl::enums::Chat::Channel(ch) = c {
                if ch.id == chat_id {
                    title = ch.title.clone();
                    username = ch.username.clone();
                    is_megagroup = ch.megagroup;
                    is_broadcast = ch.broadcast;
                    ch_participants = ch.participants_count;
                    break;
                }
            }
        }

        match full.full_chat {
            tl::enums::ChatFull::ChannelFull(cf) => {
                let invite_link = match cf.exported_invite {
                    Some(tl::enums::ExportedChatInvite::ChatInviteExported(inv)) => Some(inv.link),
                    _ => None,
                };
                return Ok(TelegramFullChannelInfo {
                    chat_id,
                    title,
                    about: cf.about,
                    participants_count: cf.participants_count.or(ch_participants),
                    admins_count: cf.admins_count,
                    kicked_count: cf.kicked_count,
                    banned_count: cf.banned_count,
                    online_count: cf.online_count,
                    username,
                    invite_link,
                    slowmode_seconds: cf.slowmode_seconds,
                    linked_chat_id: cf.linked_chat_id,
                    can_view_participants: cf.can_view_participants,
                    is_megagroup,
                    is_broadcast,
                });
            }
            tl::enums::ChatFull::Full(cf) => {
                return Ok(TelegramFullChannelInfo {
                    chat_id,
                    title,
                    about: cf.about,
                    participants_count: ch_participants,
                    admins_count: None,
                    kicked_count: None,
                    banned_count: None,
                    online_count: None,
                    username,
                    invite_link: None,
                    slowmode_seconds: None,
                    linked_chat_id: None,
                    can_view_participants: true,
                    is_megagroup,
                    is_broadcast,
                });
            }
        }
    }

    if chat_type == "group" {
        let req = tl::functions::messages::GetFullChat { chat_id };
        let res = invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.getFullChat failed: {}", e))?;
        let tl::enums::messages::ChatFull::Full(full) = res;
        let mut title = String::new();
        let mut participants_count = None;
        for c in &full.chats {
            if let tl::enums::Chat::Chat(chat) = c {
                if chat.id == chat_id {
                    title = chat.title.clone();
                    participants_count = Some(chat.participants_count);
                    break;
                }
            }
        }
        let (about, invite_link) = match full.full_chat {
            tl::enums::ChatFull::Full(cf) => {
                let link = match cf.exported_invite {
                    Some(tl::enums::ExportedChatInvite::ChatInviteExported(inv)) => Some(inv.link),
                    _ => None,
                };
                (cf.about, link)
            }
            tl::enums::ChatFull::ChannelFull(cf) => (cf.about, None),
        };
        return Ok(TelegramFullChannelInfo {
            chat_id,
            title,
            about,
            participants_count,
            admins_count: None,
            kicked_count: None,
            banned_count: None,
            online_count: None,
            username: None,
            invite_link,
            slowmode_seconds: None,
            linked_chat_id: None,
            can_view_participants: true,
            is_megagroup: false,
            is_broadcast: false,
        });
    }

    let req = tl::functions::users::GetUsers {
        id: vec![tl::enums::InputUser::User(tl::types::InputUser {
            user_id: chat_id,
            access_hash,
        })],
    };
    let users = invoke_with_flood_wait(&client, &req)
        .await
        .unwrap_or_default();
    let (title, username) = match users.into_iter().next() {
        Some(tl::enums::User::User(u)) => {
            let name = match (&u.first_name, &u.last_name) {
                (Some(f), Some(l)) if !l.is_empty() => format!("{} {}", f, l),
                (Some(f), _) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => u.username.clone().unwrap_or_default(),
            };
            (name, u.username)
        }
        _ => (String::new(), None),
    };

    Ok(TelegramFullChannelInfo {
        chat_id,
        title,
        about: String::new(),
        participants_count: None,
        admins_count: None,
        kicked_count: None,
        banned_count: None,
        online_count: None,
        username,
        invite_link: None,
        slowmode_seconds: None,
        linked_chat_id: None,
        can_view_participants: false,
        is_megagroup: false,
        is_broadcast: false,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct TelegramParticipant {
    pub user_id: i64,
    pub role: String,
    pub rank: Option<String>,
    pub joined_date: Option<i32>,
    pub promoted_by: Option<i64>,
    pub can_edit: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub phone: Option<String>,
    pub is_bot: bool,
    pub is_scam: bool,
    pub is_fake: bool,
    pub is_premium: bool,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TelegramParticipantsPage {
    pub count: i32,
    pub users: Vec<TelegramParticipant>,
}

pub async fn list_participants(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    filter_name: &str,
    offset: i32,
    limit: i32,
    search: Option<&str>,
) -> anyhow::Result<TelegramParticipantsPage> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let q = search.unwrap_or("").to_string();
    let filter = match filter_name {
        "admins" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsAdmins,
        "bots" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsBots,
        "kicked" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsKicked(
            tl::types::ChannelParticipantsKicked { q },
        ),
        "banned" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsBanned(
            tl::types::ChannelParticipantsBanned { q },
        ),
        "contacts" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsContacts(
            tl::types::ChannelParticipantsContacts { q },
        ),
        "search" => tl::enums::ChannelParticipantsFilter::ChannelParticipantsSearch(
            tl::types::ChannelParticipantsSearch { q },
        ),
        _ => tl::enums::ChannelParticipantsFilter::ChannelParticipantsRecent,
    };

    let req = tl::functions::channels::GetParticipants {
        channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
            channel_id: chat_id,
            access_hash,
        }),
        filter,
        offset,
        limit: limit.clamp(1, 200),
        hash: 0,
    };

    let res = invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("channels.getParticipants failed: {}", e))?;

    match res {
        tl::enums::channels::ChannelParticipants::NotModified => Ok(TelegramParticipantsPage {
            count: 0,
            users: Vec::new(),
        }),
        tl::enums::channels::ChannelParticipants::Participants(p) => {
            let mut user_map = std::collections::HashMap::new();
            for u in p.users {
                if let tl::enums::User::User(user) = u {
                    user_map.insert(user.id, user);
                }
            }

            let mut out = Vec::new();
            for part in p.participants {
                let (uid, role, rank, joined_date, promoted_by, can_edit) = match part {
                    tl::enums::ChannelParticipant::Creator(c) => {
                        (c.user_id, "creator", c.rank, None, None, false)
                    }
                    tl::enums::ChannelParticipant::Admin(a) => (
                        a.user_id,
                        "admin",
                        a.rank,
                        Some(a.date),
                        Some(a.promoted_by),
                        a.can_edit,
                    ),
                    tl::enums::ChannelParticipant::Participant(m) => {
                        (m.user_id, "member", None, Some(m.date), None, false)
                    }
                    tl::enums::ChannelParticipant::ParticipantSelf(m) => (
                        m.user_id,
                        "member",
                        None,
                        Some(m.date),
                        Some(m.inviter_id),
                        false,
                    ),
                    tl::enums::ChannelParticipant::Banned(b) => {
                        let uid = match b.peer {
                            tl::enums::Peer::User(u) => u.user_id,
                            tl::enums::Peer::Chat(c) => c.chat_id,
                            tl::enums::Peer::Channel(ch) => ch.channel_id,
                        };
                        (uid, "banned", None, Some(b.date), Some(b.kicked_by), false)
                    }
                    tl::enums::ChannelParticipant::Left(l) => {
                        let uid = match l.peer {
                            tl::enums::Peer::User(u) => u.user_id,
                            tl::enums::Peer::Chat(c) => c.chat_id,
                            tl::enums::Peer::Channel(ch) => ch.channel_id,
                        };
                        (uid, "left", None, None, None, false)
                    }
                };

                let u = user_map.get(&uid);
                out.push(TelegramParticipant {
                    user_id: uid,
                    role: role.to_string(),
                    rank,
                    joined_date,
                    promoted_by,
                    can_edit,
                    first_name: u.and_then(|u| u.first_name.clone()).unwrap_or_default(),
                    last_name: u.and_then(|u| u.last_name.clone()),
                    username: u.and_then(|u| u.username.clone()),
                    phone: u.and_then(|u| u.phone.clone()),
                    is_bot: u.map(|u| u.bot).unwrap_or(false),
                    is_scam: u.map(|u| u.scam).unwrap_or(false),
                    is_fake: u.map(|u| u.fake).unwrap_or(false),
                    is_premium: u.map(|u| u.premium).unwrap_or(false),
                    is_verified: u.map(|u| u.verified).unwrap_or(false),
                });
            }

            Ok(TelegramParticipantsPage {
                count: p.count,
                users: out,
            })
        }
    }
}

fn extract_channel_from_updates(updates: tl::enums::Updates) -> Option<tl::types::Channel> {
    let chats = match updates {
        tl::enums::Updates::Updates(u) => u.chats,
        tl::enums::Updates::Combined(u) => u.chats,
        _ => Vec::new(),
    };
    for c in chats {
        if let tl::enums::Chat::Channel(ch) = c {
            return Some(ch);
        }
    }
    None
}

pub async fn create_channel_or_folder(
    handle: &TelegramSessionHandle,
    title: &str,
    about: &str,
    megagroup: bool,
) -> anyhow::Result<TelegramChat> {
    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };

    let req = tl::functions::channels::CreateChannel {
        broadcast: !megagroup,
        megagroup,
        for_import: false,
        forum: false,
        title: title.to_string(),
        about: about.to_string(),
        geo_point: None,
        address: None,
        ttl_period: None,
    };

    let updates = invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("channels.createChannel failed: {}", e))?;

    let ch = extract_channel_from_updates(updates)
        .ok_or_else(|| anyhow::anyhow!("Channel created but missing from Updates response"))?;

    let hash = ch.access_hash.unwrap_or(0);
    {
        let mut guard = handle.lock().await;
        guard.peer_hashes.insert(ch.id, hash);
    }

    Ok(TelegramChat {
        id: ch.id,
        title: ch.title,
        chat_type: if ch.megagroup {
            "group".to_string()
        } else {
            "channel".to_string()
        },
        peer_hash: Some(hash),
    })
}

pub async fn rename_channel(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    new_title: &str,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel {
        let req = tl::functions::channels::EditTitle {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
            title: new_title.to_string(),
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.editTitle failed: {}", e))?;
    } else {
        let req = tl::functions::messages::EditChatTitle {
            chat_id,
            title: new_title.to_string(),
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.editChatTitle failed: {}", e))?;
    }
    Ok(())
}

pub async fn delete_channel(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel {
        let req = tl::functions::channels::DeleteChannel {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.deleteChannel failed: {}", e))?;
    } else {
        let peer = make_input_peer(chat_id, chat_type, access_hash);
        let req = tl::functions::messages::DeleteHistory {
            just_clear: false,
            revoke: true,
            peer,
            max_id: i32::MAX,
            min_date: None,
            max_date: None,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.deleteHistory failed: {}", e))?;
    }
    Ok(())
}

pub async fn leave_channel(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel {
        let req = tl::functions::channels::LeaveChannel {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.leaveChannel failed: {}", e))?;
    } else if chat_type == "group" {
        let req = tl::functions::messages::DeleteChatUser {
            revoke_history: false,
            chat_id,
            user_id: tl::enums::InputUser::UserSelf,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.deleteChatUser failed: {}", e))?;
    }
    Ok(())
}

pub async fn delete_history(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    revoke: bool,
    just_clear: bool,
) -> anyhow::Result<i32> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    if is_channel && !just_clear {
        let req = tl::functions::channels::DeleteHistory {
            for_everyone: revoke,
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
            max_id: i32::MAX,
        };
        let _ = invoke_with_flood_wait(&client, &req).await;
        return Ok(1);
    }

    let peer = make_input_peer(chat_id, chat_type, access_hash);
    let mut batches = 0;
    loop {
        let req = tl::functions::messages::DeleteHistory {
            just_clear,
            revoke,
            peer: peer.clone(),
            max_id: i32::MAX,
            min_date: None,
            max_date: None,
        };
        let res = invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.deleteHistory failed: {}", e))?;
        let tl::enums::messages::AffectedHistory::History(h) = res;
        batches += 1;
        if h.offset <= 0 || batches >= 20 {
            break;
        }
    }
    Ok(batches)
}

pub async fn set_blocked(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    blocked: bool,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };
    let id = make_input_peer(chat_id, chat_type, access_hash);
    if blocked {
        let req = tl::functions::contacts::Block {
            my_stories_from: false,
            id,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("contacts.block failed: {}", e))?;
    } else {
        let req = tl::functions::contacts::Unblock {
            my_stories_from: false,
            id,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("contacts.unblock failed: {}", e))?;
    }
    Ok(())
}

pub async fn report_peer(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_ids: Vec<i32>,
    option: Vec<u8>,
    message: &str,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };
    let peer = make_input_peer(chat_id, chat_type, access_hash);
    if message_ids.is_empty() {
        let req = tl::functions::messages::ReportSpam { peer };
        let _ = invoke_with_flood_wait(&client, &req).await;
    } else {
        let req = tl::functions::messages::Report {
            peer,
            id: message_ids,
            option,
            message: message.to_string(),
        };
        let _ = invoke_with_flood_wait(&client, &req).await;
    }
    Ok(())
}

pub async fn set_mute(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    muted: bool,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };
    let peer = make_input_peer(chat_id, chat_type, access_hash);
    let req = tl::functions::account::UpdateNotifySettings {
        peer: tl::enums::InputNotifyPeer::Peer(tl::types::InputNotifyPeer { peer }),
        settings: tl::enums::InputPeerNotifySettings::Settings(
            tl::types::InputPeerNotifySettings {
                show_previews: Some(true),
                silent: Some(muted),
                mute_until: Some(if muted { i32::MAX } else { 0 }),
                sound: None,
                stories_muted: None,
                stories_hide_sender: None,
                stories_sound: None,
            },
        ),
    };
    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("account.updateNotifySettings failed: {}", e))?;
    Ok(())
}

pub async fn toggle_pin(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    pinned: bool,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };
    let peer = make_input_peer(chat_id, chat_type, access_hash);
    let req = tl::functions::messages::ToggleDialogPin {
        pinned,
        peer: tl::enums::InputDialogPeer::Peer(tl::types::InputDialogPeer { peer }),
    };
    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("messages.toggleDialogPin failed: {}", e))?;
    Ok(())
}

pub async fn set_archived(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    archived: bool,
) -> anyhow::Result<()> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };
    let peer = make_input_peer(chat_id, chat_type, access_hash);
    let req = tl::functions::folders::EditPeerFolders {
        folder_peers: vec![tl::enums::InputFolderPeer::Peer(
            tl::types::InputFolderPeer {
                peer,
                folder_id: if archived { 1 } else { 0 },
            },
        )],
    };
    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("folders.editPeerFolders failed: {}", e))?;
    Ok(())
}

pub async fn reorder_pinned(
    handle: &TelegramSessionHandle,
    folder_id: i32,
    peers: Vec<(i64, String)>,
) -> anyhow::Result<()> {
    let (client, hashes) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        (client, guard.peer_hashes.clone())
    };
    let order = peers
        .into_iter()
        .map(|(cid, ctype)| {
            let hash = hashes.get(&cid).copied().unwrap_or(0);
            let peer = make_input_peer(cid, &ctype, hash);
            tl::enums::InputDialogPeer::Peer(tl::types::InputDialogPeer { peer })
        })
        .collect();
    let req = tl::functions::messages::ReorderPinnedDialogs {
        force: true,
        folder_id,
        order,
    };
    invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("messages.reorderPinnedDialogs failed: {}", e))?;
    Ok(())
}

pub async fn takeout_start(handle: &TelegramSessionHandle) -> anyhow::Result<i64> {
    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };
    let req = tl::functions::account::InitTakeoutSession {
        contacts: false,
        message_users: true,
        message_chats: true,
        message_megagroups: true,
        message_channels: true,
        files: true,
        file_max_size: Some(4 * 1024 * 1024 * 1024),
    };
    match invoke_with_flood_wait(&client, &req).await {
        Ok(tl::enums::account::Takeout::Takeout(t)) => Ok(t.id),
        Err(e) => Err(anyhow::anyhow!("account.initTakeoutSession failed: {}", e)),
    }
}

pub async fn takeout_finish(handle: &TelegramSessionHandle, success: bool) -> anyhow::Result<()> {
    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };
    let req = tl::functions::account::FinishTakeoutSession { success };
    let _ = invoke_with_flood_wait(&client, &req).await;
    Ok(())
}

pub async fn expand_album(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
    message_id: i32,
    fix_extensions: bool,
) -> anyhow::Result<Vec<TelegramMediaItem>> {
    let (client, access_hash) = {
        let guard = handle.lock().await;
        let client = guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone();
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let start_id = (message_id - 10).max(1);
    let end_id = message_id + 10;
    let ids: Vec<tl::enums::InputMessage> = (start_id..=end_id)
        .map(|id| tl::enums::InputMessage::Id(tl::types::InputMessageId { id }))
        .collect();

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    let res = if is_channel {
        let req = tl::functions::channels::GetMessages {
            channel: tl::enums::InputChannel::Channel(tl::types::InputChannel {
                channel_id: chat_id,
                access_hash,
            }),
            id: ids,
        };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("channels.getMessages failed: {}", e))?
    } else {
        let req = tl::functions::messages::GetMessages { id: ids };
        invoke_with_flood_wait(&client, &req)
            .await
            .map_err(|e| anyhow::anyhow!("messages.getMessages failed: {}", e))?
    };

    let raw_messages = match res {
        tl::enums::messages::Messages::Messages(m) => m.messages,
        tl::enums::messages::Messages::Slice(m) => m.messages,
        tl::enums::messages::Messages::ChannelMessages(m) => m.messages,
        tl::enums::messages::Messages::NotModified(_) => Vec::new(),
    };

    let mut parsed: Vec<(i32, Option<i64>, TelegramMediaItem)> = Vec::new();
    let mut target_group: Option<i64> = None;

    for raw_msg in raw_messages {
        let msg = match raw_msg {
            tl::enums::Message::Message(m) => m,
            _ => continue,
        };
        if msg.id == message_id {
            target_group = msg.grouped_id;
        }
        let caption = if msg.message.trim().is_empty() {
            None
        } else {
            Some(msg.message.clone())
        };
        if let Some(ref media) = msg.media {
            if let Some((file_name, file_size, media_type, webpage)) =
                extract_raw_media_info_with_webpage(media, fix_extensions)
            {
                parsed.push((
                    msg.id,
                    msg.grouped_id,
                    TelegramMediaItem {
                        message_id: msg.id,
                        file_name,
                        file_size,
                        media_type,
                        date: msg.date as i64,
                        webpage,
                        caption,
                        grouped_id: msg.grouped_id,
                    },
                ));
            }
        }
    }

    let mut out: Vec<TelegramMediaItem> = match target_group {
        Some(gid) => parsed
            .into_iter()
            .filter(|(_, g, _)| *g == Some(gid))
            .map(|(_, _, item)| item)
            .collect(),
        None => parsed
            .into_iter()
            .filter(|(id, _, _)| *id == message_id)
            .map(|(_, _, item)| item)
            .collect(),
    };
    out.sort_by_key(|item| item.message_id);
    Ok(out)
}

pub async fn diag_list_media(
    handle: &TelegramSessionHandle,
    chat_id: i64,
    chat_type: &str,
) -> anyhow::Result<serde_json::Value> {
    let (client_opt, access_hash, cached_peers) = {
        let guard = handle.lock().await;
        (
            guard.client.clone(),
            guard.peer_hashes.get(&chat_id).copied(),
            guard.peer_hashes.len(),
        )
    };

    let client = match client_opt {
        Some(c) => c,
        None => {
            return Ok(serde_json::json!({
                "chat_id": chat_id,
                "chat_type": chat_type,
                "has_client": false,
                "is_authorized": false,
                "access_hash_cached": access_hash.is_some(),
                "access_hash_nonzero": access_hash.unwrap_or(0) != 0,
                "cached_peers_total": cached_peers,
                "input_peer_kind": "none",
                "search_probes": [],
                "history_probe": { "ok": false, "error": "Not authenticated" }
            }));
        }
    };

    let is_auth = client.is_authorized().await.unwrap_or(false);
    let hash = access_hash.unwrap_or(0);
    let peer = make_input_peer(chat_id, chat_type, hash);
    let peer_kind = match &peer {
        tl::enums::InputPeer::User(_) => "user",
        tl::enums::InputPeer::Chat(_) => "chat",
        tl::enums::InputPeer::Channel(_) => "channel",
        _ => "empty",
    };

    let mut search_probes = Vec::new();
    for filter_name in ["photo", "video", "document", "audio"] {
        let t0 = std::time::Instant::now();
        match fetch_media_page(&client, &peer, Some(filter_name), "", 0, 10, true).await {
            Ok(items) => search_probes.push(serde_json::json!({
                "filter": filter_name,
                "ok": true,
                "elapsed_ms": t0.elapsed().as_millis() as u64,
                "raw_messages": items.len(),
                "extracted_media": items.len(),
            })),
            Err(e) => search_probes.push(serde_json::json!({
                "filter": filter_name,
                "ok": false,
                "elapsed_ms": t0.elapsed().as_millis() as u64,
                "error": e.to_string(),
            })),
        }
    }

    let t_hist = std::time::Instant::now();
    let hist_req = tl::functions::messages::GetHistory {
        peer,
        offset_id: 0,
        offset_date: 0,
        add_offset: 0,
        limit: 20,
        max_id: 0,
        min_id: 0,
        hash: 0,
    };
    let history_probe = match invoke_with_flood_wait(&client, &hist_req).await {
        Ok(res) => {
            let msgs = match res {
                tl::enums::messages::Messages::Messages(m) => m.messages,
                tl::enums::messages::Messages::Slice(m) => m.messages,
                tl::enums::messages::Messages::ChannelMessages(m) => m.messages,
                tl::enums::messages::Messages::NotModified(_) => Vec::new(),
            };
            let mut with_media = 0usize;
            for m in &msgs {
                if let tl::enums::Message::Message(msg) = m {
                    if msg.media.is_some() {
                        with_media += 1;
                    }
                }
            }
            serde_json::json!({
                "ok": true,
                "elapsed_ms": t_hist.elapsed().as_millis() as u64,
                "raw_messages": msgs.len(),
                "with_media": with_media,
                "media_kinds": {},
            })
        }
        Err(e) => serde_json::json!({
            "ok": false,
            "elapsed_ms": t_hist.elapsed().as_millis() as u64,
            "error": e.to_string(),
        }),
    };

    Ok(serde_json::json!({
        "chat_id": chat_id,
        "chat_type": chat_type,
        "has_client": true,
        "is_authorized": is_auth,
        "access_hash_cached": access_hash.is_some(),
        "access_hash_nonzero": hash != 0,
        "cached_peers_total": cached_peers,
        "input_peer_kind": peer_kind,
        "search_probes": search_probes,
        "history_probe": history_probe,
    }))
}

pub fn guess_mime_from_filename(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "opus" => "audio/x-opus+ogg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

pub async fn upload_file_to_peer(
    handle: &TelegramSessionHandle,
    peer: tl::enums::InputPeer,
    file_path: &Path,
    caption: &str,
    progress_tx: mpsc::Sender<(u64, u64)>,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    use tokio::io::AsyncReadExt;

    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };

    let meta = tokio::fs::metadata(file_path).await?;
    let total_bytes = meta.len();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.bin")
        .to_string();

    let chunk_size: usize = 512 * 1024;
    let total_parts = total_bytes.div_ceil(chunk_size as u64).max(1) as i32;
    let file_id = next_random_id(total_bytes);
    let is_big = total_bytes > 10 * 1024 * 1024;

    let mut file = tokio::fs::File::open(file_path).await?;
    let mut uploaded: u64 = 0;
    let mut part_idx: i32 = 0;

    loop {
        if cancel_token.is_cancelled() {
            return Err(anyhow::anyhow!("Upload cancelled"));
        }

        let mut buf = vec![0u8; chunk_size];
        let mut read_in_part = 0usize;
        while read_in_part < chunk_size {
            let n = file.read(&mut buf[read_in_part..]).await?;
            if n == 0 {
                break;
            }
            read_in_part += n;
        }

        if read_in_part == 0 && part_idx > 0 {
            break;
        }
        buf.truncate(read_in_part);

        if is_big {
            let req = tl::functions::upload::SaveBigFilePart {
                file_id,
                file_part: part_idx,
                file_total_parts: total_parts,
                bytes: buf,
            };
            let ok = invoke_with_flood_wait(&client, &req)
                .await
                .map_err(|e| anyhow::anyhow!("upload.saveBigFilePart failed: {}", e))?;
            if !ok {
                return Err(anyhow::anyhow!("Telegram rejected uploaded file part"));
            }
        } else {
            let req = tl::functions::upload::SaveFilePart {
                file_id,
                file_part: part_idx,
                bytes: buf,
            };
            let ok = invoke_with_flood_wait(&client, &req)
                .await
                .map_err(|e| anyhow::anyhow!("upload.saveFilePart failed: {}", e))?;
            if !ok {
                return Err(anyhow::anyhow!("Telegram rejected uploaded file part"));
            }
        }

        uploaded += read_in_part as u64;
        part_idx += 1;
        let _ = progress_tx.send((uploaded, total_bytes)).await;

        if read_in_part < chunk_size || uploaded >= total_bytes {
            break;
        }
    }

    let input_file = if is_big {
        tl::enums::InputFile::Big(tl::types::InputFileBig {
            id: file_id,
            parts: part_idx.max(1),
            name: file_name.clone(),
        })
    } else {
        tl::enums::InputFile::File(tl::types::InputFile {
            id: file_id,
            parts: part_idx.max(1),
            name: file_name.clone(),
            md5_checksum: String::new(),
        })
    };

    let mime_type = guess_mime_from_filename(&file_name).to_string();
    let media = tl::enums::InputMedia::UploadedDocument(tl::types::InputMediaUploadedDocument {
        nosound_video: false,
        force_file: false,
        spoiler: false,
        file: input_file,
        thumb: None,
        mime_type,
        attributes: vec![tl::enums::DocumentAttribute::Filename(
            tl::types::DocumentAttributeFilename { file_name },
        )],
        stickers: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    });

    let send_req = tl::functions::messages::SendMedia {
        silent: false,
        background: false,
        clear_draft: false,
        noforwards: false,
        update_stickersets_order: false,
        invert_media: false,
        allow_paid_floodskip: false,
        peer,
        reply_to: None,
        media,
        message: caption.to_string(),
        random_id: next_random_id(file_id as u64),
        reply_markup: None,
        entities: None,
        schedule_date: None,
        send_as: None,
        quick_reply_shortcut: None,
        effect: None,
        allow_paid_stars: None,
        suggested_post: None,
    };

    invoke_with_flood_wait(&client, &send_req)
        .await
        .map_err(|e| anyhow::anyhow!("messages.sendMedia failed: {}", e))?;

    Ok(total_bytes)
}

pub async fn resolve_destination_peer(
    handle: &TelegramSessionHandle,
    destination: &str,
) -> anyhow::Result<tl::enums::InputPeer> {
    let dest = destination.trim();
    if dest.is_empty() || dest.eq_ignore_ascii_case("me") || dest.eq_ignore_ascii_case("saved") {
        return Ok(tl::enums::InputPeer::PeerSelf);
    }

    if let Ok(id) = dest.parse::<i64>() {
        let bare = if id < -1000000000000 {
            (-id) - 1000000000000
        } else {
            id.abs()
        };
        let hash = {
            let guard = handle.lock().await;
            guard.peer_hashes.get(&bare).copied().unwrap_or(0)
        };
        if id < -1000000000000 || hash != 0 {
            return Ok(tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
                channel_id: bare,
                access_hash: hash,
            }));
        }
        return Ok(tl::enums::InputPeer::Chat(tl::types::InputPeerChat {
            chat_id: bare,
        }));
    }

    let username = dest
        .trim_start_matches("https://t.me/")
        .trim_start_matches("http://t.me/")
        .trim_start_matches("t.me/")
        .trim_start_matches('@')
        .split('/')
        .next()
        .unwrap_or("")
        .trim();

    let client = {
        let guard = handle.lock().await;
        guard
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?
            .clone()
    };

    let req = tl::functions::contacts::ResolveUsername {
        username: username.to_string(),
        referer: None,
    };
    let resolved = invoke_with_flood_wait(&client, &req)
        .await
        .map_err(|e| anyhow::anyhow!("contacts.resolveUsername failed: {}", e))?;
    let tl::enums::contacts::ResolvedPeer::Peer(r) = resolved;

    for c in r.chats {
        match c {
            tl::enums::Chat::Channel(ch) => {
                let hash = ch.access_hash.unwrap_or(0);
                handle.lock().await.peer_hashes.insert(ch.id, hash);
                return Ok(tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
                    channel_id: ch.id,
                    access_hash: hash,
                }));
            }
            tl::enums::Chat::Chat(chat) => {
                return Ok(tl::enums::InputPeer::Chat(tl::types::InputPeerChat {
                    chat_id: chat.id,
                }));
            }
            _ => {}
        }
    }
    for u in r.users {
        if let tl::enums::User::User(user) = u {
            let hash = user.access_hash.unwrap_or(0);
            handle.lock().await.peer_hashes.insert(user.id, hash);
            return Ok(tl::enums::InputPeer::User(tl::types::InputPeerUser {
                user_id: user.id,
                access_hash: hash,
            }));
        }
    }

    Err(anyhow::anyhow!(
        "Could not resolve destination: {}",
        destination
    ))
}

pub async fn ensure_stream_server(
    session: TelegramSessionHandle,
    slot: std::sync::Arc<tokio::sync::Mutex<Option<crate::state::TelegramStreamInfo>>>,
) -> anyhow::Result<crate::state::TelegramStreamInfo> {
    let mut guard = slot.lock().await;
    if let Some(ref info) = *guard {
        return Ok(info.clone());
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let token = format!("{:016x}", next_random_id(port as u64) as u64);
    let info = crate::state::TelegramStreamInfo {
        port,
        token: token.clone(),
        base_url: format!("http://127.0.0.1:{}", port),
    };
    *guard = Some(info.clone());
    drop(guard);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let session_clone = session.clone();
            let token_clone = token.clone();
            tokio::spawn(async move {
                let _ = handle_stream_connection(stream, session_clone, &token_clone).await;
            });
        }
    });

    Ok(info)
}

async fn handle_stream_connection(
    mut socket: tokio::net::TcpStream,
    session: TelegramSessionHandle,
    expected_token: &str,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut req_buf = vec![0u8; 8192];
    let n = socket.read(&mut req_buf).await?;
    if n == 0 {
        return Ok(());
    }
    let req_str = String::from_utf8_lossy(&req_buf[..n]);
    let mut lines = req_str.lines();
    let first_line = lines.next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let raw_target = parts.next().unwrap_or("/");

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range\r\n\r\n";
        socket.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let (path, query) = raw_target.split_once('?').unwrap_or((raw_target, ""));
    let token_ok = query.split('&').any(|kv| {
        if let Some((k, v)) = kv.split_once('=') {
            k == "token" && v == expected_token
        } else {
            false
        }
    });

    if !token_ok {
        let resp = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
        socket.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    if segs.len() < 4 || segs[0] != "stream" {
        let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        socket.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let chat_type = segs[1];
    let chat_id: i64 = segs[2].parse().unwrap_or(0);
    let message_id: i32 = segs[3].parse().unwrap_or(0);

    let mut range_header: Option<(u64, Option<u64>)> = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("range") {
                let val = v.trim();
                if let Some(bytes_spec) = val.strip_prefix("bytes=") {
                    if let Some((start_s, end_s)) = bytes_spec.split_once('-') {
                        if let Ok(start) = start_s.trim().parse::<u64>() {
                            let end = end_s.trim().parse::<u64>().ok();
                            range_header = Some((start, end));
                        }
                    }
                }
            }
        }
    }

    let (client, access_hash) = {
        let guard = session.lock().await;
        let client = match guard.client.as_ref() {
            Some(c) => c.clone(),
            None => {
                let resp = "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";
                socket.write_all(resp.as_bytes()).await?;
                return Ok(());
            }
        };
        let hash = guard.peer_hashes.get(&chat_id).copied().unwrap_or(0);
        (client, hash)
    };

    let is_channel = chat_type == "channel" || (chat_type == "group" && access_hash != 0);
    let (raw_media, _) = match super::parallel_download::fetch_raw_media(
        &client,
        chat_id,
        access_hash,
        is_channel,
        message_id,
    )
    .await
    {
        Ok(m) => m,
        Err(_) => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            socket.write_all(resp.as_bytes()).await?;
            return Ok(());
        }
    };

    let media = match super::parallel_download::media_to_location(&raw_media) {
        Some(m) => m,
        None => {
            let resp = "HTTP/1.1 415 Unsupported Media Type\r\nContent-Length: 0\r\n\r\n";
            socket.write_all(resp.as_bytes()).await?;
            return Ok(());
        }
    };

    let total_size = media.size;
    if total_size == 0 {
        let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        socket.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let (start, end, is_partial) = match range_header {
        Some((s, e_opt)) => {
            let s = s.min(total_size.saturating_sub(1));
            let e = e_opt
                .unwrap_or(total_size - 1)
                .min(total_size.saturating_sub(1))
                .max(s);
            (s, e, true)
        }
        None => (0, total_size - 1, false),
    };

    let content_length = end - start + 1;
    let headers = if is_partial {
        format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
            media.mime_type, start, end, total_size, content_length
        )
    } else {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
            media.mime_type, content_length
        )
    };

    socket.write_all(headers.as_bytes()).await?;
    if method == "HEAD" {
        return Ok(());
    }

    let chunk_limit: u64 = 512 * 1024;
    let mut cursor = start;
    while cursor <= end {
        let aligned_offset = (cursor / 4096) * 4096;
        let skip = (cursor - aligned_offset) as usize;
        let bytes = super::parallel_download::fetch_media_chunk(
            &client,
            &media,
            aligned_offset as i64,
            chunk_limit as i32,
        )
        .await?;
        if bytes.len() <= skip {
            break;
        }
        let available = &bytes[skip..];
        let remaining = (end - cursor + 1) as usize;
        let to_write = available.len().min(remaining);
        if socket.write_all(&available[..to_write]).await.is_err() {
            break;
        }
        cursor += to_write as u64;
        if (bytes.len() as u64) < chunk_limit {
            break;
        }
    }

    let _ = socket.flush().await;
    Ok(())
}
