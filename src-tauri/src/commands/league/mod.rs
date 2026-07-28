pub mod analysis;
pub mod stats;

use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
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
static CS_HANDLED: Lazy<Mutex<HashSet<i64>>> = Lazy::new(|| Mutex::new(HashSet::new()));

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
    lcu_get_raw(client, "/lol-gameflow/v1/gameflow-phase")
        .await
        .is_ok()
}

async fn lcu_send(
    client: &LcuClient,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let url = format!("https://127.0.0.1:{}{}", client.port, path);
    let mut req = http_client()?
        .request(method, &url)
        .basic_auth("riot", Some(&client.token));
    req = match body {
        Some(b) => req.json(&b),
        None => req.header("content-length", "0"),
    };
    let resp = req
        .send()
        .await
        .map_err(|e| format!("lcu request failed: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let detail = resp
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v.get("message").and_then(Value::as_str).map(String::from))
            .unwrap_or_default();
        return Err(format!(
            "lcu returned {} for {}{}",
            status.as_u16(),
            path,
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {}", detail)
            }
        ));
    }
    Ok(resp.json::<Value>().await.unwrap_or(Value::Null))
}

async fn lcu_get_raw(client: &LcuClient, path: &str) -> Result<Value, String> {
    lcu_send(client, reqwest::Method::GET, path, None).await
}

async fn lcu_post_raw(client: &LcuClient, path: &str) -> Result<Value, String> {
    lcu_send(client, reqwest::Method::POST, path, None).await
}

fn league_settings() -> omniget_core::models::settings::LeagueSettings {
    crate::storage::config::load_settings_standalone().league
}

fn league_enabled() -> bool {
    league_settings().enabled
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
    spawn_poller();
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
pub async fn league_lobby_queues() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let queues = lcu_get_raw(&client, "/lol-game-queues/v1/queues").await?;
    let eligibility = lcu_send(
        &client,
        reqwest::Method::POST,
        "/lol-lobby/v2/eligibility/self",
        Some(json!({})),
    )
    .await
    .unwrap_or(Value::Null);

    let eligible: HashSet<i64> = eligibility
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|e| e.get("eligible").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|e| e.get("queueId").and_then(Value::as_i64))
                .collect()
        })
        .unwrap_or_default();

    let list: Vec<Value> = queues
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|q| {
                    let id = q.get("id").and_then(Value::as_i64).unwrap_or(-1);
                    let available = q
                        .get("queueAvailability")
                        .and_then(Value::as_str)
                        .map(|a| a == "Available")
                        .unwrap_or(false);
                    let pvp = q
                        .get("category")
                        .and_then(Value::as_str)
                        .map(|c| c == "PvP")
                        .unwrap_or(false);
                    available && pvp && eligible.contains(&id)
                })
                .map(|q| {
                    json!({
                        "id": q.get("id"),
                        "name": q.get("name"),
                        "shortName": q.get("shortName"),
                        "gameMode": q.get("gameMode"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Value::Array(list))
}

#[tauri::command]
pub async fn league_create_lobby(queue_id: i64) -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_send(
        &client,
        reqwest::Method::POST,
        "/lol-lobby/v2/lobby",
        Some(json!({ "queueId": queue_id })),
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn league_start_matchmaking() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_post_raw(&client, "/lol-lobby/v2/lobby/matchmaking/search").await?;
    Ok(())
}

#[tauri::command]
pub async fn league_stop_matchmaking() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_send(
        &client,
        reqwest::Method::DELETE,
        "/lol-lobby/v2/lobby/matchmaking/search",
        None,
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn league_leave_lobby() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_send(&client, reqwest::Method::DELETE, "/lol-lobby/v2/lobby", None).await?;
    Ok(())
}

#[tauri::command]
pub async fn league_play_again() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_post_raw(&client, "/lol-lobby/v2/play-again").await?;
    Ok(())
}

#[tauri::command]
pub async fn league_champ_select_session() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_get_raw(&client, "/lol-champ-select/v1/session").await
}

#[tauri::command]
pub async fn league_bench_swap(champion_id: i64) -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let path = format!("/lol-champ-select/v1/session/bench/swap/{}", champion_id);
    lcu_post_raw(&client, &path).await?;
    Ok(())
}

#[tauri::command]
pub async fn league_reroll() -> Result<(), String> {
    ensure_enabled()?;
    let client = get_client().await?;
    lcu_post_raw(&client, "/lol-champ-select/v1/session/my-selection/reroll").await?;
    Ok(())
}

#[tauri::command]
pub async fn league_live_game() -> Result<Value, String> {
    ensure_enabled()?;
    let base = "https://127.0.0.1:2999/liveclientdata";
    let http = http_client()?;
    let stats = http
        .get(format!("{}/gamestats", base))
        .send()
        .await
        .map_err(|e| format!("live client not reachable: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("invalid live client response: {}", e))?;
    let players = http
        .get(format!("{}/playerlist", base))
        .send()
        .await
        .ok();
    let players = match players {
        Some(r) => r.json::<Value>().await.unwrap_or(Value::Null),
        None => Value::Null,
    };
    let active = http
        .get(format!("{}/activeplayername", base))
        .send()
        .await
        .ok();
    let active = match active {
        Some(r) => r.json::<Value>().await.unwrap_or(Value::Null),
        None => Value::Null,
    };
    Ok(json!({ "stats": stats, "players": players, "activePlayer": active }))
}

fn player_identity(entry: &Value, is_ally: bool) -> Value {
    let game_name = entry
        .get("gameName")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| entry.get("summonerName").and_then(Value::as_str))
        .unwrap_or("");
    json!({
        "puuid": entry.get("puuid").and_then(Value::as_str).unwrap_or(""),
        "summonerId": entry.get("summonerId").and_then(Value::as_i64).unwrap_or(0),
        "gameName": game_name,
        "tagLine": entry.get("tagLine").and_then(Value::as_str).unwrap_or(""),
        "championId": entry.get("championId").and_then(Value::as_i64).unwrap_or(0),
        "cellId": entry.get("cellId").and_then(Value::as_i64),
        "isAlly": is_ally,
    })
}

#[tauri::command]
pub async fn league_game_players() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let phase = lcu_get_raw(&client, "/lol-gameflow/v1/gameflow-phase").await?;
    let phase = phase.as_str().unwrap_or("");

    let mut players: Vec<Value> = Vec::new();

    if phase == "ChampSelect" {
        let session = lcu_get_raw(&client, "/lol-champ-select/v1/session").await?;
        for (team_key, is_ally) in [("myTeam", true), ("theirTeam", false)] {
            if let Some(team) = session.get(team_key).and_then(Value::as_array) {
                for member in team {
                    players.push(player_identity(member, is_ally));
                }
            }
        }
    } else {
        let session = lcu_get_raw(&client, "/lol-gameflow/v1/session").await?;
        let my_puuid = lcu_get_raw(&client, "/lol-summoner/v1/current-summoner")
            .await
            .ok()
            .and_then(|s| {
                s.get("puuid").and_then(Value::as_str).map(String::from)
            })
            .unwrap_or_default();
        let game_data = session.get("gameData").cloned().unwrap_or(Value::Null);
        let team_one: Vec<Value> = game_data
            .get("teamOne")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let team_two: Vec<Value> = game_data
            .get("teamTwo")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let one_has_me = team_one.iter().any(|m| {
            m.get("puuid").and_then(Value::as_str) == Some(my_puuid.as_str())
        });
        for (team, mine) in [(team_one, one_has_me), (team_two, !one_has_me)] {
            for member in &team {
                players.push(player_identity(member, mine));
            }
        }
    }

    let mut resolved: Vec<Value> = Vec::new();
    let mut to_lookup: Vec<(usize, String, String)> = Vec::new();
    for (i, p) in players.iter().enumerate() {
        let puuid = p.get("puuid").and_then(Value::as_str).unwrap_or("");
        let name = p.get("gameName").and_then(Value::as_str).unwrap_or("");
        if puuid.is_empty() && name.contains('#') {
            let mut parts = name.splitn(2, '#');
            let game_name = parts.next().unwrap_or("").to_string();
            let tag = parts.next().unwrap_or("").to_string();
            to_lookup.push((i, game_name, tag));
        }
        resolved.push(p.clone());
    }
    if !to_lookup.is_empty() {
        let body: Vec<Value> = to_lookup
            .iter()
            .map(|(_, g, t)| json!({ "gameName": g, "tagLine": t }))
            .collect();
        if let Ok(found) = lcu_send(
            &client,
            reqwest::Method::POST,
            "/lol-summoner/v1/summoners/aliases",
            Some(Value::Array(body)),
        )
        .await
        {
            if let Some(arr) = found.as_array() {
                for (slot, (idx, g, _)) in to_lookup.iter().enumerate() {
                    let hit = arr.get(slot).or_else(|| {
                        arr.iter().find(|s| {
                            s.get("gameName").and_then(Value::as_str) == Some(g.as_str())
                        })
                    });
                    if let Some(s) = hit {
                        if let Some(puuid) = s.get("puuid").and_then(Value::as_str) {
                            resolved[*idx]["puuid"] = json!(puuid);
                        }
                    }
                }
            }
        }
    }

    let mut name_tasks = Vec::new();
    for (i, p) in resolved.iter().enumerate() {
        let puuid = p.get("puuid").and_then(Value::as_str).unwrap_or("").to_string();
        let has_name = p
            .get("gameName")
            .and_then(Value::as_str)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if puuid.is_empty() || has_name {
            continue;
        }
        let task_client = client.clone();
        name_tasks.push(tauri::async_runtime::spawn(async move {
            let path = format!("/lol-summoner/v2/summoners/puuid/{}", puuid);
            (i, lcu_get_raw(&task_client, &path).await.ok())
        }));
    }
    for task in name_tasks {
        if let Ok((i, Some(summoner))) = task.await {
            if let Some(name) = summoner.get("gameName").and_then(Value::as_str) {
                resolved[i]["gameName"] = json!(name);
            }
            if let Some(tag) = summoner.get("tagLine").and_then(Value::as_str) {
                resolved[i]["tagLine"] = json!(tag);
            }
            if let Some(level) = summoner.get("summonerLevel").and_then(Value::as_i64) {
                resolved[i]["summonerLevel"] = json!(level);
            }
            if let Some(icon) = summoner.get("profileIconId").and_then(Value::as_i64) {
                resolved[i]["profileIconId"] = json!(icon);
            }
        }
    }

    Ok(json!({ "phase": phase, "players": resolved }))
}

fn compute_history_stats(games: &[Value], puuid: &str) -> Value {
    let mut wins = 0usize;
    let mut kills = 0i64;
    let mut deaths = 0i64;
    let mut assists = 0i64;
    let mut streak_kind: Option<bool> = None;
    let mut streak_len = 0usize;
    let mut streak_done = false;
    let mut champ_games: std::collections::HashMap<i64, (usize, usize)> =
        std::collections::HashMap::new();

    for game in games {
        let participant = game
            .get("participants")
            .and_then(Value::as_array)
            .and_then(|arr| {
                if arr.len() == 1 {
                    arr.first()
                } else {
                    let identities = game.get("participantIdentities").and_then(Value::as_array);
                    let pid = identities.and_then(|ids| {
                        ids.iter()
                            .find(|i| {
                                i.get("player")
                                    .and_then(|p| p.get("puuid"))
                                    .and_then(Value::as_str)
                                    == Some(puuid)
                            })
                            .and_then(|i| i.get("participantId").and_then(Value::as_i64))
                    });
                    match pid {
                        Some(pid) => arr.iter().find(|p| {
                            p.get("participantId").and_then(Value::as_i64) == Some(pid)
                        }),
                        None => arr.first(),
                    }
                }
            });
        let participant = match participant {
            Some(p) => p,
            None => continue,
        };
        let stats = participant.get("stats").cloned().unwrap_or(Value::Null);
        let win = stats.get("win").and_then(Value::as_bool).unwrap_or(false);
        if win {
            wins += 1;
        }
        kills += stats.get("kills").and_then(Value::as_i64).unwrap_or(0);
        deaths += stats.get("deaths").and_then(Value::as_i64).unwrap_or(0);
        assists += stats.get("assists").and_then(Value::as_i64).unwrap_or(0);
        if !streak_done {
            match streak_kind {
                None => {
                    streak_kind = Some(win);
                    streak_len = 1;
                }
                Some(k) if k == win => streak_len += 1,
                Some(_) => streak_done = true,
            }
        }
        let champ = participant
            .get("championId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if champ > 0 {
            let e = champ_games.entry(champ).or_insert((0, 0));
            e.0 += 1;
            if win {
                e.1 += 1;
            }
        }
    }

    let total = games.len();
    let kda = if deaths > 0 {
        (kills + assists) as f64 / deaths as f64
    } else {
        (kills + assists) as f64
    };
    let mut top: Vec<(i64, usize, usize)> = champ_games
        .into_iter()
        .map(|(id, (g, w))| (id, g, w))
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    top.truncate(3);

    let winrate = if total > 0 {
        (wins as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };

    let mut insights: Vec<&str> = Vec::new();
    if total >= 5 {
        if let Some(kind) = streak_kind {
            if streak_len >= 3 {
                insights.push(if kind { "hot_streak" } else { "cold_streak" });
            }
        }
        if total >= 10 && winrate >= 60.0 {
            insights.push("high_winrate");
        }
        if total >= 10 && winrate <= 40.0 {
            insights.push("low_winrate");
        }
        if let Some((_, g, _)) = top.first() {
            if *g * 2 >= total {
                insights.push("one_trick");
            }
        }
        if kda >= 4.0 {
            insights.push("high_kda");
        }
        if kda <= 1.5 && deaths > 0 {
            insights.push("low_kda");
        }
    }

    json!({
        "games": total,
        "wins": wins,
        "winrate": winrate,
        "kda": (kda * 10.0).round() / 10.0,
        "streak": { "win": streak_kind.unwrap_or(false), "length": streak_len },
        "topChampions": top.iter().map(|(id, g, w)| json!({ "championId": id, "games": g, "wins": w })).collect::<Vec<_>>(),
        "insights": insights,
    })
}

#[tauri::command]
pub async fn league_player_report(puuid: String) -> Result<Value, String> {
    ensure_enabled()?;
    if puuid.is_empty() {
        return Err("empty puuid".to_string());
    }
    let client = get_client().await?;

    let ranked = lcu_get_raw(&client, &format!("/lol-ranked/v1/ranked-stats/{}", puuid))
        .await
        .unwrap_or(Value::Null);
    let solo = ranked
        .get("queueMap")
        .and_then(|q| q.get("RANKED_SOLO_5x5"))
        .cloned()
        .unwrap_or(Value::Null);
    let flex = ranked
        .get("queueMap")
        .and_then(|q| q.get("RANKED_FLEX_SR"))
        .cloned()
        .unwrap_or(Value::Null);

    let history = lcu_get_raw(
        &client,
        &format!(
            "/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=19",
            puuid
        ),
    )
    .await;
    let mut history_ids: Vec<i64> = Vec::new();
    let (stats, private_profile) = match history {
        Ok(h) => {
            let games: Vec<Value> = h
                .get("games")
                .and_then(|g| g.get("games"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            history_ids = games
                .iter()
                .filter_map(|g| g.get("gameId").and_then(Value::as_i64))
                .collect();
            (compute_history_stats(&games, &puuid), games.is_empty())
        }
        Err(_) => (json!({ "games": 0, "insights": [] }), true),
    };

    // Impact needs team totals, which only the full game detail carries, so it
    // is computed over a small sample instead of the whole history.
    let recent_ids: Vec<i64> = history_ids.into_iter().take(5).collect();
    let mut impact_tasks = Vec::new();
    for game_id in recent_ids {
        let task_client = client.clone();
        let task_puuid = puuid.clone();
        impact_tasks.push(tauri::async_runtime::spawn(async move {
            let detail = lcu_get_raw(
                &task_client,
                &format!("/lol-match-history/v1/games/{}", game_id),
            )
            .await
            .ok()?;
            impact_from_detail(&detail, &task_puuid)
        }));
    }
    let mut impacts: Vec<f64> = Vec::new();
    for task in impact_tasks {
        if let Ok(Some(score)) = task.await {
            impacts.push(score);
        }
    }
    let impact = if impacts.is_empty() {
        Value::Null
    } else {
        json!(((impacts.iter().sum::<f64>() / impacts.len() as f64) * 10.0).round() / 10.0)
    };

    let mastery = lcu_get_raw(
        &client,
        &format!("/lol-champion-mastery/v1/{}/champion-mastery", puuid),
    )
    .await
    .ok()
    .and_then(|m| {
        m.as_array().map(|arr| {
            arr.iter()
                .take(3)
                .map(|c| {
                    json!({
                        "championId": c.get("championId"),
                        "championLevel": c.get("championLevel"),
                        "championPoints": c.get("championPoints"),
                    })
                })
                .collect::<Vec<_>>()
        })
    })
    .unwrap_or_default();

    Ok(json!({
        "puuid": puuid,
        "solo": solo,
        "flex": flex,
        "stats": stats,
        "mastery": mastery,
        "impact": impact,
        "impactGames": impacts.len(),
        "privateProfile": private_profile,
    }))
}

/// Extracts rank plus, when trustworthy, the season win/loss record.
///
/// For anyone other than the local player the client reports `losses = 0` and a
/// career-long `wins` total, so a non-zero win count with zero losses is treated
/// as "hidden" instead of a perfect record.
fn ranked_entry_parts(entry: &Value) -> (Option<f64>, u32, u32) {
    let tier = entry.get("tier").and_then(Value::as_str).unwrap_or("");
    let division = entry.get("division").and_then(Value::as_str).unwrap_or("");
    let lp = entry
        .get("leaguePoints")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0) as u32;
    let wins = entry.get("wins").and_then(Value::as_i64).unwrap_or(0).max(0) as u32;
    let losses = entry
        .get("losses")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0) as u32;
    let rank = stats::rank_rating(tier, division, lp);
    if losses == 0 && wins > 0 {
        return (rank, 0, 0);
    }
    (rank, wins, losses)
}

/// Full pre-game analysis: per-player strength, team win probability and the
/// premade groups inferred from overlapping match histories.
#[tauri::command]
pub async fn league_match_analysis() -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let roster = league_game_players().await?;
    let players: Vec<Value> = roster
        .get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if players.is_empty() {
        return Err("no players in the current game".to_string());
    }

    let mut tasks = Vec::new();
    for (index, player) in players.iter().enumerate() {
        let puuid = player
            .get("puuid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let task_client = client.clone();
        tasks.push(tauri::async_runtime::spawn(async move {
            if puuid.is_empty() {
                return (index, Value::Null, Vec::new());
            }
            let ranked = lcu_get_raw(
                &task_client,
                &format!("/lol-ranked/v1/ranked-stats/{}", puuid),
            )
            .await
            .unwrap_or(Value::Null);
            let history = lcu_get_raw(
                &task_client,
                &format!(
                    "/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=19",
                    puuid
                ),
            )
            .await
            .unwrap_or(Value::Null);
            let game_ids: Vec<i64> = history
                .get("games")
                .and_then(|g| g.get("games"))
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|g| g.get("gameId").and_then(Value::as_i64))
                        .collect()
                })
                .unwrap_or_default();
            (index, json!({ "ranked": ranked, "history": history }), game_ids)
        }));
    }

    let mut fetched: Vec<Value> = vec![Value::Null; players.len()];
    let mut histories: Vec<Vec<i64>> = vec![Vec::new(); players.len()];
    for task in tasks {
        if let Ok((index, data, ids)) = task.await {
            fetched[index] = data;
            histories[index] = ids;
        }
    }

    let mut ally_strengths = Vec::new();
    let mut enemy_strengths = Vec::new();
    let mut detail: Vec<Value> = Vec::new();

    for (index, player) in players.iter().enumerate() {
        let is_ally = player.get("isAlly").and_then(Value::as_bool).unwrap_or(false);
        let data = &fetched[index];
        let solo = data
            .get("ranked")
            .and_then(|r| r.get("queueMap"))
            .and_then(|q| q.get("RANKED_SOLO_5x5"))
            .cloned()
            .unwrap_or(Value::Null);
        let (rank, season_wins, season_losses) = if solo.is_null() {
            (None, 0, 0)
        } else {
            ranked_entry_parts(&solo)
        };

        let games: Vec<Value> = data
            .get("history")
            .and_then(|h| h.get("games"))
            .and_then(|g| g.get("games"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut recent_wins = 0u32;
        let mut recent_losses = 0u32;
        for game in &games {
            let win = game
                .get("participants")
                .and_then(Value::as_array)
                .and_then(|p| p.first())
                .and_then(|p| p.get("stats"))
                .and_then(|s| s.get("win"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if win {
                recent_wins += 1;
            } else {
                recent_losses += 1;
            }
        }

        let strength =
            stats::player_strength(rank, season_wins, season_losses, recent_wins, recent_losses);
        if is_ally {
            ally_strengths.push(strength);
        } else {
            enemy_strengths.push(strength);
        }

        let season_shrunk =
            stats::shrunk_winrate(season_wins, season_losses, stats::PRIOR_GAMES);
        let (wilson_low, wilson_high) =
            stats::wilson_interval(season_wins, season_losses, stats::Z_90);

        detail.push(json!({
            "puuid": player.get("puuid"),
            "isAlly": is_ally,
            "rating": strength.rating.round(),
            "sigma": strength.sigma.round(),
            "known": strength.known,
            "rankRating": rank.map(|r| r.round()),
            "seasonWins": season_wins,
            "seasonLosses": season_losses,
            "seasonWinrate": if season_wins + season_losses > 0 {
                json!(((season_wins as f64 / (season_wins + season_losses) as f64) * 100.0).round())
            } else { Value::Null },
            "seasonAvailable": season_wins + season_losses > 0,
            "recentWinrate": if recent_wins + recent_losses > 0 {
                json!(((recent_wins as f64 / (recent_wins + recent_losses) as f64) * 100.0).round())
            } else { Value::Null },
            "shrunkWinrate": (season_shrunk * 1000.0).round() / 10.0,
            "wilsonLow": (wilson_low * 1000.0).round() / 10.0,
            "wilsonHigh": (wilson_high * 1000.0).round() / 10.0,
            "recentWins": recent_wins,
            "recentGames": recent_wins + recent_losses,
        }));
    }

    let win = stats::team_win_probability(&ally_strengths, &enemy_strengths);

    // Two players who keep appearing in the same games are queueing together.
    let mut pairs: Vec<(usize, usize, u32)> = Vec::new();
    for i in 0..players.len() {
        for j in (i + 1)..players.len() {
            let a: HashSet<i64> = histories[i].iter().copied().collect();
            let shared = histories[j].iter().filter(|id| a.contains(id)).count() as u32;
            if shared > 0 {
                pairs.push((i, j, shared));
            }
        }
    }
    let groups = stats::premade_groups(players.len(), &pairs, 3);
    let premades: Vec<Value> = groups
        .iter()
        .enumerate()
        .map(|(gi, group)| {
            json!({
                "label": char::from(b'A' + (gi.min(25) as u8)).to_string(),
                "puuids": group
                    .iter()
                    .filter_map(|i| players[*i].get("puuid").cloned())
                    .collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(json!({
        "winProbability": (win.probability * 1000.0).round() / 10.0,
        "winLow": (win.low * 1000.0).round() / 10.0,
        "winHigh": (win.high * 1000.0).round() / 10.0,
        "ratingGap": win.rating_gap.round(),
        "knownPlayers": win.known_players,
        "totalPlayers": win.total_players,
        "players": detail,
        "premades": premades,
    }))
}

/// Live economy snapshot: gold carried in items, CS and level for every player,
/// plus the difference against the opponent in the same position.
#[tauri::command]
pub async fn league_live_metrics() -> Result<Value, String> {
    ensure_enabled()?;
    let http = http_client()?;
    let base = "https://127.0.0.1:2999/liveclientdata";
    let stats_value = http
        .get(format!("{}/gamestats", base))
        .send()
        .await
        .map_err(|e| format!("live client not reachable: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("invalid live client response: {}", e))?;
    let players = http
        .get(format!("{}/playerlist", base))
        .send()
        .await
        .map_err(|e| format!("live client not reachable: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("invalid live client response: {}", e))?;
    let active = http
        .get(format!("{}/activeplayername", base))
        .send()
        .await
        .ok();
    let active_name = match active {
        Some(r) => r
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        None => String::new(),
    };

    let seconds = stats_value
        .get("gameTime")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let list = players.as_array().cloned().unwrap_or_default();

    let mut rows: Vec<Value> = Vec::new();
    for p in &list {
        let scores = p.get("scores").cloned().unwrap_or(Value::Null);
        let cs = scores
            .get("creepScore")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let prices: Vec<(u32, u32)> = p
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|i| {
                        (
                            i.get("price").and_then(Value::as_i64).unwrap_or(0).max(0) as u32,
                            i.get("count").and_then(Value::as_i64).unwrap_or(1).max(0) as u32,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let item_gold = stats::items_gold(&prices);
        let riot_id = p
            .get("riotId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        rows.push(json!({
            "riotId": riot_id,
            "championName": p.get("championName"),
            "rawChampionName": p.get("rawChampionName"),
            "team": p.get("team"),
            "position": p.get("position"),
            "level": p.get("level"),
            "isDead": p.get("isDead"),
            "respawnTimer": p.get("respawnTimer"),
            "kills": scores.get("kills"),
            "deaths": scores.get("deaths"),
            "assists": scores.get("assists"),
            "cs": cs,
            "csPerMin": (stats::per_minute(cs, seconds) * 10.0).round() / 10.0,
            "visionScore": scores.get("wardScore"),
            "itemGold": item_gold,
            "goldPerMin": (stats::per_minute(item_gold as f64, seconds)).round(),
            "kda": (stats::kda(
                scores.get("kills").and_then(Value::as_i64).unwrap_or(0).max(0) as u32,
                scores.get("deaths").and_then(Value::as_i64).unwrap_or(0).max(0) as u32,
                scores.get("assists").and_then(Value::as_i64).unwrap_or(0).max(0) as u32,
            ) * 100.0).round() / 100.0,
            "isSelf": !active_name.is_empty() && riot_id == active_name,
        }));
    }

    // Match each player against the opponent holding the same position.
    let mut enriched: Vec<Value> = Vec::new();
    for row in &rows {
        let position = row.get("position").and_then(Value::as_str).unwrap_or("");
        let team = row.get("team").and_then(Value::as_str).unwrap_or("");
        let opponent = rows.iter().find(|other| {
            other.get("position").and_then(Value::as_str) == Some(position)
                && other.get("team").and_then(Value::as_str) != Some(team)
                && !position.is_empty()
        });
        let mut row = row.clone();
        if let Some(o) = opponent {
            let gold = row.get("itemGold").and_then(Value::as_i64).unwrap_or(0);
            let ogold = o.get("itemGold").and_then(Value::as_i64).unwrap_or(0);
            let cs = row.get("cs").and_then(Value::as_f64).unwrap_or(0.0);
            let ocs = o.get("cs").and_then(Value::as_f64).unwrap_or(0.0);
            let level = row.get("level").and_then(Value::as_i64).unwrap_or(0);
            let olevel = o.get("level").and_then(Value::as_i64).unwrap_or(0);
            row["goldDiff"] = json!(gold - ogold);
            row["csDiff"] = json!(cs - ocs);
            row["levelDiff"] = json!(level - olevel);
            row["opponent"] = o.get("championName").cloned().unwrap_or(Value::Null);
        }
        enriched.push(row);
    }

    let team_gold = |team: &str| -> i64 {
        enriched
            .iter()
            .filter(|r| r.get("team").and_then(Value::as_str) == Some(team))
            .filter_map(|r| r.get("itemGold").and_then(Value::as_i64))
            .sum()
    };

    Ok(json!({
        "gameTime": seconds.round(),
        "players": enriched,
        "teamGold": { "ORDER": team_gold("ORDER"), "CHAOS": team_gold("CHAOS") },
        "teamGoldDiff": team_gold("ORDER") - team_gold("CHAOS"),
    }))
}

/// Computes the composite impact score for one player in one finished game.
fn impact_from_detail(detail: &Value, puuid: &str) -> Option<f64> {
    let identities = detail.get("participantIdentities")?.as_array()?;
    let participants = detail.get("participants")?.as_array()?;
    let pid = identities
        .iter()
        .find(|i| {
            i.get("player")
                .and_then(|p| p.get("puuid"))
                .and_then(Value::as_str)
                == Some(puuid)
        })
        .and_then(|i| i.get("participantId").and_then(Value::as_i64))?;
    let me = participants
        .iter()
        .find(|p| p.get("participantId").and_then(Value::as_i64) == Some(pid))?;
    let team_id = me.get("teamId").and_then(Value::as_i64)?;
    let team: Vec<&Value> = participants
        .iter()
        .filter(|p| p.get("teamId").and_then(Value::as_i64) == Some(team_id))
        .collect();
    if team.is_empty() {
        return None;
    }

    let stat = |p: &Value, key: &str| -> f64 {
        p.get("stats")
            .and_then(|s| s.get(key))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    let sum = |key: &str| -> f64 { team.iter().map(|p| stat(p, key)).sum() };
    let count = |p: &Value, key: &str| -> u32 { stat(p, key).max(0.0) as u32 };

    let input = stats::ImpactInput {
        kills: count(me, "kills"),
        deaths: count(me, "deaths"),
        assists: count(me, "assists"),
        team_kills: sum("kills").max(0.0) as u32,
        damage_to_champions: stat(me, "totalDamageDealtToChampions"),
        team_damage: sum("totalDamageDealtToChampions"),
        damage_taken: stat(me, "totalDamageTaken"),
        team_damage_taken: sum("totalDamageTaken"),
        gold: stat(me, "goldEarned"),
        team_gold: sum("goldEarned"),
        cs: stat(me, "totalMinionsKilled") + stat(me, "neutralMinionsKilled"),
        vision_score: stat(me, "visionScore"),
        team_vision: sum("visionScore"),
        duration_seconds: detail
            .get("gameDuration")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        team_size: team.len(),
    };
    Some(stats::impact_score(&input))
}

/// Aggregates a player's champion records the way a profile page shows them.
fn champion_records_from_history(games: &[Value]) -> Vec<analysis::ChampionRecord> {
    let mut map: std::collections::HashMap<i64, analysis::ChampionRecord> =
        std::collections::HashMap::new();
    for game in games {
        let participant = match game
            .get("participants")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
        {
            Some(p) => p,
            None => continue,
        };
        let champion_id = participant
            .get("championId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if champion_id <= 0 {
            continue;
        }
        let s = participant.get("stats").cloned().unwrap_or(Value::Null);
        let num = |key: &str| s.get(key).and_then(Value::as_i64).unwrap_or(0).max(0) as u32;
        let entry = map
            .entry(champion_id)
            .or_insert_with(|| analysis::ChampionRecord {
                champion_id,
                ..Default::default()
            });
        entry.games += 1;
        if s.get("win").and_then(Value::as_bool).unwrap_or(false) {
            entry.wins += 1;
        }
        entry.kills += num("kills");
        entry.deaths += num("deaths");
        entry.assists += num("assists");
        entry.cs += (num("totalMinionsKilled") + num("neutralMinionsKilled")) as f64;
        entry.seconds += game
            .get("gameDuration")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
    }
    analysis::rank_champion_records(map.into_values().collect(), 2)
}

fn champion_records_json(records: &[analysis::ChampionRecord]) -> Vec<Value> {
    records
        .iter()
        .map(|r| {
            json!({
                "championId": r.champion_id,
                "games": r.games,
                "wins": r.wins,
                "winrate": (r.winrate() * 1000.0).round() / 10.0,
                "kda": (r.kda() * 100.0).round() / 100.0,
                "csPerMin": (r.cs_per_min() * 10.0).round() / 10.0,
                "kills": r.kills,
                "deaths": r.deaths,
                "assists": r.assists,
            })
        })
        .collect()
}

/// Resolves a Riot ID to a full profile: rank, level, champion records and
/// mastery, in the shape a profile page needs.
#[tauri::command]
pub async fn league_search_player(game_name: String, tag_line: String) -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let name = game_name.trim().to_string();
    let tag = tag_line.trim().trim_start_matches('#').to_string();
    if name.is_empty() {
        return Err("empty name".to_string());
    }

    let found = lcu_send(
        &client,
        reqwest::Method::POST,
        "/lol-summoner/v1/summoners/aliases",
        Some(json!([{ "gameName": name, "tagLine": tag }])),
    )
    .await?;
    let summoner = found
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "player not found".to_string())?;
    let puuid = summoner
        .get("puuid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if puuid.is_empty() {
        return Err("player not found".to_string());
    }

    let profile = league_player_report(puuid.clone()).await?;
    let history = lcu_get_raw(
        &client,
        &format!(
            "/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=19",
            puuid
        ),
    )
    .await
    .unwrap_or(Value::Null);
    let games: Vec<Value> = history
        .get("games")
        .and_then(|g| g.get("games"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let records = champion_records_from_history(&games);

    Ok(json!({
        "summoner": summoner,
        "report": profile,
        "champions": champion_records_json(&records),
        "games": games,
    }))
}

/// Teammates the player wins the most with, ranked by shrunk win rate.
#[tauri::command]
pub async fn league_duos(sample: Option<u32>) -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let me = lcu_get_raw(&client, "/lol-summoner/v1/current-summoner").await?;
    let my_puuid = me
        .get("puuid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if my_puuid.is_empty() {
        return Err("no puuid".to_string());
    }
    let count = sample.unwrap_or(20).clamp(5, 50);
    let history = lcu_get_raw(
        &client,
        &format!(
            "/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex={}",
            my_puuid,
            count.saturating_sub(1)
        ),
    )
    .await?;
    let games: Vec<Value> = history
        .get("games")
        .and_then(|g| g.get("games"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut tasks = Vec::new();
    for game in &games {
        let game_id = match game.get("gameId").and_then(Value::as_i64) {
            Some(id) => id,
            None => continue,
        };
        let task_client = client.clone();
        tasks.push(tauri::async_runtime::spawn(async move {
            lcu_get_raw(
                &task_client,
                &format!("/lol-match-history/v1/games/{}", game_id),
            )
            .await
            .ok()
        }));
    }

    let mut tally: std::collections::HashMap<String, (u32, u32)> =
        std::collections::HashMap::new();
    let mut names: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    let mut analysed = 0u32;

    for task in tasks {
        let detail = match task.await {
            Ok(Some(d)) => d,
            _ => continue,
        };
        let identities: Vec<Value> = detail
            .get("participantIdentities")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let participants: Vec<Value> = detail
            .get("participants")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if identities.len() < 2 || participants.is_empty() {
            continue;
        }
        let pid_of = |puuid: &str| -> Option<i64> {
            identities
                .iter()
                .find(|i| {
                    i.get("player")
                        .and_then(|p| p.get("puuid"))
                        .and_then(Value::as_str)
                        == Some(puuid)
                })
                .and_then(|i| i.get("participantId").and_then(Value::as_i64))
        };
        let my_pid = match pid_of(&my_puuid) {
            Some(p) => p,
            None => continue,
        };
        let team_of = |pid: i64| -> Option<i64> {
            participants
                .iter()
                .find(|p| p.get("participantId").and_then(Value::as_i64) == Some(pid))
                .and_then(|p| p.get("teamId").and_then(Value::as_i64))
        };
        let my_team = match team_of(my_pid) {
            Some(t) => t,
            None => continue,
        };
        let won = participants
            .iter()
            .find(|p| p.get("participantId").and_then(Value::as_i64) == Some(my_pid))
            .and_then(|p| p.get("stats"))
            .and_then(|s| s.get("win"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        analysed += 1;

        for identity in &identities {
            let player = identity.get("player").cloned().unwrap_or(Value::Null);
            let puuid = player.get("puuid").and_then(Value::as_str).unwrap_or("");
            if puuid.is_empty() || puuid == my_puuid {
                continue;
            }
            let pid = identity
                .get("participantId")
                .and_then(Value::as_i64)
                .unwrap_or(-1);
            if team_of(pid) != Some(my_team) {
                continue;
            }
            let entry = tally.entry(puuid.to_string()).or_insert((0, 0));
            entry.0 += 1;
            if won {
                entry.1 += 1;
            }
            names.entry(puuid.to_string()).or_insert(player);
        }
    }

    let duos: Vec<analysis::DuoRecord> = tally
        .iter()
        .map(|(puuid, (games, wins))| analysis::DuoRecord {
            puuid: puuid.clone(),
            games: *games,
            wins: *wins,
        })
        .collect();
    let ranked = analysis::rank_duos(duos, 2);

    let list: Vec<Value> = ranked
        .iter()
        .take(15)
        .map(|d| {
            let player = names.get(&d.puuid).cloned().unwrap_or(Value::Null);
            json!({
                "puuid": d.puuid,
                "gameName": player.get("gameName").or_else(|| player.get("summonerName")),
                "tagLine": player.get("tagLine"),
                "games": d.games,
                "wins": d.wins,
                "winrate": (d.winrate() * 1000.0).round() / 10.0,
                "score": (d.score() * 1000.0).round() / 10.0,
            })
        })
        .collect();

    Ok(json!({ "analysedGames": analysed, "duos": list }))
}

/// Early-game map reading for a jungler, from match timelines.
#[tauri::command]
pub async fn league_jungle_report(puuid: String, sample: Option<u32>) -> Result<Value, String> {
    ensure_enabled()?;
    if puuid.is_empty() {
        return Err("empty puuid".to_string());
    }
    let client = get_client().await?;
    let count = sample.unwrap_or(8).clamp(1, 15);
    let history = lcu_get_raw(
        &client,
        &format!(
            "/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex={}",
            puuid,
            count.saturating_sub(1)
        ),
    )
    .await?;
    let games: Vec<Value> = history
        .get("games")
        .and_then(|g| g.get("games"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut tasks = Vec::new();
    for game in &games {
        // Only Summoner's Rift games carry a meaningful jungle path.
        if game.get("mapId").and_then(Value::as_i64) != Some(11) {
            continue;
        }
        let game_id = match game.get("gameId").and_then(Value::as_i64) {
            Some(id) => id,
            None => continue,
        };
        let task_client = client.clone();
        tasks.push(tauri::async_runtime::spawn(async move {
            let detail = lcu_get_raw(
                &task_client,
                &format!("/lol-match-history/v1/games/{}", game_id),
            )
            .await
            .ok();
            let timeline = lcu_get_raw(
                &task_client,
                &format!("/lol-match-history/v1/game-timelines/{}", game_id),
            )
            .await
            .ok();
            (detail, timeline)
        }));
    }

    let mut weights = analysis::ZoneWeights::default();
    let mut analysed = 0u32;
    let mut invades = 0u32;
    let mut level3_ganks = 0u32;
    let mut first_camps: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    let mut objectives: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    for task in tasks {
        let (detail, timeline) = match task.await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let (detail, timeline) = match (detail, timeline) {
            (Some(d), Some(t)) => (d, t),
            _ => continue,
        };
        let identities: Vec<Value> = detail
            .get("participantIdentities")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let pid = identities
            .iter()
            .find(|i| {
                i.get("player")
                    .and_then(|p| p.get("puuid"))
                    .and_then(Value::as_str)
                    == Some(puuid.as_str())
            })
            .and_then(|i| i.get("participantId").and_then(Value::as_i64));
        let pid = match pid {
            Some(p) => p,
            None => continue,
        };
        let team_id = detail
            .get("participants")
            .and_then(Value::as_array)
            .and_then(|arr| {
                arr.iter()
                    .find(|p| p.get("participantId").and_then(Value::as_i64) == Some(pid))
            })
            .and_then(|p| p.get("teamId").and_then(Value::as_i64))
            .unwrap_or(100);
        let on_blue_side = team_id == 100;

        let frames: Vec<Value> = timeline
            .get("frames")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if frames.is_empty() {
            continue;
        }
        analysed += 1;

        let frame_participant = |frame: &Value| -> Option<Value> {
            frame
                .get("participantFrames")
                .and_then(Value::as_object)
                .and_then(|obj| {
                    obj.values()
                        .find(|v| v.get("participantId").and_then(Value::as_i64) == Some(pid))
                        .cloned()
                })
        };

        for frame in frames.iter().take(analysis::EARLY_MINUTES + 1).skip(1) {
            if let Some(pf) = frame_participant(frame) {
                if let Some(pos) = pf.get("position") {
                    let x = pos.get("x").and_then(Value::as_f64).unwrap_or(-1.0);
                    let y = pos.get("y").and_then(Value::as_f64).unwrap_or(-1.0);
                    if x >= 0.0 && y >= 0.0 {
                        weights.add(analysis::classify_zone(x, y), 1.0);
                    }
                }
            }
        }

        if let Some(pf) = frames.get(1).and_then(frame_participant) {
            if let Some(pos) = pf.get("position") {
                let x = pos.get("x").and_then(Value::as_f64).unwrap_or(-1.0);
                let y = pos.get("y").and_then(Value::as_f64).unwrap_or(-1.0);
                if x >= 0.0 && y >= 0.0 {
                    let (camp, camp_blue) = analysis::nearest_camp(x, y);
                    *first_camps.entry(camp.to_string()).or_insert(0) += 1;
                    if analysis::is_invade(camp_blue, on_blue_side) {
                        invades += 1;
                    }
                }
            }
        }

        let early_ms = (analysis::EARLY_MINUTES as i64) * 60_000;
        let mut acted_early = false;
        for frame in &frames {
            for event in frame
                .get("events")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let ts = event.get("timestamp").and_then(Value::as_i64).unwrap_or(0);
                let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
                if kind == "CHAMPION_KILL" && ts <= early_ms {
                    let involved = event.get("killerId").and_then(Value::as_i64) == Some(pid)
                        || event
                            .get("assistingParticipantIds")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().any(|v| v.as_i64() == Some(pid)))
                            .unwrap_or(false);
                    if involved {
                        if ts <= 180_000 {
                            acted_early = true;
                        }
                        if let Some(pos) = event.get("position") {
                            let x = pos.get("x").and_then(Value::as_f64).unwrap_or(-1.0);
                            let y = pos.get("y").and_then(Value::as_f64).unwrap_or(-1.0);
                            if x >= 0.0 && y >= 0.0 {
                                weights.add(
                                    analysis::classify_zone(x, y),
                                    analysis::KILL_WEIGHT,
                                );
                            }
                        }
                    }
                }
                if kind == "ELITE_MONSTER_KILL"
                    && event.get("killerId").and_then(Value::as_i64) == Some(pid)
                {
                    let monster = event
                        .get("monsterType")
                        .and_then(Value::as_str)
                        .unwrap_or("UNKNOWN");
                    *objectives.entry(monster.to_string()).or_insert(0) += 1;
                }
            }
        }

        if let Some(pf3) = frames.get(3).and_then(frame_participant) {
            let cs = (pf3.get("minionsKilled").and_then(Value::as_i64).unwrap_or(0)
                + pf3
                    .get("jungleMinionsKilled")
                    .and_then(Value::as_i64)
                    .unwrap_or(0))
            .max(0) as u32;
            let level = pf3.get("level").and_then(Value::as_i64).unwrap_or(0).max(0) as u32;
            if analysis::looks_like_level3_gank(cs, level, acted_early) {
                level3_ganks += 1;
            }
        }
    }

    let shares = weights.shares();
    Ok(json!({
        "analysedGames": analysed,
        "zones": {
            "top": (shares.top * 1000.0).round() / 10.0,
            "mid": (shares.mid * 1000.0).round() / 10.0,
            "bot": (shares.bot * 1000.0).round() / 10.0,
        },
        "preference": weights.preference(),
        "invadeRate": if analysed > 0 { ((invades as f64 / analysed as f64) * 1000.0).round() / 10.0 } else { 0.0 },
        "level3GankRate": if analysed > 0 { ((level3_ganks as f64 / analysed as f64) * 1000.0).round() / 10.0 } else { 0.0 },
        "firstCamps": first_camps,
        "objectives": objectives,
    }))
}

/// Applies a rune page for a champion, replacing a page OmniGet owns.
#[tauri::command]
pub async fn league_apply_runes(
    name: String,
    primary_style_id: i64,
    sub_style_id: i64,
    selected_perk_ids: Vec<i64>,
    spell1: Option<i64>,
    spell2: Option<i64>,
) -> Result<Value, String> {
    ensure_enabled()?;
    if selected_perk_ids.len() < 6 {
        return Err("a rune page needs at least 6 perks".to_string());
    }
    let client = get_client().await?;

    // Reuse our own page when it exists so the user's own pages survive.
    let pages = lcu_get_raw(&client, "/lol-perks/v1/pages")
        .await
        .unwrap_or(Value::Null);
    let owned: Option<i64> = pages.as_array().and_then(|arr| {
        arr.iter()
            .find(|p| {
                p.get("name")
                    .and_then(Value::as_str)
                    .map(|n| n.starts_with("OmniGet"))
                    .unwrap_or(false)
                    && p.get("isDeletable")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
            })
            .and_then(|p| p.get("id").and_then(Value::as_i64))
    });
    if let Some(id) = owned {
        let _ = lcu_send(
            &client,
            reqwest::Method::DELETE,
            &format!("/lol-perks/v1/pages/{}", id),
            None,
        )
        .await;
    }

    let page = lcu_send(
        &client,
        reqwest::Method::POST,
        "/lol-perks/v1/pages",
        Some(json!({
            "name": format!("OmniGet · {}", name),
            "primaryStyleId": primary_style_id,
            "subStyleId": sub_style_id,
            "selectedPerkIds": selected_perk_ids,
            "current": true,
        })),
    )
    .await?;

    if let (Some(s1), Some(s2)) = (spell1, spell2) {
        let _ = lcu_send(
            &client,
            reqwest::Method::PATCH,
            "/lol-champ-select/v1/session/my-selection",
            Some(json!({ "spell1Id": s1, "spell2Id": s2 })),
        )
        .await;
    }

    Ok(page)
}

/// Sends a line to the champion-select (or lobby) chat.
#[tauri::command]
pub async fn league_send_chat(message: String) -> Result<(), String> {
    ensure_enabled()?;
    let text = message.trim();
    if text.is_empty() {
        return Err("empty message".to_string());
    }
    let client = get_client().await?;
    let conversations = lcu_get_raw(&client, "/lol-chat/v1/conversations").await?;
    let target = conversations
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|c| {
                    matches!(
                        c.get("type").and_then(Value::as_str),
                        Some("championSelect") | Some("customGame")
                    )
                })
                .or_else(|| {
                    arr.iter().find(|c| {
                        c.get("type").and_then(Value::as_str) == Some("chat")
                            && c.get("isMuted").and_then(Value::as_bool) == Some(false)
                    })
                })
        })
        .and_then(|c| c.get("id").and_then(Value::as_str).map(String::from))
        .ok_or_else(|| "no champion select chat open".to_string())?;

    lcu_send(
        &client,
        reqwest::Method::POST,
        &format!("/lol-chat/v1/conversations/{}/messages", target),
        Some(json!({ "body": text, "type": "chat" })),
    )
    .await?;
    Ok(())
}

/// Rune and summoner-spell setups the game client itself recommends.
///
/// This is the client's own recommendation engine, so it always matches the
/// installed patch and comes localized — no scraping involved.
#[tauri::command]
pub async fn league_rune_recommendations(
    champion_id: i64,
    position: Option<String>,
) -> Result<Value, String> {
    ensure_enabled()?;
    let client = get_client().await?;
    let pos = position
        .map(|p| p.to_ascii_uppercase())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "NONE".to_string());
    let path = format!(
        "/lol-perks/v1/recommended-pages/champion/{}/position/{}/map/11",
        champion_id, pos
    );
    let pages = lcu_get_raw(&client, &path).await?;

    let list: Vec<Value> = pages
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|p| {
                    let perks: Vec<i64> = p
                        .get("perks")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(|x| x.get("id").and_then(Value::as_i64)).collect())
                        .unwrap_or_default();
                    json!({
                        "recommendationId": p.get("recommendationId"),
                        "keystoneId": p.get("keystone").and_then(|k| k.get("id")),
                        "keystoneName": p.get("keystone").and_then(|k| k.get("name")),
                        "primaryStyleId": p.get("primaryPerkStyleId"),
                        "subStyleId": p.get("secondaryPerkStyleId"),
                        "selectedPerkIds": perks,
                        "summonerSpellIds": p.get("summonerSpellIds"),
                        "primaryAttribute": p.get("primaryRecommendationAttribute"),
                        "secondaryAttribute": p.get("secondaryRecommendationAttribute"),
                        "isDefault": p.get("isDefaultPosition"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({ "championId": champion_id, "position": pos, "pages": list }))
}

/// Champion tier rankings, per position, from op.gg's public champion API.
///
/// The response is passed through with only the fields the UI needs; the
/// numeric `tier` (1 = best) is what op.gg renders as S+/S/A and friends.
#[tauri::command]
pub async fn league_champion_tiers(
    region: Option<String>,
    tier: Option<String>,
) -> Result<Value, String> {
    ensure_enabled()?;
    let region = region.unwrap_or_else(|| "br".to_string());
    let bracket = tier.unwrap_or_else(|| "emerald_plus".to_string());
    if !region.chars().all(|c| c.is_ascii_alphanumeric())
        || !bracket.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("invalid region or tier".to_string());
    }
    let url = format!(
        "https://lol-api-champion.op.gg/api/{}/champions/ranked?tier={}",
        region, bracket
    );
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("OmniGet")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("tier list unavailable: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("tier list returned {}", resp.status().as_u16()));
    }
    let body = resp
        .json::<Value>()
        .await
        .map_err(|e| format!("invalid tier list response: {}", e))?;

    let entries: Vec<Value> = body
        .get("data")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let id = c.get("id").and_then(Value::as_i64)?;
                    let positions: Vec<Value> = c
                        .get("positions")
                        .and_then(Value::as_array)
                        .map(|ps| {
                            ps.iter()
                                .filter_map(|p| {
                                    let stats = p.get("stats")?;
                                    Some(json!({
                                        "position": p.get("name"),
                                        "tier": stats.get("tier_data").and_then(|t| t.get("tier")),
                                        "rank": stats.get("tier_data").and_then(|t| t.get("rank")),
                                        "winRate": stats.get("win_rate"),
                                        "pickRate": stats.get("pick_rate"),
                                        "banRate": stats.get("ban_rate"),
                                        "roleRate": stats.get("role_rate"),
                                        "games": stats.get("play"),
                                    }))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if positions.is_empty() {
                        return None;
                    }
                    Some(json!({ "championId": id, "positions": positions }))
                })
                .collect()
        })
        .unwrap_or_default();

    if entries.is_empty() {
        return Err("tier list returned no champions".to_string());
    }
    Ok(json!({ "region": region, "bracket": bracket, "champions": entries }))
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

fn action_champion_lists(
    settings: &omniget_core::models::settings::LeagueSettings,
    action_type: &str,
) -> (bool, Vec<i64>) {
    if action_type == "ban" {
        (settings.auto_ban, settings.ban_champions.clone())
    } else {
        (settings.auto_pick, settings.pick_champions.clone())
    }
}

async fn handle_champ_select(
    client: &LcuClient,
    settings: &omniget_core::models::settings::LeagueSettings,
) -> Result<(), String> {
    let session = lcu_get_raw(client, "/lol-champ-select/v1/session").await?;
    let cell = session
        .get("localPlayerCellId")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    if cell < 0 {
        return Ok(());
    }

    let mut taken: HashSet<i64> = HashSet::new();
    if let Some(groups) = session.get("actions").and_then(Value::as_array) {
        for group in groups {
            for action in group.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                let completed = action
                    .get("completed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if completed {
                    if let Some(id) = action.get("championId").and_then(Value::as_i64) {
                        if id > 0 {
                            taken.insert(id);
                        }
                    }
                }
            }
        }
    }
    for team_key in ["myTeam", "theirTeam"] {
        if let Some(team) = session.get(team_key).and_then(Value::as_array) {
            for member in team {
                if let Some(id) = member.get("championId").and_then(Value::as_i64) {
                    if id > 0 {
                        taken.insert(id);
                    }
                }
            }
        }
    }

    let mut pickable: Option<HashSet<i64>> = None;
    let mut bannable: Option<HashSet<i64>> = None;

    let groups: Vec<Value> = session
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for group in groups {
        for action in group.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
            let actor = action
                .get("actorCellId")
                .and_then(Value::as_i64)
                .unwrap_or(-2);
            let in_progress = action
                .get("isInProgress")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let completed = action
                .get("completed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let action_id = action.get("id").and_then(Value::as_i64).unwrap_or(-1);
            if actor != cell || !in_progress || completed || action_id < 0 {
                continue;
            }
            {
                let handled = CS_HANDLED.lock().await;
                if handled.contains(&action_id) {
                    continue;
                }
            }
            let action_type = action
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("pick")
                .to_string();
            let (enabled, list) = action_champion_lists(settings, &action_type);
            if !enabled || list.is_empty() {
                continue;
            }
            let pool = if action_type == "ban" {
                if bannable.is_none() {
                    let ids =
                        lcu_get_raw(client, "/lol-champ-select/v1/bannable-champion-ids").await?;
                    bannable = Some(
                        ids.as_array()
                            .map(|a| a.iter().filter_map(Value::as_i64).collect())
                            .unwrap_or_default(),
                    );
                }
                bannable.as_ref()
            } else {
                if pickable.is_none() {
                    let ids =
                        lcu_get_raw(client, "/lol-champ-select/v1/pickable-champion-ids").await?;
                    pickable = Some(
                        ids.as_array()
                            .map(|a| a.iter().filter_map(Value::as_i64).collect())
                            .unwrap_or_default(),
                    );
                }
                pickable.as_ref()
            };
            let pool = match pool {
                Some(p) => p,
                None => continue,
            };
            let choice = list
                .iter()
                .find(|c| pool.contains(c) && !taken.contains(c))
                .copied();
            let choice = match choice {
                Some(c) => c,
                None => continue,
            };
            let complete_action = settings.auto_lock || action_type == "ban";
            let path = format!("/lol-champ-select/v1/session/actions/{}", action_id);
            let body = json!({ "championId": choice, "completed": complete_action });
            match lcu_send(client, reqwest::Method::PATCH, &path, Some(body)).await {
                Ok(_) => {
                    let mut handled = CS_HANDLED.lock().await;
                    handled.insert(action_id);
                    tracing::info!(
                        "[league] auto-{} champion {} (locked: {})",
                        action_type,
                        choice,
                        complete_action
                    );
                }
                Err(e) => {
                    tracing::warn!("[league] auto-{} failed: {}", action_type, e);
                }
            }
        }
    }
    Ok(())
}

fn spawn_poller() {
    if POLLER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let settings = league_settings();
            if !settings.enabled {
                continue;
            }
            let auto_accept = AUTO_ACCEPT.load(Ordering::Relaxed);
            let champ_select_auto = settings.auto_pick || settings.auto_ban;
            if !auto_accept && !champ_select_auto {
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
            let phase = phase.as_str().unwrap_or("");
            if phase == "ReadyCheck" && auto_accept {
                if let Err(e) =
                    lcu_post_raw(&client, "/lol-matchmaking/v1/ready-check/accept").await
                {
                    tracing::warn!("[league] auto-accept failed: {}", e);
                } else {
                    tracing::info!("[league] ready check accepted");
                }
            }
            if phase == "ChampSelect" && champ_select_auto {
                if let Err(e) = handle_champ_select(&client, &settings).await {
                    tracing::debug!("[league] champ select tick: {}", e);
                }
            } else if phase != "ChampSelect" {
                let mut handled = CS_HANDLED.lock().await;
                if !handled.is_empty() {
                    handled.clear();
                }
            }
        }
    });
}

pub fn start_background() {
    let settings = crate::storage::config::load_settings_standalone();
    if settings.league.enabled {
        AUTO_ACCEPT.store(settings.league.auto_accept, Ordering::Relaxed);
        spawn_poller();
    }
}
