use std::time::Duration;

use serde::{Deserialize, Serialize};

const MANIFEST_URL: &str = "https://petdex.crafter.run/api/manifest";
const FALLBACK_BUNDLE_URL: &str =
    "https://pub-94495283df974cfea5e98d6a9e3fa462.r2.dev/packs/petdex-approved.zip";
const USER_AGENT: &str = "omniget-pets/1.0";

const MIN_SPRITE_BYTES: usize = 10 * 1024;
const MAX_SPRITE_BYTES: usize = 5 * 1024 * 1024;
const MAX_PET_JSON_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteManifest {
    #[serde(rename = "generatedAt")]
    pub generated_at: Option<String>,
    pub total: u32,
    #[serde(default)]
    pub featured: u32,
    #[serde(rename = "allPetsPackPath")]
    pub all_pets_pack_path: Option<String>,
    pub pets: Vec<RemotePet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePet {
    pub slug: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub vibes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default, rename = "spritesheetUrl")]
    pub spritesheet_url: Option<String>,
    #[serde(default, rename = "petJsonUrl")]
    pub pet_json_url: Option<String>,
    #[serde(default, rename = "zipUrl")]
    pub zip_url: Option<String>,
    #[serde(default, rename = "submittedBy")]
    pub submitted_by: Option<String>,
}

fn build_client(timeout_secs: u64) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(30))
        .build()
}

fn is_retryable(status: Option<u16>) -> bool {
    match status {
        Some(s) => s >= 500 || s == 408 || s == 429,
        None => true,
    }
}

async fn with_retry<F, Fut, T>(label: &str, mut op: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, RetryError>>,
{
    let mut backoff_ms: u64 = 1000;
    for attempt in 0..3 {
        match op().await {
            Ok(v) => return Ok(v),
            Err(err) => {
                let last = attempt == 2;
                if last || !err.retryable {
                    return Err(anyhow::anyhow!("{}: {}", label, err.message));
                }
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2;
            }
        }
    }
    anyhow::bail!("{}: retry exhausted", label)
}

pub struct RetryError {
    pub message: String,
    pub retryable: bool,
}

pub async fn fetch_manifest() -> anyhow::Result<RemoteManifest> {
    with_retry("fetch_manifest", || async {
        let client = build_client(60).map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        let res = client.get(MANIFEST_URL).send().await.map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        let status = res.status();
        if !status.is_success() {
            let retryable = is_retryable(Some(status.as_u16()));
            return Err(RetryError {
                message: format!("status {}", status),
                retryable,
            });
        }
        let text = res.text().await.map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        let parsed: RemoteManifest = serde_json::from_str(&text).map_err(|e| RetryError {
            message: format!("parse: {}", e),
            retryable: false,
        })?;
        Ok(parsed)
    })
    .await
}

pub fn fallback_bundle_url() -> &'static str {
    FALLBACK_BUNDLE_URL
}

pub async fn fetch_bytes(url: &str, max_bytes: usize, timeout_secs: u64) -> anyhow::Result<Vec<u8>> {
    let label = format!("GET {}", url);
    with_retry(&label, || async {
        let client = build_client(timeout_secs).map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        let res = client.get(url).send().await.map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        let status = res.status();
        if !status.is_success() {
            let retryable = is_retryable(Some(status.as_u16()));
            return Err(RetryError {
                message: format!("status {}", status),
                retryable,
            });
        }
        let bytes = res.bytes().await.map_err(|e| RetryError {
            message: e.to_string(),
            retryable: true,
        })?;
        if bytes.len() > max_bytes {
            return Err(RetryError {
                message: format!("payload {} > limit {}", bytes.len(), max_bytes),
                retryable: false,
            });
        }
        Ok(bytes.to_vec())
    })
    .await
}

pub async fn fetch_pet_json(url: &str) -> anyhow::Result<Vec<u8>> {
    fetch_bytes(url, MAX_PET_JSON_BYTES, 60).await
}

pub async fn fetch_spritesheet(url: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = fetch_bytes(url, MAX_SPRITE_BYTES, 60).await?;
    if bytes.len() < MIN_SPRITE_BYTES {
        anyhow::bail!("spritesheet too small ({} bytes)", bytes.len());
    }
    if !is_webp_or_png(&bytes) {
        anyhow::bail!("spritesheet is not webp or png");
    }
    Ok(bytes)
}

fn is_webp_or_png(b: &[u8]) -> bool {
    if b.len() < 12 {
        return false;
    }
    if &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return true;
    }
    if b.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return true;
    }
    false
}

pub async fn fetch_bundle_zip(url: &str) -> anyhow::Result<Vec<u8>> {
    fetch_bytes(url, 200 * 1024 * 1024, 600).await
}

pub fn extract_ext_from_url(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else {
        "webp"
    }
}
