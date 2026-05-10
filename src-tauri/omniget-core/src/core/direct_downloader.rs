use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use std::time::Duration;

use anyhow::anyhow;
use futures::StreamExt;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::core::http_fetcher::{
    get_global_max_concurrent_segments, HttpFetcher, HttpFetcherConfig,
};

const CHUNK_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_RETRIES: u32 = 3;
const CHUNK_SIZE: u64 = 10 * 1024 * 1024;
const CHUNK_THRESHOLD: u64 = 10 * 1024 * 1024;
const MAX_PARALLEL: usize = 12;
const MAX_PER_HOST: usize = 16;

fn host_semaphores() -> &'static tokio::sync::Mutex<HashMap<String, Arc<Semaphore>>> {
    static MAP: OnceLock<tokio::sync::Mutex<HashMap<String, Arc<Semaphore>>>> = OnceLock::new();
    MAP.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

pub async fn get_host_semaphore(url: &str) -> Arc<Semaphore> {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    let mut map = host_semaphores().lock().await;
    map.entry(host)
        .or_insert_with(|| Arc::new(Semaphore::new(MAX_PER_HOST)))
        .clone()
}

struct ProbeResult {
    content_length: Option<u64>,
    accept_ranges: bool,
}

pub async fn download_direct(
    client: &reqwest::Client,
    url: &str,
    output: &Path,
    progress_tx: mpsc::Sender<f64>,
    cancel: Option<&CancellationToken>,
) -> anyhow::Result<u64> {
    download_direct_with_headers(client, url, output, progress_tx, None, cancel).await
}

pub async fn download_direct_with_headers(
    client: &reqwest::Client,
    url: &str,
    output: &Path,
    progress_tx: mpsc::Sender<f64>,
    headers: Option<reqwest::header::HeaderMap>,
    cancel: Option<&CancellationToken>,
) -> anyhow::Result<u64> {
    let mut last_err = None;

    for attempt in 0..MAX_RETRIES {
        if let Some(token) = cancel {
            if token.is_cancelled() {
                return Err(anyhow!("Download cancelled"));
            }
        }

        if attempt > 0 {
            let base = 1000 * (attempt as u64);
            let jitter = rand::random::<u64>() % (base / 2 + 1);
            tokio::time::sleep(Duration::from_millis(base + jitter)).await;
        }

        match download_attempt(client, url, output, &progress_tx, headers.clone(), cancel).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                if is_fatal_error(&e) {
                    let _ = std::fs::remove_file(&part_path_for(output));
                    return Err(e);
                }
                tracing::warn!(
                    "[direct] attempt {}/{} failed: {}",
                    attempt + 1,
                    MAX_RETRIES,
                    e
                );
                last_err = Some(e);
            }
        }
    }

    let _ = std::fs::remove_file(&part_path_for(output));
    Err(last_err.unwrap_or_else(|| anyhow!("Download failed after {} attempts", MAX_RETRIES)))
}

fn part_path_for(output: &Path) -> PathBuf {
    let mut part = output.as_os_str().to_owned();
    part.push(".part");
    PathBuf::from(part)
}

fn is_fatal_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    for code in &[
        "HTTP 400", "HTTP 401", "HTTP 403", "HTTP 404", "HTTP 405", "HTTP 410", "HTTP 451",
    ] {
        if msg.contains(code) {
            return true;
        }
    }
    if msg.contains("HTML instead of media") {
        return true;
    }
    if msg.contains("cancelled") {
        return true;
    }
    false
}

async fn probe_url(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&reqwest::header::HeaderMap>,
) -> ProbeResult {
    let mut request = client.head(url);
    if let Some(h) = headers {
        request = request.headers(h.clone());
    }
    match tokio::time::timeout(Duration::from_secs(15), request.send()).await {
        Ok(Ok(resp)) if resp.status().is_success() => {
            let content_length = resp.content_length();
            let accept_ranges = resp
                .headers()
                .get("accept-ranges")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("bytes"))
                .unwrap_or(false);
            ProbeResult {
                content_length,
                accept_ranges,
            }
        }
        _ => ProbeResult {
            content_length: None,
            accept_ranges: false,
        },
    }
}

async fn download_attempt(
    client: &reqwest::Client,
    url: &str,
    output: &Path,
    progress_tx: &mpsc::Sender<f64>,
    headers: Option<reqwest::header::HeaderMap>,
    cancel: Option<&CancellationToken>,
) -> anyhow::Result<u64> {
    let part_path = part_path_for(output);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let probe = probe_url(client, url, headers.as_ref()).await;

    let use_chunked =
        probe.accept_ranges && probe.content_length.is_some_and(|s| s > CHUNK_THRESHOLD);

    if use_chunked {
        match run_http_fetcher(client, url, output, progress_tx, headers.clone(), cancel).await {
            Ok(size) => return Ok(size),
            Err(fetch_err) => {
                if is_fatal_error(&fetch_err) {
                    return Err(fetch_err);
                }
                let _ = std::fs::remove_file(&part_path);
                tracing::warn!(
                    "[direct] http_fetcher failed, falling back to single stream: {}",
                    fetch_err
                );
                download_single_stream(
                    client,
                    url,
                    &part_path,
                    0,
                    probe.content_length,
                    progress_tx,
                    headers,
                    cancel,
                )
                .await?;
            }
        }
    } else {
        let existing = match std::fs::metadata(&part_path) {
            Ok(m) if m.len() > 0 && probe.accept_ranges => m.len(),
            _ => 0,
        };
        download_single_stream(
            client,
            url,
            &part_path,
            existing,
            probe.content_length,
            progress_tx,
            headers,
            cancel,
        )
        .await?;
    }

    if let Some(expected) = probe.content_length {
        let actual = std::fs::metadata(&part_path)?.len();
        if expected > 0 && actual != expected {
            let _ = std::fs::remove_file(&part_path);
            return Err(anyhow!(
                "Size mismatch: expected {} bytes, got {}",
                expected,
                actual
            ));
        }
    }

    std::fs::rename(&part_path, output)?;
    let _ = progress_tx.send(100.0).await;

    let size = std::fs::metadata(output)?.len();
    Ok(size)
}

async fn run_http_fetcher(
    client: &reqwest::Client,
    url: &str,
    output: &Path,
    progress_tx: &mpsc::Sender<f64>,
    headers: Option<reqwest::header::HeaderMap>,
    cancel: Option<&CancellationToken>,
) -> anyhow::Result<u64> {
    let concurrent = get_global_max_concurrent_segments()
        .unwrap_or(MAX_PARALLEL)
        .clamp(1, 32);
    let cfg = HttpFetcherConfig {
        min_size_for_chunked: 0,
        concurrent_segments: concurrent,
        segment_size_hint: CHUNK_SIZE,
        ..Default::default()
    };

    let mut fetcher = HttpFetcher::new(client.clone(), url.to_string(), output.to_path_buf())
        .with_config(cfg);
    if let Some(h) = headers {
        fetcher = fetcher.with_headers(h);
    }
    if let Some(c) = cancel {
        fetcher = fetcher.with_cancel(c.clone());
    }
    let result = fetcher.download(progress_tx.clone()).await?;
    Ok(result.bytes_written)
}

#[allow(clippy::too_many_arguments)]
async fn download_single_stream(
    client: &reqwest::Client,
    url: &str,
    part_path: &Path,
    existing_bytes: u64,
    total_size: Option<u64>,
    progress_tx: &mpsc::Sender<f64>,
    headers: Option<reqwest::header::HeaderMap>,
    cancel: Option<&CancellationToken>,
) -> anyhow::Result<()> {
    let mut request = client.get(url);
    if let Some(h) = headers {
        request = request.headers(h);
    }

    if existing_bytes > 0 {
        if let Some(total) = total_size {
            if existing_bytes >= total {
                return Ok(());
            }
        }
        request = request.header("Range", format!("bytes={}-", existing_bytes));
    }

    let response = request.send().await?;

    let mut offset = 0u64;
    if existing_bytes > 0 {
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            offset = existing_bytes;
        } else if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            let _ = std::fs::remove_file(part_path);
            return Err(anyhow!("Range not satisfiable, restarting"));
        } else if !response.status().is_success() {
            return Err(anyhow!("HTTP {} downloading {}", response.status(), url));
        }
    } else if !response.status().is_success() {
        return Err(anyhow!("HTTP {} downloading {}", response.status(), url));
    }

    if let Some(ct) = response.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            if ct_str.contains("text/html") {
                return Err(anyhow!(
                    "Server returned HTML instead of media — URL may have expired"
                ));
            }
        }
    }

    use std::io::Write;
    let raw_file = if offset > 0 {
        std::fs::OpenOptions::new().append(true).open(part_path)?
    } else {
        std::fs::File::create(part_path)?
    };

    let mut file = std::io::BufWriter::with_capacity(256 * 1024, raw_file);
    let mut downloaded = offset;
    let mut stream = response.bytes_stream();

    loop {
        if let Some(token) = cancel {
            if token.is_cancelled() {
                file.flush()?;
                return Err(anyhow!("Download cancelled"));
            }
        }

        let chunk_result = tokio::time::timeout(CHUNK_TIMEOUT, stream.next()).await;
        match chunk_result {
            Ok(Some(Ok(chunk))) => {
                file.write_all(&chunk)
                    .map_err(|e| anyhow!("Write error (disk full?): {}", e))?;
                downloaded += chunk.len() as u64;

                if let Some(total) = total_size {
                    if total > 0 {
                        let percent = (downloaded as f64 / total as f64) * 100.0;
                        let _ = progress_tx.send(percent).await;
                    }
                } else {
                    let percent = (downloaded as f64 / (downloaded as f64 + 500_000.0)) * 100.0;
                    let _ = progress_tx.send(percent.min(95.0)).await;
                }
            }
            Ok(Some(Err(e))) => {
                file.flush()?;
                return Err(anyhow!("Download stream error: {}", e));
            }
            Ok(None) => break,
            Err(_) => {
                file.flush()?;
                return Err(anyhow!(
                    "Download timeout — no data received for 30 seconds"
                ));
            }
        }
    }

    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_appends_suffix() {
        let output = Path::new("video.mp4");
        let part = part_path_for(output);
        assert_eq!(part, PathBuf::from("video.mp4.part"));
    }

    #[test]
    fn part_path_no_extension() {
        let output = Path::new("video");
        let part = part_path_for(output);
        assert_eq!(part, PathBuf::from("video.part"));
    }

    #[test]
    fn part_path_nested() {
        let output = Path::new("downloads/curso/aula.mp4");
        let part = part_path_for(output);
        assert_eq!(part, PathBuf::from("downloads/curso/aula.mp4.part"));
    }

    #[test]
    fn is_fatal_http_400() {
        assert!(is_fatal_error(&anyhow!("HTTP 400 downloading url")));
    }

    #[test]
    fn is_fatal_http_401() {
        assert!(is_fatal_error(&anyhow!("HTTP 401 downloading url")));
    }

    #[test]
    fn is_fatal_http_403() {
        assert!(is_fatal_error(&anyhow!("HTTP 403 downloading url")));
    }

    #[test]
    fn is_fatal_http_404() {
        assert!(is_fatal_error(&anyhow!("HTTP 404 downloading url")));
    }

    #[test]
    fn is_fatal_html_response() {
        assert!(is_fatal_error(&anyhow!(
            "Server returned HTML instead of media — URL may have expired"
        )));
    }

    #[test]
    fn is_fatal_cancelled() {
        assert!(is_fatal_error(&anyhow!("Download cancelled")));
    }

    #[test]
    fn is_not_fatal_500() {
        assert!(!is_fatal_error(&anyhow!("HTTP 500 Internal Server Error")));
    }

    #[test]
    fn is_not_fatal_502() {
        assert!(!is_fatal_error(&anyhow!("HTTP 502 Bad Gateway")));
    }

    #[test]
    fn is_not_fatal_timeout() {
        assert!(!is_fatal_error(&anyhow!("connection timed out")));
    }

    #[test]
    fn is_not_fatal_network() {
        assert!(!is_fatal_error(&anyhow!("network error")));
    }

    #[test]
    fn chunk_count_for_12mb() {
        let total: u64 = 12 * 1024 * 1024;
        assert_eq!(total.div_ceil(CHUNK_SIZE), 2);
    }

    #[test]
    fn chunk_count_exact_boundary() {
        assert_eq!(CHUNK_SIZE.div_ceil(CHUNK_SIZE), 1);
    }

    #[test]
    fn chunk_count_single_byte_over() {
        assert_eq!((CHUNK_SIZE + 1).div_ceil(CHUNK_SIZE), 2);
    }

    #[test]
    fn threshold_gte_chunk_size() {
        assert!(CHUNK_THRESHOLD >= CHUNK_SIZE);
    }
}
