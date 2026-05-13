//! Tauri commands exposed to the Settings → Cookies UI.

use serde::{Deserialize, Serialize};

use super::parsers;
use super::platform::PlatformKind;
use super::storage::{
    self, AccountEntry, CookieRegistry, IngestSource,
};

#[derive(Debug, Serialize)]
pub struct CookieListResponse {
    pub registry: CookieRegistry,
    pub cookies_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub content: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub buckets_written: Vec<BucketWrite>,
}

#[derive(Debug, Serialize)]
pub struct BucketWrite {
    pub domain: String,
    pub cookie_count: usize,
    pub platform_kind: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadRequest {
    pub domain: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadResponse {
    pub content: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ClearRequest {
    pub domain: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub domain: String,
    #[serde(default)]
    pub slug: Option<String>,
    pub new_alias: String,
}

#[derive(Debug, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

const DEFAULT_SLUG: &str = "_default";

#[tauri::command]
pub async fn cookies_list() -> Result<CookieListResponse, String> {
    let registry = storage::load_registry();
    let cookies_dir = storage::cookies_root().to_string_lossy().into_owned();
    Ok(CookieListResponse { registry, cookies_dir })
}

#[tauri::command]
pub async fn cookies_read(request: ReadRequest) -> Result<ReadResponse, String> {
    let slug = request.slug.as_deref().unwrap_or(DEFAULT_SLUG);
    let content = storage::read_account_file(&request.domain, slug).map_err(|e| e.to_string())?;
    let path = storage::account_file(&request.domain, slug).to_string_lossy().into_owned();
    Ok(ReadResponse { content, path })
}

#[tauri::command]
pub async fn cookies_import(request: ImportRequest) -> Result<ImportResponse, String> {
    let cookies = parsers::parse(&request.content).map_err(|e| e.to_string())?;
    if cookies.is_empty() {
        return Err("No cookies found in payload".to_string());
    }
    let label = request.source_label.unwrap_or_else(|| "Manual import".to_string());
    let written = storage::ingest_batch(
        &cookies,
        IngestSource {
            source_url: request.source_url,
            source_label: label,
            alias_hint: request.alias,
        },
    )
    .map_err(|e| e.to_string())?;

    let buckets_written = written
        .into_iter()
        .map(|(domain, cookie_count)| {
            let platform_kind = PlatformKind::from_domain(&domain).as_str().to_string();
            BucketWrite { domain, cookie_count, platform_kind }
        })
        .collect();
    Ok(ImportResponse { buckets_written })
}

#[tauri::command]
pub async fn cookies_clear(request: ClearRequest) -> Result<OkResponse, String> {
    let slug = request.slug.as_deref().unwrap_or(DEFAULT_SLUG);
    storage::move_to_trash(&request.domain, slug).map_err(|e| e.to_string())?;
    Ok(OkResponse { ok: true })
}

#[tauri::command]
pub async fn cookies_rename(request: RenameRequest) -> Result<OkResponse, String> {
    let slug = request.slug.as_deref().unwrap_or(DEFAULT_SLUG);
    if request.new_alias.trim().is_empty() {
        return Err("Alias cannot be empty".to_string());
    }
    storage::rename_account(&request.domain, slug, request.new_alias.trim())
        .map_err(|e| e.to_string())?;
    Ok(OkResponse { ok: true })
}

#[tauri::command]
pub async fn cookies_migrate_legacy() -> Result<OkResponse, String> {
    storage::migrate_legacy_if_needed().map_err(|e| e.to_string())?;
    Ok(OkResponse { ok: true })
}

#[tauri::command]
pub async fn cookies_detect_platform(domain: String) -> Result<String, String> {
    Ok(PlatformKind::from_domain(&domain).as_str().to_string())
}

#[derive(Debug, Deserialize)]
pub struct ImportFileRequest {
    pub path: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[tauri::command]
pub async fn cookies_import_file(request: ImportFileRequest) -> Result<ImportResponse, String> {
    let path = std::path::Path::new(&request.path);
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", request.path))?;
    let cookies = parsers::parse(&content).map_err(|e| e.to_string())?;
    if cookies.is_empty() {
        return Err("No cookies found in file".to_string());
    }
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| request.path.clone());
    let label = format!("File import: {}", filename);
    let written = storage::ingest_batch(
        &cookies,
        IngestSource {
            source_url: None,
            source_label: label,
            alias_hint: request.alias,
        },
    )
    .map_err(|e| e.to_string())?;
    let buckets_written = written
        .into_iter()
        .map(|(domain, cookie_count)| {
            let platform_kind = PlatformKind::from_domain(&domain).as_str().to_string();
            BucketWrite { domain, cookie_count, platform_kind }
        })
        .collect();
    Ok(ImportResponse { buckets_written })
}

#[derive(Debug, Deserialize)]
pub struct ExportToRequest {
    pub domain: String,
    #[serde(default)]
    pub slug: Option<String>,
    pub destination_path: String,
}

#[derive(Debug, Deserialize)]
pub struct AddAccountRequest {
    pub domain: String,
    pub alias: String,
    pub content: String,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddAccountResponse {
    pub slug: String,
    pub cookie_count: usize,
}

#[tauri::command]
pub async fn cookies_add_account(request: AddAccountRequest) -> Result<AddAccountResponse, String> {
    if request.alias.trim().is_empty() {
        return Err("Alias is required for a new account".to_string());
    }
    let cookies = parsers::parse(&request.content).map_err(|e| e.to_string())?;
    if cookies.is_empty() {
        return Err("No cookies found in payload".to_string());
    }
    let (slug, cookie_count) = storage::ingest_to_account(
        &request.domain,
        request.alias.trim(),
        &cookies,
        IngestSource {
            source_url: request.source_url,
            source_label: "Manual add (multi-account)".to_string(),
            alias_hint: Some(request.alias.trim().to_string()),
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(AddAccountResponse { slug, cookie_count })
}

#[tauri::command]
pub async fn cookies_export_to(request: ExportToRequest) -> Result<OkResponse, String> {
    let slug = request.slug.as_deref().unwrap_or(DEFAULT_SLUG);
    let content = storage::read_account_file(&request.domain, slug).map_err(|e| e.to_string())?;
    std::fs::write(&request.destination_path, content)
        .map_err(|e| format!("Failed to write {}: {e}", request.destination_path))?;
    Ok(OkResponse { ok: true })
}

#[allow(dead_code)]
pub fn list_accounts_for_domain(domain: &str) -> Vec<AccountEntry> {
    let registry = storage::load_registry();
    registry
        .buckets
        .get(domain)
        .map(|b| b.accounts.clone())
        .unwrap_or_default()
}
