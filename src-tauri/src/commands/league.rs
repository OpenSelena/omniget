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
    let (stats, private_profile) = match history {
        Ok(h) => {
            let games: Vec<Value> = h
                .get("games")
                .and_then(|g| g.get("games"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            (compute_history_stats(&games, &puuid), games.is_empty())
        }
        Err(_) => (json!({ "games": 0, "insights": [] }), true),
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
        "privateProfile": private_profile,
    }))
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
