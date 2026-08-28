use super::{normalize_instance_url, store};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::time::Duration;

pub const ERR_UNAUTHORIZED: &str = "ERR_UNAUTHORIZED";
pub const ERR_FORBIDDEN: &str = "ERR_FORBIDDEN";
pub const ERR_NOT_FOUND: &str = "ERR_NOT_FOUND";
pub const ERR_RATE_LIMITED: &str = "ERR_RATE_LIMITED";
pub const ERR_UNREACHABLE: &str = "ERR_UNREACHABLE";
pub const ERR_SERVER: &str = "ERR_SERVER";
pub const ERR_BAD_REQUEST: &str = "ERR_BAD_REQUEST";
pub const ERR_NO_SESSION: &str = "ERR_NO_SESSION";

/// Ids reach `Api` as strings from the frontend and from server payloads, and
/// they end up interpolated into a request path. `Url::parse` resolves `..`, so
/// an id is only ever a snowflake, a device id or a group id: never a way to
/// walk out of the route it was meant for.
pub fn path_id(id: &str) -> Result<&str, String> {
    let ok = !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if ok {
        Ok(id)
    } else {
        tracing::warn!("[omnidisc] refused an id that is not a plain identifier");
        Err(format!("{}:invalid_id", ERR_BAD_REQUEST))
    }
}

/// Last line of defence for every request this module makes: the assembled path
/// must still be the route we wrote, whatever was interpolated into it.
fn safe_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains("..")
        && !path.contains("//")
        && path.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '@' | '%' | '~')
        })
}

pub fn http_client(timeout: Duration) -> Result<reqwest::Client, String> {
    crate::core::http_client::apply_global_proxy(
        reqwest::Client::builder()
            .user_agent(format!("OmniGet/{}", env!("CARGO_PKG_VERSION")))
            .timeout(timeout),
    )
    .build()
    .map_err(|e| format!("OmniDisc: could not build HTTP client: {}", e))
}

pub struct Api {
    pub base: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl Api {
    pub fn public(url: &str) -> Result<Self, String> {
        let base = normalize_instance_url(url)?;
        Ok(Self {
            base,
            token: None,
            http: http_client(Duration::from_secs(15))?,
        })
    }

    pub fn authed(url: &str) -> Result<Self, String> {
        let base = normalize_instance_url(url)?;
        let token = store::load_token(&base)?.ok_or_else(|| ERR_NO_SESSION.to_string())?;
        Ok(Self {
            base,
            token: Some(token),
            http: http_client(Duration::from_secs(15))?,
        })
    }

    pub fn with_token(base: String, token: String) -> Result<Self, String> {
        Ok(Self {
            base,
            token: Some(token),
            http: http_client(Duration::from_secs(15))?,
        })
    }

    pub async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<T, String> {
        let text = self.raw(method, path, query, body).await?;
        serde_json::from_str(&text).map_err(|e| {
            tracing::warn!(
                "[omnidisc] {}{} returned unexpected JSON: {}",
                self.base,
                path,
                e
            );
            ERR_SERVER.to_string()
        })
    }

    pub async fn send_empty(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<(), String> {
        self.raw(method, path, &[], body).await.map(|_| ())
    }

    async fn raw(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<String, String> {
        if !safe_path(path) {
            tracing::warn!("[omnidisc] refused a request path that is not a plain route");
            return Err(format!("{}:invalid_path", ERR_BAD_REQUEST));
        }
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.request(method.clone(), &url);
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req.send().await.map_err(|e| {
            tracing::warn!("[omnidisc] {} {} unreachable: {}", method, url, e);
            ERR_UNREACHABLE.to_string()
        })?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status.is_success() {
            return Ok(text);
        }
        let code = serde_json::from_str::<omnidisc_proto::rest::ApiError>(&text)
            .map(|e| e.code)
            .unwrap_or_default();
        tracing::warn!(
            "[omnidisc] {} {} -> {} {}",
            method,
            url,
            status.as_u16(),
            code
        );
        Err(map_error(status, &code))
    }
}

pub fn map_error(status: StatusCode, code: &str) -> String {
    match status {
        StatusCode::UNAUTHORIZED => ERR_UNAUTHORIZED.to_string(),
        StatusCode::FORBIDDEN => with_code(ERR_FORBIDDEN, code),
        StatusCode::NOT_FOUND => ERR_NOT_FOUND.to_string(),
        StatusCode::TOO_MANY_REQUESTS => ERR_RATE_LIMITED.to_string(),
        s if s.is_client_error() => with_code(ERR_BAD_REQUEST, code),
        _ => ERR_SERVER.to_string(),
    }
}

fn with_code(base: &str, code: &str) -> String {
    if code.is_empty() {
        base.to_string()
    } else {
        format!("{}:{}", base, code)
    }
}

fn opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[tauri::command]
pub async fn omnidisc_list_messages(
    url: String,
    channel_id: String,
    before: Option<String>,
    after: Option<String>,
    around: Option<String>,
    limit: Option<u32>,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(b) = opt(before) {
        query.push(("before", b));
    }
    if let Some(a) = opt(after) {
        query.push(("after", a));
    }
    if let Some(a) = opt(around) {
        query.push(("around", a));
    }
    if let Some(l) = limit {
        query.push(("limit", l.clamp(1, 100).to_string()));
    }
    api.send(
        Method::GET,
        &format!("/api/channels/{}/messages", channel_id),
        &query,
        None,
    )
    .await
}

/// One entry point for both surfaces. `encrypted` comes from the channel kind:
/// DMs and group DMs go through MLS, guild channels go through the plain REST
/// route the server can read. Uploads are referenced by their local id so the
/// file keys never cross the bridge.
// The argument list is the IPC contract the frontend calls by name; folding it
// into a struct would change the payload shape for no gain here.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn omnidisc_send_message(
    state: tauri::State<'_, crate::AppState>,
    url: String,
    channel_id: String,
    content: String,
    reply_to: Option<String>,
    nonce: Option<String>,
    upload_ids: Option<Vec<String>>,
    encrypted: Option<bool>,
    recipient_ids: Option<Vec<String>>,
) -> Result<Value, String> {
    let base = super::normalize_instance_url(&url)?;
    let upload_ids = upload_ids.unwrap_or_default();
    let uploads = state.omnidisc_uploads.peek(&upload_ids).await?;
    if encrypted.unwrap_or(false) {
        let payload = super::mls::E2eePayload {
            v: 1,
            content,
            reply_to: opt(reply_to),
            nonce: opt(nonce),
            files: uploads.iter().filter_map(|u| u.manifest()).collect(),
        };
        let recipients = recipient_ids.unwrap_or_default();
        let handle = state.omnidisc_mls.session(&base).await?;
        let mut session = handle.lock().await;
        let api = Api::authed(&base)?;
        let sent =
            super::mls::send_encrypted(&api, &mut session, &channel_id, &recipients, payload)
                .await?;
        state.omnidisc_uploads.release(&upload_ids).await;
        return Ok(sent);
    }
    let api = Api::authed(&base)?;
    let mut body = json!({ "content": content });
    if let Some(n) = opt(nonce) {
        body["nonce"] = Value::String(n);
    }
    if let Some(r) = opt(reply_to) {
        body["reference"] = json!({ "message_id": r, "channel_id": channel_id });
    }
    let attachment_ids: Vec<String> = uploads.iter().map(|u| u.attachment_id.clone()).collect();
    if !attachment_ids.is_empty() {
        body["attachment_ids"] = json!(attachment_ids);
    }
    let sent = api
        .send(
            Method::POST,
            &format!("/api/channels/{}/messages", channel_id),
            &[],
            Some(body),
        )
        .await?;
    state.omnidisc_uploads.release(&upload_ids).await;
    Ok(sent)
}

#[tauri::command]
pub async fn omnidisc_edit_message(
    url: String,
    channel_id: String,
    message_id: String,
    content: String,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::PATCH,
        &format!("/api/channels/{}/messages/{}", channel_id, message_id),
        &[],
        Some(json!({ "content": content })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_delete_message(
    url: String,
    channel_id: String,
    message_id: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/channels/{}/messages/{}", channel_id, message_id),
        None,
    )
    .await
}

fn reaction_path(channel_id: &str, message_id: &str, emoji: &str) -> String {
    format!(
        "/api/channels/{}/messages/{}/reactions/{}/@me",
        channel_id,
        message_id,
        urlencoding::encode(emoji)
    )
}

#[tauri::command]
pub async fn omnidisc_add_reaction(
    url: String,
    channel_id: String,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::PUT,
        &reaction_path(&channel_id, &message_id, &emoji),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_remove_reaction(
    url: String,
    channel_id: String,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &reaction_path(&channel_id, &message_id, &emoji),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_ack(
    url: String,
    channel_id: String,
    message_id: String,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::POST,
        &format!("/api/channels/{}/ack", channel_id),
        &[],
        Some(json!({ "message_id": message_id })),
    )
    .await
}

pub async fn typing_rest(url: &str, channel_id: &str) -> Result<(), String> {
    let api = Api::authed(url)?;
    api.send_empty(
        Method::POST,
        &format!("/api/channels/{}/typing", channel_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_create_guild(url: String, name: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::POST,
        "/api/guilds",
        &[],
        Some(json!({ "name": name })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_create_channel(
    url: String,
    guild_id: String,
    name: String,
    kind: u8,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::POST,
        &format!("/api/guilds/{}/channels", guild_id),
        &[],
        Some(json!({ "name": name, "type": kind })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_create_invite(
    url: String,
    guild_id: String,
    channel_id: Option<String>,
    max_age_seconds: Option<u64>,
    max_uses: Option<u32>,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let mut body = json!({ "guild_id": guild_id });
    if let Some(c) = opt(channel_id) {
        body["channel_id"] = Value::String(c);
    }
    if let Some(age) = max_age_seconds.filter(|a| *a > 0) {
        body["max_age_seconds"] = json!(age);
    }
    if let Some(uses) = max_uses.filter(|u| *u > 0) {
        body["max_uses"] = json!(uses);
    }
    api.send(Method::POST, "/api/invites", &[], Some(body))
        .await
}

#[tauri::command]
pub async fn omnidisc_join_invite(url: String, code: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let code = extract_invite_code(&code);
    if code.is_empty() {
        return Err(format!("{}:invalid_invite", ERR_BAD_REQUEST));
    }
    api.send(
        Method::POST,
        &format!("/api/invites/{}", urlencoding::encode(&code)),
        &[],
        Some(json!({})),
    )
    .await
}

pub fn extract_invite_code(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    match trimmed.rsplit_once("/invite/") {
        Some((_, code)) => code.trim().to_string(),
        None => trimmed
            .rsplit('/')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_string(),
    }
}

#[tauri::command]
pub async fn omnidisc_create_dm(url: String, recipient_ids: Vec<String>) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::POST,
        "/api/users/@me/channels",
        &[],
        Some(json!({ "recipient_ids": recipient_ids })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_update_me(url: String, patch: Value) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::PATCH, "/api/users/@me", &[], Some(patch))
        .await
}

#[tauri::command]
pub async fn omnidisc_get_user(url: String, user_id: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, &format!("/api/users/{}", user_id), &[], None)
        .await
}

#[tauri::command]
pub async fn omnidisc_get_guild(url: String, guild_id: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, &format!("/api/guilds/{}", guild_id), &[], None)
        .await
}

#[tauri::command]
pub async fn omnidisc_get_me(url: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, "/api/users/@me", &[], None).await
}

// The argument list is the IPC contract the frontend calls by name; folding it
// into a struct would change the payload shape for no gain here.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn omnidisc_search(
    url: String,
    scope: String,
    scope_id: String,
    query: String,
    from: Option<String>,
    channel: Option<String>,
    has: Option<String>,
    before: Option<String>,
    after: Option<String>,
    limit: Option<u32>,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let path = match scope.as_str() {
        "guild" => format!("/api/guilds/{}/messages/search", scope_id),
        "channel" => format!("/api/channels/{}/messages/search", scope_id),
        _ => return Err(format!("{}:invalid_scope", ERR_BAD_REQUEST)),
    };
    let mut q: Vec<(&str, String)> = vec![("q", query)];
    if let Some(v) = opt(from) {
        q.push(("from", v));
    }
    if let Some(v) = opt(channel) {
        q.push(("in", v));
    }
    if let Some(v) = opt(has) {
        q.push(("has", v));
    }
    if let Some(v) = opt(before) {
        q.push(("before", v));
    }
    if let Some(v) = opt(after) {
        q.push(("after", v));
    }
    q.push(("limit", limit.unwrap_or(25).clamp(1, 100).to_string()));
    api.send(Method::GET, &path, &q, None).await
}

#[tauri::command]
pub async fn omnidisc_list_pins(url: String, channel_id: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::GET,
        &format!("/api/channels/{}/pins", channel_id),
        &[],
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_pin_message(
    url: String,
    channel_id: String,
    message_id: String,
    pinned: bool,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    let method = if pinned { Method::PUT } else { Method::DELETE };
    api.send_empty(
        method,
        &format!("/api/channels/{}/pins/{}", channel_id, message_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_list_relationships(url: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, "/api/users/@me/relationships", &[], None)
        .await
}

#[tauri::command]
pub async fn omnidisc_add_relationship(
    url: String,
    username: Option<String>,
    user_id: Option<String>,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    let mut body = json!({});
    if let Some(u) = opt(username) {
        body["username"] = Value::String(u);
    }
    if let Some(u) = opt(user_id) {
        body["user_id"] = Value::String(u);
    }
    if body.get("username").is_none() && body.get("user_id").is_none() {
        return Err(format!("{}:invalid_field", ERR_BAD_REQUEST));
    }
    api.send_empty(Method::POST, "/api/users/@me/relationships", Some(body))
        .await
}

#[tauri::command]
pub async fn omnidisc_accept_relationship(url: String, user_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::PUT,
        &format!("/api/users/@me/relationships/{}", user_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_remove_relationship(url: String, user_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/users/@me/relationships/{}", user_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_block_user(url: String, user_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::PUT,
        &format!("/api/users/@me/relationships/{}/block", user_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_list_notes(url: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, "/api/users/@me/notes", &[], None)
        .await
}

#[tauri::command]
pub async fn omnidisc_put_note(url: String, user_id: String, note: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::PUT,
        &format!("/api/users/@me/notes/{}", user_id),
        Some(json!({ "note": note })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_update_guild(
    url: String,
    guild_id: String,
    patch: Value,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::PATCH,
        &format!("/api/guilds/{}", guild_id),
        &[],
        Some(patch),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_delete_guild(url: String, guild_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(Method::DELETE, &format!("/api/guilds/{}", guild_id), None)
        .await
}

#[tauri::command]
pub async fn omnidisc_leave_guild(url: String, guild_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/users/@me/guilds/{}", guild_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_transfer_guild(
    url: String,
    guild_id: String,
    user_id: String,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::POST,
        &format!("/api/guilds/{}/transfer", guild_id),
        &[],
        Some(json!({ "user_id": user_id })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_create_role(
    url: String,
    guild_id: String,
    name: String,
    permissions: String,
    color: Option<u32>,
    hoist: bool,
    mentionable: bool,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let mut body = json!({
        "name": name,
        "permissions": permissions,
        "hoist": hoist,
        "mentionable": mentionable,
    });
    if let Some(c) = color {
        body["color"] = json!(c);
    }
    api.send(
        Method::POST,
        &format!("/api/guilds/{}/roles", guild_id),
        &[],
        Some(body),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_update_role(
    url: String,
    guild_id: String,
    role_id: String,
    patch: Value,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::PATCH,
        &format!("/api/guilds/{}/roles/{}", guild_id, role_id),
        &[],
        Some(patch),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_delete_role(
    url: String,
    guild_id: String,
    role_id: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/guilds/{}/roles/{}", guild_id, role_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_set_member_role(
    url: String,
    guild_id: String,
    user_id: String,
    role_id: String,
    granted: bool,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    let method = if granted { Method::PUT } else { Method::DELETE };
    api.send_empty(
        method,
        &format!(
            "/api/guilds/{}/members/{}/roles/{}",
            guild_id, user_id, role_id
        ),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_update_member(
    url: String,
    guild_id: String,
    user_id: String,
    patch: Value,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::PATCH,
        &format!("/api/guilds/{}/members/{}", guild_id, user_id),
        &[],
        Some(patch),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_kick_member(
    url: String,
    guild_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    let mut body = json!({});
    if let Some(r) = opt(reason) {
        body["reason"] = Value::String(r);
    }
    api.send_empty(
        Method::DELETE,
        &format!("/api/guilds/{}/members/{}", guild_id, user_id),
        Some(body),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_ban_member(
    url: String,
    guild_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let mut body = json!({});
    if let Some(r) = opt(reason) {
        body["reason"] = Value::String(r);
    }
    api.send(
        Method::PUT,
        &format!("/api/guilds/{}/bans/{}", guild_id, user_id),
        &[],
        Some(body),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_unban_member(
    url: String,
    guild_id: String,
    user_id: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/guilds/{}/bans/{}", guild_id, user_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_list_bans(url: String, guild_id: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::GET,
        &format!("/api/guilds/{}/bans", guild_id),
        &[],
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_audit_log(
    url: String,
    guild_id: String,
    action: Option<String>,
    actor_id: Option<String>,
    before: Option<String>,
    limit: Option<u32>,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    let mut q: Vec<(&str, String)> = Vec::new();
    if let Some(v) = opt(action) {
        q.push(("action", v));
    }
    if let Some(v) = opt(actor_id) {
        q.push(("actor_id", v));
    }
    if let Some(v) = opt(before) {
        q.push(("before", v));
    }
    q.push(("limit", limit.unwrap_or(50).clamp(1, 100).to_string()));
    api.send(
        Method::GET,
        &format!("/api/guilds/{}/audit-log", guild_id),
        &q,
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_update_channel(
    url: String,
    channel_id: String,
    patch: Value,
) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(
        Method::PATCH,
        &format!("/api/channels/{}", channel_id),
        &[],
        Some(patch),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_delete_channel(url: String, channel_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/channels/{}", channel_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_put_overwrite(
    url: String,
    channel_id: String,
    target_id: String,
    target_kind: String,
    allow: String,
    deny: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::PUT,
        &format!("/api/channels/{}/overwrites/{}", channel_id, target_id),
        Some(json!({ "target_kind": target_kind, "allow": allow, "deny": deny })),
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_delete_overwrite(
    url: String,
    channel_id: String,
    target_id: String,
) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/channels/{}/overwrites/{}", channel_id, target_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_list_sessions(url: String) -> Result<Value, String> {
    let api = Api::authed(&url)?;
    api.send(Method::GET, "/api/auth/sessions", &[], None).await
}

#[tauri::command]
pub async fn omnidisc_revoke_session(url: String, session_id: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(
        Method::DELETE,
        &format!("/api/auth/sessions/{}", session_id),
        None,
    )
    .await
}

#[tauri::command]
pub async fn omnidisc_revoke_other_sessions(url: String) -> Result<(), String> {
    let api = Api::authed(&url)?;
    api.send_empty(Method::DELETE, "/api/auth/sessions", None)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses_to_stable_codes() {
        assert_eq!(
            map_error(StatusCode::UNAUTHORIZED, "unauthorized"),
            "ERR_UNAUTHORIZED"
        );
        assert_eq!(
            map_error(StatusCode::FORBIDDEN, "registration_closed"),
            "ERR_FORBIDDEN:registration_closed"
        );
        assert_eq!(map_error(StatusCode::FORBIDDEN, ""), "ERR_FORBIDDEN");
        assert_eq!(
            map_error(StatusCode::NOT_FOUND, "not_found"),
            "ERR_NOT_FOUND"
        );
        assert_eq!(
            map_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            "ERR_RATE_LIMITED"
        );
        assert_eq!(
            map_error(StatusCode::BAD_REQUEST, "invalid_credentials"),
            "ERR_BAD_REQUEST:invalid_credentials"
        );
        assert_eq!(
            map_error(StatusCode::CONFLICT, "username_taken"),
            "ERR_BAD_REQUEST:username_taken"
        );
        assert_eq!(
            map_error(StatusCode::INTERNAL_SERVER_ERROR, "internal"),
            "ERR_SERVER"
        );
        assert_eq!(map_error(StatusCode::BAD_GATEWAY, ""), "ERR_SERVER");
    }

    /// `Url::parse` resolves `..`, so an id that walks up the path would reach a
    /// route the caller never wrote.
    #[test]
    fn ids_are_plain_identifiers_or_nothing() {
        assert_eq!(path_id("1234567890123456789"), Ok("1234567890123456789"));
        assert!(path_id("od-3f2a1b4c-1111-2222-3333-444455556666").is_ok());
        assert!(path_id("../../admin/instance").is_err());
        assert!(path_id("..").is_err());
        assert!(path_id("7/messages").is_err());
        assert!(path_id("7?limit=1").is_err());
        assert!(path_id("7#frag").is_err());
        assert!(path_id("7 8").is_err());
        assert!(path_id("").is_err());
        assert!(path_id(&"9".repeat(129)).is_err());
    }

    #[test]
    fn assembled_paths_stay_inside_the_api() {
        assert!(safe_path("/api/users/@me/devices"));
        assert!(safe_path(
            "/api/channels/17/messages/42/reactions/%F0%9F%91%8D/@me"
        ));
        assert!(!safe_path("/api/channels/../../admin"));
        assert!(!safe_path("/api/channels//messages"));
        assert!(!safe_path("api/instance"));
        assert!(!safe_path("/api/channels/7?x=1"));
        assert!(!safe_path("/api/channels/7\n/x"));
    }

    #[test]
    fn invite_code_is_extracted_from_links() {
        assert_eq!(extract_invite_code("abc123"), "abc123");
        assert_eq!(
            extract_invite_code("https://chat.example.org/invite/abc123"),
            "abc123"
        );
        assert_eq!(
            extract_invite_code("https://chat.example.org/invite/abc123/"),
            "abc123"
        );
        assert_eq!(extract_invite_code("  abc123 "), "abc123");
    }
}
