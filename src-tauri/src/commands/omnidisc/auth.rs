use super::api::{Api, ERR_SERVER};
use super::store;
use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
pub struct SessionStatus {
    pub url: String,
    pub has_session: bool,
}

fn device_name() -> String {
    let host = hostname();
    match host {
        Some(h) => format!("OmniGet on {}", h),
        None => "OmniGet".to_string(),
    }
}

fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

async fn finish_auth(api: &Api, body: Value) -> Result<Value, String> {
    let token = body
        .get("token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ERR_SERVER.to_string())?;
    let user = body.get("user").cloned().ok_or_else(|| ERR_SERVER.to_string())?;
    store::save_token(&api.base, token)?;
    let username = user.get("username").and_then(Value::as_str).unwrap_or("?").to_string();
    tracing::info!("[omnidisc] signed in to {} as {}", api.base, username);
    Ok(user)
}

pub async fn register(
    url: &str,
    username: &str,
    password: &str,
    display_name: Option<&str>,
    invite_code: Option<&str>,
) -> Result<Value, String> {
    let api = Api::public(url)?;
    let mut body = json!({ "username": username.trim(), "password": password });
    if let Some(d) = display_name.map(str::trim).filter(|d| !d.is_empty()) {
        body["display_name"] = Value::String(d.to_string());
    }
    if let Some(c) = invite_code.map(str::trim).filter(|c| !c.is_empty()) {
        body["invite_code"] = Value::String(super::api::extract_invite_code(c));
    }
    let res: Value = api.send(Method::POST, "/api/auth/register", &[], Some(body)).await?;
    finish_auth(&api, res).await
}

pub async fn login(url: &str, username: &str, password: &str) -> Result<Value, String> {
    let api = Api::public(url)?;
    let body = json!({
        "username": username.trim(),
        "password": password,
        "device_name": device_name(),
    });
    let res: Value = api.send(Method::POST, "/api/auth/login", &[], Some(body)).await?;
    finish_auth(&api, res).await
}

pub async fn logout(url: &str) -> Result<(), String> {
    let base = super::normalize_instance_url(url)?;
    if let Ok(api) = Api::authed(&base) {
        if let Err(e) = api.send_empty(Method::POST, "/api/auth/logout", None).await {
            tracing::warn!("[omnidisc] logout on {} failed ({}); dropping the local session anyway", base, e);
        }
    }
    store::delete_token(&base)
}

#[tauri::command]
pub async fn omnidisc_register(
    url: String,
    username: String,
    password: String,
    display_name: Option<String>,
    invite_code: Option<String>,
) -> Result<Value, String> {
    register(&url, &username, &password, display_name.as_deref(), invite_code.as_deref()).await
}

#[tauri::command]
pub async fn omnidisc_login(url: String, username: String, password: String) -> Result<Value, String> {
    login(&url, &username, &password).await
}

#[tauri::command]
pub async fn omnidisc_logout(
    state: tauri::State<'_, crate::AppState>,
    url: String,
) -> Result<(), String> {
    let base = super::normalize_instance_url(&url)?;
    super::gateway::stop(&state.omnidisc_gateways, &base).await;
    // The device key and the MLS state stay on disk: signing back in on the same
    // instance keeps this device (and therefore its readable history) instead of
    // silently orphaning every group it belongs to.
    state.omnidisc_mls.forget(&base).await;
    logout(&base).await
}

#[tauri::command]
pub async fn omnidisc_has_session(url: String) -> Result<SessionStatus, String> {
    let base = super::normalize_instance_url(&url)?;
    let has_session = store::load_token(&base)?.is_some();
    Ok(SessionStatus { url: base, has_session })
}
