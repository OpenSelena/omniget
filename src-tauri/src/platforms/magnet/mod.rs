use omniget_core::models::progress::ProgressUpdate;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use librqbit::{api::TorrentIdOrHash, AddTorrent, AddTorrentOptions, Session, SessionOptions};
use tokio::sync::mpsc;

use crate::models::media::{DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality};
use crate::platforms::traits::PlatformDownloader;

type SharedSession = Arc<tokio::sync::Mutex<Option<Arc<Session>>>>;

/// True when the input names a local BitTorrent metainfo file rather than a URI.
///
/// A bare `Remastered.torrent` has no scheme, so handing it to the torrent backend
/// as a URL fails with "unsupported URL ... Supporting magnet:, http:, and https".
/// Detecting the `.torrent` suffix first is what keeps local files working.
fn looks_like_torrent_path(input: &str) -> bool {
    let trimmed = input.trim().trim_matches('"');
    let lower = trimmed.to_ascii_lowercase();
    !trimmed.is_empty()
        && !lower.starts_with("magnet:")
        && !trimmed.contains("://")
        && lower.ends_with(".torrent")
}

/// `Remastered.torrent` → `Remastered`, matching the suffix case-insensitively.
fn torrent_stem(name: &str) -> &str {
    const SUFFIX: &str = ".torrent";
    match name.len().checked_sub(SUFFIX.len()) {
        Some(cut) if name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(SUFFIX) => {
            &name[..cut]
        }
        _ => name,
    }
}

/// True when the input is an HTTP or HTTPS URL pointing to a .torrent file.
fn is_remote_torrent_url(url: &str) -> bool {
    let trimmed = url.trim().trim_matches('"');
    let lower = trimmed.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://")) && lower.ends_with(".torrent")
}

/// Folders searched for a bare `.torrent` filename that is not in the working directory.
fn torrent_search_dirs(output_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![output_dir.to_path_buf()];
    if let Ok(cwd) = std::env::current_dir() {
        if cwd != output_dir {
            dirs.push(cwd);
        }
    }
    if let Some(downloads) = dirs::download_dir() {
        dirs.push(downloads);
    }
    dirs
}

/// Resolve a local `.torrent` input to an existing file.
///
/// `None` means either "not a local torrent path" or "not found"; callers use
/// [`looks_like_torrent_path`] to tell those apart when reporting the error.
fn resolve_torrent_file(input: &str, output_dir: &Path) -> Option<PathBuf> {
    if !looks_like_torrent_path(input) {
        return None;
    }
    let trimmed = input.trim().trim_matches('"');
    let candidate = Path::new(trimmed);
    if candidate.is_file() {
        return Some(candidate.to_path_buf());
    }
    let file_name = candidate.file_name()?;
    torrent_search_dirs(output_dir)
        .into_iter()
        .map(|dir| dir.join(file_name))
        .find(|path| path.is_file())
}

pub struct MagnetDownloader {
    session: SharedSession,
    session_output_dir: Arc<tokio::sync::Mutex<Option<PathBuf>>>,
}

impl MagnetDownloader {
    pub fn new(session: SharedSession) -> Self {
        Self {
            session,
            session_output_dir: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl PlatformDownloader for MagnetDownloader {
    fn name(&self) -> &str {
        "magnet"
    }

    fn can_handle(&self, url: &str) -> bool {
        let trimmed = url.trim().trim_matches('"');
        trimmed.starts_with("magnet:")
            || looks_like_torrent_path(trimmed)
            || is_remote_torrent_url(trimmed)
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        let mut title = "Torrent Download".to_string();

        if url.starts_with("magnet:") {
            if let Ok(parsed_url) = url::Url::parse(url) {
                for (key, value) in parsed_url.query_pairs() {
                    if key == "dn" {
                        title = value.to_string();
                        break;
                    }
                }
            }
        } else {
            let trimmed = url.trim().trim_matches('"');
            let path = Path::new(trimmed);
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    title = stem.to_string_lossy().to_string();
                }
            } else if looks_like_torrent_path(trimmed) || is_remote_torrent_url(trimmed) {
                let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
                title = torrent_stem(name).to_string();
            }
        }

        Ok(MediaInfo {
            title,
            author: "BitTorrent".to_string(),
            platform: "magnet".to_string(),
            duration_seconds: None,
            thumbnail_url: None,
            available_qualities: vec![VideoQuality {
                label: "Original".to_string(),
                width: 0,
                height: 0,
                url: url.to_string(),
                format: "torrent".to_string(),
            }],
            media_type: MediaType::Video,
            file_size_bytes: None,
        })
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<ProgressUpdate>,
    ) -> anyhow::Result<DownloadResult> {
        let _ = progress.send(ProgressUpdate::percent(0.0)).await;

        let url = match info.available_qualities.first() {
            Some(q) => &q.url,
            None => anyhow::bail!("No URL found in MediaInfo"),
        };

        let output_dir = &opts.output_dir;

        // Get or create the shared session, recreating if output_dir changed
        let session = {
            let mut guard = self.session.lock().await;
            let mut dir_guard = self.session_output_dir.lock().await;
            let need_new_session = match (&*guard, &*dir_guard) {
                (Some(_), Some(prev_dir)) => prev_dir != output_dir,
                (None, _) => true,
                _ => true,
            };
            if need_new_session {
                if guard.is_some() {
                    tracing::info!("[magnet] output dir changed, recreating session");
                }
                let listen_port = opts.torrent_listen_port.unwrap_or(6881).min(65525);
                tracing::info!(
                    "[magnet] creating shared session, port: {}, output: {}",
                    listen_port,
                    output_dir.display()
                );
                let enable_upnp = opts.torrent_upnp;
                let make_opts = |disable_dht_persistence: bool, disable_dht: bool| SessionOptions {
                    listen_port_range: Some(listen_port..listen_port.saturating_add(10)),
                    disable_dht,
                    disable_dht_persistence,
                    enable_upnp_port_forwarding: enable_upnp,
                    ..Default::default()
                };
                let cancel_rx = opts.cancel_token.clone();
                let output_pb: PathBuf = output_dir.into();
                let s = tokio::select! {
                    result = async {
                        match Session::new_with_opts(output_pb.clone(), make_opts(false, false)).await {
                            Ok(s) => Ok(s),
                            Err(e) => {
                                tracing::warn!(
                                    "[magnet] persistent DHT init failed ({}); retrying without DHT persistence",
                                    e
                                );
                                match Session::new_with_opts(output_pb.clone(), make_opts(true, false)).await {
                                    Ok(s) => Ok(s),
                                    Err(e2) => {
                                        tracing::warn!(
                                            "[magnet] DHT init failed ({}); retrying with DHT disabled (trackers/PEX only)",
                                            e2
                                        );
                                        Session::new_with_opts(output_pb, make_opts(true, true)).await
                                    }
                                }
                            }
                        }
                    } => {
                        result.map_err(|e| anyhow::anyhow!("Failed to initialize torrent session: {}", e))?
                    }
                    _ = cancel_rx.cancelled() => {
                        anyhow::bail!("Download cancelled during session creation");
                    }
                };
                *guard = Some(s.clone());
                *dir_guard = Some(output_dir.clone());
                s
            } else {
                tracing::info!("[magnet] reusing existing session");
                guard.as_ref().unwrap().clone()
            }
        };

        let torrent_file = resolve_torrent_file(url, output_dir);

        let add_torrent = if let Some(path) = torrent_file.as_deref() {
            tracing::info!("[magnet] loading local torrent file: {}", path.display());
            let bytes = tokio::fs::read(path).await.map_err(|e| {
                anyhow::anyhow!("Failed to read .torrent file {}: {}", path.display(), e)
            })?;
            AddTorrent::from_bytes(bytes)
        } else if url.starts_with("magnet:")
            || url.starts_with("http://")
            || url.starts_with("https://")
        {
            AddTorrent::from_url(url)
        } else if looks_like_torrent_path(url) {
            anyhow::bail!(
                "Torrent file not found: \"{}\". Put the .torrent in the download folder or in Downloads, or paste a magnet link instead.",
                url
            )
        } else {
            AddTorrent::from_url(url)
        };
        let only_files = opts
            .torrent_files
            .as_ref()
            .filter(|v| !v.is_empty())
            .cloned();
        if let Some(sel) = &only_files {
            tracing::info!("[magnet] selective download: {} file(s)", sel.len());
        }
        let trackers = if opts.torrent_auto_trackers {
            let list = crate::core::trackers::extra_trackers();
            tracing::info!("[magnet] injecting {} public trackers", list.len());
            Some(list)
        } else {
            None
        };
        let torrent_opts = AddTorrentOptions {
            overwrite: true,
            only_files,
            trackers,
            ..Default::default()
        };

        tracing::info!("[magnet] adding torrent, output: {}", output_dir.display());
        let (torrent_id, managed_torrent) =
            match session.add_torrent(add_torrent, Some(torrent_opts)).await {
                Ok(resp) => match resp {
                    librqbit::AddTorrentResponse::Added(id, handle) => (id, handle),
                    librqbit::AddTorrentResponse::AlreadyManaged(id, handle) => (id, handle),
                    librqbit::AddTorrentResponse::ListOnly(_) => {
                        anyhow::bail!("Torrent was added in list-only mode");
                    }
                },
                Err(e) => anyhow::bail!("Failed to add torrent: {}", e),
            };

        tracing::info!(
            "[magnet] torrent added (id={}), waiting for download...",
            torrent_id
        );

        if let Some(slot) = &opts.torrent_id_slot {
            *slot.lock().await = Some(torrent_id);
        }

        let completion = managed_torrent.wait_until_completed();
        tokio::pin!(completion);

        let cancel_rx = opts.cancel_token.clone();
        let session_for_cancel = session.clone();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    if managed_torrent.is_paused() {
                        continue;
                    }
                    let stats = managed_torrent.stats();
                    let total = stats.total_bytes;
                    let downloaded = stats.progress_bytes;

                    if total > 0 {
                        let pct = (downloaded as f64 / total as f64) * 100.0;
                        let speed_bps = stats
                            .live
                            .as_ref()
                            .map(|l| l.download_speed.mbps * 1_048_576.0);
                        let eta_seconds = speed_bps.and_then(|bps| {
                            if bps > 1024.0 && total > downloaded {
                                Some(((total - downloaded) as f64 / bps) as u64)
                            } else {
                                None
                            }
                        });

                        let _ = progress
                            .send(ProgressUpdate::rich(
                                pct,
                                Some(downloaded),
                                Some(total),
                                speed_bps,
                                eta_seconds,
                            ))
                            .await;
                        tracing::debug!(
                            "[magnet] progress: {:.1}% ({:.1} MB / {:.1} MB, speed={:.1} KB/s)",
                            pct,
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                            speed_bps.unwrap_or(0.0) / 1024.0,
                        );

                        if stats.finished || downloaded >= total {
                            tracing::info!(
                                "[magnet] download complete from stats (id={}, finished={}, downloaded={}/{})",
                                torrent_id,
                                stats.finished,
                                downloaded,
                                total
                            );
                            let _ = progress
                                .send(ProgressUpdate::rich(
                                    100.0,
                                    Some(total),
                                    Some(total),
                                    Some(0.0),
                                    Some(0),
                                ))
                                .await;
                            break;
                        }
                    } else if stats.finished {
                        tracing::info!(
                            "[magnet] download complete from stats with total=0 (id={})",
                            torrent_id
                        );
                        let _ = progress
                            .send(ProgressUpdate::rich(
                                100.0,
                                Some(downloaded),
                                Some(downloaded),
                                Some(0.0),
                                Some(0),
                            ))
                            .await;
                        break;
                    }
                }
                _ = cancel_rx.cancelled() => {
                    tracing::info!("[magnet] download cancelled, removing torrent id={}", torrent_id);
                    if let Err(e) = session_for_cancel.delete(TorrentIdOrHash::Id(torrent_id), false).await {
                        tracing::warn!("[magnet] failed to delete torrent on cancel: {}", e);
                    }
                    anyhow::bail!("Download cancelled");
                }
                res = &mut completion => {
                    if let Err(e) = res {
                        anyhow::bail!("Torrent download failed: {}", e);
                    }
                    let stats = managed_torrent.stats();
                    let final_bytes = if stats.total_bytes > 0 {
                        stats.total_bytes
                    } else {
                        stats.progress_bytes
                    };
                    let _ = progress
                        .send(ProgressUpdate::rich(
                            100.0,
                            Some(final_bytes),
                            Some(final_bytes),
                            Some(0.0),
                            Some(0),
                        ))
                        .await;
                    tracing::info!("[magnet] download complete (id={})", torrent_id);
                    break;
                }
            }
        }

        let (total_size, torrent_name) = managed_torrent
            .with_metadata(|meta| {
                let size = meta
                    .info
                    .iter_file_lengths()
                    .ok()
                    .map(|iter| iter.sum::<u64>())
                    .unwrap_or_else(|| meta.file_infos.iter().map(|f| f.len).sum());
                let name = meta
                    .info
                    .name
                    .as_ref()
                    .map(|n| String::from_utf8_lossy(n.as_ref()).to_string())
                    .unwrap_or_default();
                (size, name)
            })
            .unwrap_or((0, String::new()));

        let file_path = if torrent_name.is_empty() {
            output_dir.clone()
        } else {
            output_dir.join(&torrent_name)
        };

        Ok(DownloadResult {
            file_path,
            file_size_bytes: total_size,
            duration_seconds: 0.0,
            torrent_id: Some(torrent_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torrent_suffix_is_matched_case_insensitively() {
        assert_eq!(torrent_stem("Remastered.torrent"), "Remastered");
        assert_eq!(torrent_stem("Remastered.TORRENT"), "Remastered");
        assert_eq!(torrent_stem("no-suffix"), "no-suffix");
        assert_eq!(torrent_stem(""), "");
    }

    #[test]
    fn local_file_inputs_are_recognised() {
        assert!(looks_like_torrent_path("Remastered.torrent"));
        assert!(looks_like_torrent_path(
            "  \"C:\\Users\\me\\Downloads\\Remastered.torrent\"  "
        ));
        assert!(!looks_like_torrent_path("magnet:?xt=urn:btih:abc"));
        assert!(!looks_like_torrent_path(
            "https://example.com/Remastered.torrent"
        ));
        assert!(!looks_like_torrent_path("https://example.com/video.mp4"));
        assert!(!looks_like_torrent_path("   "));
    }

    #[test]
    fn can_handle_recognises_all_torrent_sources() {
        let downloader = MagnetDownloader::new(Arc::new(tokio::sync::Mutex::new(None)));
        assert!(downloader.can_handle("magnet:?xt=urn:btih:abc"));
        assert!(downloader.can_handle("Remastered.torrent"));
        assert!(downloader.can_handle("  \"C:\\Downloads\\test.torrent\"  "));
        assert!(downloader.can_handle("https://example.com/linux.iso.torrent"));
        assert!(downloader.can_handle("http://example.com/linux.iso.TORRENT"));
        assert!(!downloader.can_handle("https://example.com/video.mp4"));
        assert!(!downloader.can_handle("https://youtube.com/watch?v=123"));
        assert!(!downloader.can_handle("   "));
    }

    #[test]
    fn bare_filename_resolves_through_search_dirs() {
        let tmp = std::env::temp_dir().join(format!("omniget-magnet-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let file = tmp.join("OmniGetTest.torrent");
        std::fs::write(&file, b"d4:infod4:name4:teste").unwrap();

        // an absolute or relative path that exists is used as given
        assert_eq!(
            resolve_torrent_file(file.to_str().unwrap(), &tmp),
            Some(file.clone())
        );
        // a bare filename is found in the output folder
        assert_eq!(
            resolve_torrent_file("OmniGetTest.torrent", &tmp),
            Some(file.clone())
        );

        // a missing filename stays a local-path input so the caller can report it
        assert!(looks_like_torrent_path("omniget-missing-xyz.torrent"));
        assert_eq!(
            resolve_torrent_file("omniget-missing-xyz.torrent", &tmp),
            None
        );

        // URIs are never resolved as files
        assert_eq!(resolve_torrent_file("magnet:?xt=urn:btih:abc", &tmp), None);
        assert_eq!(
            resolve_torrent_file("https://example.com/OmniGetTest.torrent", &tmp),
            None
        );

        std::fs::remove_file(&file).ok();
        std::fs::remove_dir_all(&tmp).ok();
    }
}
