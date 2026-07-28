use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

#[derive(Clone)]
struct LcuClient {
    port: u16,
    token: String,
    region: Option<String>,
}

#[derive(Serialize)]
pub struct LeagueStatus {
    pub connected: bool,
    pub port: Option<u16>,
    pub region: Option<String>,
}

static CACHED_CLIENT: Lazy<Mutex<Option<LcuClient>>> = Lazy::new(|| Mutex::new(None));
static AUTO_ACCEPT: AtomicBool = AtomicBool::new(false);
static POLLER_STARTED: AtomicBool = AtomicBool::new(false);

fn extract_arg(cmdline: &str, key: &str) -> Option<String> {
    let needle = format!("--{}=", key);
    let start = cmdline.find(&needle)? + needle.len();
    let rest = &cmdline[start..];
    let value: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
        .collect();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

async fn read_process_command_lines() -> Result<String, String> {
    #[cfg(windows)]
    {
        let output = crate::core::process::command("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name='LeagueClientUx.exe'\" | Select-Object -ExpandProperty CommandLine",
            ])
            .output()
            .await
            .map_err(|e| format!("failed to query processes: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    #[cfg(not(windows))]
    {
        let output = tokio::process::Command::new("ps")
            .args(["-axo", "command"])
            .output()
            .await
            .map_err(|e| format!("failed to query processes: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

async fn discover_client() -> Option<LcuClient> {
    let listing = read_process_command_lines().await.ok()?;
    for line in listing.lines() {
        if !line.contains("LeagueClientUx") {
            continue;
        }
        let port = extract_arg(line, "app-port").and_then(|p| p.parse::<u16>().ok());
        let token = extract_arg(line, "remoting-auth-token");
        if let (Some(port), Some(token)) = (port, token) {
            let region = extract_arg(line, "region")
                .or_else(|| extract_arg(line, "rso_platform_id"));
            return Some(LcuClient {
                port,
                token,
                region,
            });
        }
    }
    None
}

async fn get_client() -> Result<LcuClient, String> {
    {
        let cached = CACHED_CLIENT.lock().await;
        if let Some(client) = cached.as_ref() {
            if lcu_reachable(client).await {
                return Ok(client.clone());
            }
        }
    }
    let discovered = discover_client()
        .await
        .ok_or_else(|| "league client not running".to_string())?;
    let mut cached = CACHED_CLIENT.lock().await;
    *cached = Some(discovered.clone());
    Ok(discovered)
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(10))
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())
}

async fn lcu_reachable(client: &LcuClient) -> bool {
    match lcu_get_raw(client, "/lol-gameflow/v1/gameflow-phase").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn lcu_get_raw(client: &LcuClient, path: &str) -> Result<Value, String> {
    let url = format!("https://127.0.0.1:{}{}", client.port, path);
    let resp = http_client()?
        .get(&url)
        .basic_auth("riot", Some(&client.token))
        .send()
        .await
        .map_err(|e| format!("lcu request failed: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("lcu returned {} for {}", status.as_u16(), path));
    }
    resp.json::<Value>()
        .await
        .map_err(|e| format!("invalid lcu response: {}", e))
}

async fn lcu_post_raw(client: &LcuClient, path: &str) -> Result<Value, String> {
    let url = format!("https://127.0.0.1:{}{}", client.port, path);
    let resp = http_client()?
        .post(&url)
        .basic_auth("riot", Some(&client.token))
        .header("content-length", "0")
        .send()
        .await
        .map_err(|e| format!("lcu request failed: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("lcu returned {} for {}", status.as_u16(), path));
    }
    Ok(resp.json::<Value>().await.unwrap_or(Value::Null))
}

fn league_enabled() -> bool {
    crate::storage::config::load_settings_standalone().league.enabled
}

fn ensure_enabled() -> Result<(), String> {
    if league_enabled() {
        Ok(())
    } else {
        Err("league menu is disabled in settings".to_string())
    }
}

#[tauri::command]
pub async fn league_status() -> LeagueStatus {
    if !league_enabled() {
        return LeagueStatus {
            connected: false,
            port: None,
            region: None,
        };
    }
    match get_client().await {
        Ok(client) => LeagueStatus {
            connected: true,
            port: Some(client.port),
            region: client.region,
        },
        Err(_) => LeagueStatus {
            connected: false,
            port: None,
            region: None,
        },
    }
}

#[tauri::command]
pub async fn league_get(path: String) -> Result<Value, String> {
    ensure_enabled()?;
    let allowed = path.starts_with("/lol-") || path.starts_with("/riotclient/");
    if !allowed || path.contains("..") {
        return Err(format!("path not allowed: {}", path));
    }
    let client = get_client().await?;
    lcu_get_raw(&client, &path).await
}

#[tauri::command]
pub async fn league_summoner() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_get_raw(&client, "/lol-summoner/v1/current-summoner").await
}

#[tauri::command]
pub async fn league_ranked() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_get_raw(&client, "/lol-ranked/v1/current-ranked-stats").await
}

#[tauri::command]
pub async fn league_gameflow() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_get_raw(&client, "/lol-gameflow/v1/gameflow-phase").await
}

#[tauri::command]
pub async fn league_match_history(beg_index: u32, end_index: u32) -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let summoner = lcu_get_raw(&client, "/lol-summoner/v1/current-summoner").await?;
    let puuid = summoner
        .get("puuid")
        .and_then(Value::as_str)
        .ok_or_else(|| "no puuid in summoner response".to_string())?;
    let path = format!(
        "/lol-match-history/v1/products/lol/{}/matches?begIndex={}&endIndex={}",
        puuid, beg_index, end_index
    );
    lcu_get_raw(&client, &path).await
}

#[tauri::command]
pub async fn league_accept_ready_check() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_post_raw(&client, "/lol-matchmaking/v1/ready-check/accept").await?;
    Ok(())
}

#[tauri::command]
pub async fn league_auto_accept_set(enabled: bool) -> Result<(), String> {
    ensure_enabled()?;
    AUTO_ACCEPT.store(enabled, Ordering::Relaxed);
    if enabled {
        spawn_poller();
    }
    tracing::info!("[league] auto-accept {}", if enabled { "on" } else { "off" });
    Ok(())
}

#[tauri::command]
pub fn league_auto_accept_get() -> bool {
    AUTO_ACCEPT.load(Ordering::Relaxed)
}

fn spawn_poller() {
    if POLLER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if !AUTO_ACCEPT.load(Ordering::Relaxed) || !league_enabled() {
                continue;
            }
            let client = match get_client().await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let phase = match lcu_get_raw(&client, "/lol-gameflow/v1/gameflow-phase").await {
                Ok(v) => v,
                Err(_) => continue,
            };
            if phase.as_str() == Some("ReadyCheck") {
                if let Err(e) =
                    lcu_post_raw(&client, "/lol-matchmaking/v1/ready-check/accept").await
                {
                    tracing::warn!("[league] auto-accept failed: {}", e);
                } else {
                    tracing::info!("[league] ready check accepted");
                }
            }
        }
    });
}

pub fn start_background() {
    let settings = crate::storage::config::load_settings_standalone();
    if settings.league.enabled && settings.league.auto_accept {
        AUTO_ACCEPT.store(true, Ordering::Relaxed);
        spawn_poller();
    }
}
