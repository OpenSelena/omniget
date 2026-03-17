use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://api.estrategia.com";

#[derive(Clone)]
pub struct EstrategiaMilitaresSession {
    pub token: String,
    pub cookie_string: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub cookie_string: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub goal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<EstrategiaMilitaresLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresTrack {
    pub url: String,
    pub duration: Option<f64>,
}

pub fn parse_token_input(input: &str) -> (String, String) {
    let trimmed = input.trim();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let cookies_arr = val.get("cookies")
                .and_then(|v| v.as_array())
                .cloned()
                .or_else(|| val.as_array().cloned())
                .unwrap_or_default();

            let mut jwt = String::new();
            let mut cookie_parts: Vec<String> = Vec::new();

            for c in &cookies_arr {
                let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if !name.is_empty() && !value.is_empty() {
                    cookie_parts.push(format!("{}={}", name, value));
                    if name == "__Secure-SID" {
                        jwt = value.to_string();
                    }
                }
            }

            if !jwt.is_empty() {
                return (jwt, cookie_parts.join("; "));
            }
        }
    }

    if trimmed.contains("; ") || trimmed.contains("__Secure-SID=") || trimmed.contains("_cfuvid=") {
        let mut jwt = String::new();
        for part in trimmed.split("; ") {
            if let Some(val) = part.strip_prefix("__Secure-SID=") {
                jwt = val.to_string();
            }
        }
        if !jwt.is_empty() {
            return (jwt, trimmed.to_string());
        }
        return (trimmed.to_string(), format!("__Secure-SID={}", trimmed));
    }

    (trimmed.to_string(), format!("__Secure-SID={}", trimmed))
}

fn build_client(token: &str, cookie_string: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "Cookie",
        HeaderValue::from_str(cookie_string)?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert("Accept-Language", HeaderValue::from_static("pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://militares.estrategia.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://militares.estrategia.com/"),
    );
    headers.insert("x-vertical", HeaderValue::from_static("militares"));
    headers.insert("x-requester-id", HeaderValue::from_static("front-student"));

    let client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()?;

    Ok(client)
}

fn session_file_path() -> anyhow::Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow!("Could not find app data directory"))?;
    Ok(data_dir.join("omniget").join("estrategia_militares_session.json"))
}

pub async fn validate_token(session: &EstrategiaMilitaresSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/bff/goals/shelves?page=1&per_page=1&name=test", API_BASE))
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    tracing::info!("[estrategia_militares] validate_token status={} body_preview={}", status, &body[..body.len().min(500)]);

    Ok(status.is_success())
}

async fn fetch_ldi_courses_for_goal(session: &EstrategiaMilitaresSession, goal_id: &str) -> Vec<EstrategiaMilitaresCourse> {
    let url = format!("{}/bff/goals/{}/contents/ldi?page=1&per_page=50", API_BASE, goal_id);

    let resp = match session.client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[estrategia_militares] failed to fetch LDI for goal {}: {}", goal_id, e);
            return Vec::new();
        }
    };

    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let body: serde_json::Value = match serde_json::from_str(&body_text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let contents = body
        .get("data")
        .and_then(|d| d.get("contents"))
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();
    for c in &contents {
        let id = c.get("id").map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }).unwrap_or_default();

        let name = c.get("title")
            .or(c.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = c.get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() && !slug.is_empty() {
            courses.push(EstrategiaMilitaresCourse {
                id,
                name,
                slug,
                goal_id: goal_id.to_string(),
            });
        }
    }

    courses
}

pub async fn search_courses(session: &EstrategiaMilitaresSession, query: &str) -> anyhow::Result<Vec<EstrategiaMilitaresCourse>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let url = format!(
        "{}/bff/goals/shelves?page=1&per_page=20&name={}",
        API_BASE,
        urlencoding::encode(query.trim())
    );

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    tracing::info!("[estrategia_militares] search status={} body_len={}", status, body_text.len());

    if !status.is_success() {
        return Err(anyhow!(
            "search_courses returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    let mut all_goal_ids = Vec::new();

    let result_data = body.get("data").unwrap_or(&body);

    if let Some(shelves) = result_data.get("shelves").and_then(|v| v.as_object()) {
        for (_shelf_name, goals_val) in shelves {
            let goals = goals_val.as_array().cloned().unwrap_or_default();
            for goal in &goals {
                let goal_id = goal.get("id").map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::new(),
                }).unwrap_or_default();

                if !goal_id.is_empty() {
                    all_goal_ids.push(goal_id);
                }
            }
        }
    }

    if let Some(highlights) = result_data.get("highlights") {
        let highlight_goals = highlights.get("goals").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for goal in &highlight_goals {
            let goal_id = goal.get("id").map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            }).unwrap_or_default();

            if !goal_id.is_empty() && !all_goal_ids.contains(&goal_id) {
                all_goal_ids.push(goal_id);
            }
        }
    }

    let mut courses = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for goal_id in &all_goal_ids {
        let ldi_courses = fetch_ldi_courses_for_goal(session, goal_id).await;
        for c in ldi_courses {
            if seen_ids.insert(c.id.clone()) {
                courses.push(c);
            }
        }
    }

    tracing::info!("[estrategia_militares] found {} courses across {} goals for query '{}'", courses.len(), all_goal_ids.len(), query);
    Ok(courses)
}

pub async fn list_courses(session: &EstrategiaMilitaresSession) -> anyhow::Result<Vec<EstrategiaMilitaresCourse>> {
    search_courses(session, "militar").await
}

pub async fn get_course_content(
    session: &EstrategiaMilitaresSession,
    course_slug: &str,
) -> anyhow::Result<Vec<EstrategiaMilitaresModule>> {
    let url = format!("{}/v3/mci/courses/slug/{}", API_BASE, course_slug);

    let resp = session.client.get(&url)
        .header("cache-control", "no-cache")
        .send()
        .await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    tracing::info!("[estrategia_militares] get_course_content slug={} status={} len={}", course_slug, status, body_text.len());

    if !status.is_success() {
        return Err(anyhow!(
            "get_course_content returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let course_data = body.get("data").unwrap_or(&body);

    let chapters = course_data
        .get("chapters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_name = chapter
            .get("title")
            .or_else(|| chapter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Chapter")
            .to_string();

        let chapter_id = chapter
            .get("chapter_id")
            .or(chapter.get("id"))
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => format!("{}", ci),
            })
            .unwrap_or_else(|| format!("{}", ci));

        let items = chapter
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (ii, item) in items.iter().enumerate() {
            let item_id = item
                .get("item_id")
                .or(item.get("id"))
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => format!("{}-{}", ci, ii),
                })
                .unwrap_or_else(|| format!("{}-{}", ci, ii));

            let item_name = item
                .get("title")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            lessons.push(EstrategiaMilitaresLesson {
                id: item_id,
                name: item_name,
                order: ii as i64,
            });
        }

        modules.push(EstrategiaMilitaresModule {
            id: chapter_id,
            name: chapter_name,
            order: ci as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_item_detail(
    session: &EstrategiaMilitaresSession,
    item_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "{}/v3/mci/items/{}?page=1&per_page=50",
        API_BASE, item_id
    );

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_item_detail returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    Ok(body)
}

pub async fn get_track_info(
    session: &EstrategiaMilitaresSession,
    track_id: &str,
) -> anyhow::Result<EstrategiaMilitaresTrack> {
    let url = format!("{}/v2/tracks/{}", API_BASE, track_id);

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_track_info returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let track_url = body
        .get("url")
        .or_else(|| body.get("source"))
        .or_else(|| body.get("file"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let duration = body
        .get("duration")
        .and_then(|v| v.as_f64());

    if track_url.is_empty() {
        return Err(anyhow!("No URL found in track response"));
    }

    Ok(EstrategiaMilitaresTrack {
        url: track_url,
        duration,
    })
}

pub fn extract_track_ids(item_detail: &serde_json::Value) -> Vec<String> {
    let mut track_ids = Vec::new();

    let tracks = item_detail
        .get("tracks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for track in &tracks {
        if let Some(id) = track.get("id").map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }) {
            if !id.is_empty() {
                track_ids.push(id);
            }
        }
    }

    if track_ids.is_empty() {
        if let Some(track_id) = item_detail.get("track_id").map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }) {
            if !track_id.is_empty() {
                track_ids.push(track_id);
            }
        }
    }

    track_ids
}

pub fn extract_attachment_urls(item_detail: &serde_json::Value) -> Vec<(String, String)> {
    let mut attachments = Vec::new();

    let atts = item_detail
        .get("attachments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for att in &atts {
        let url = att
            .get("url")
            .or_else(|| att.get("file"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = att
            .get("name")
            .or_else(|| att.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("attachment")
            .to_string();

        if !url.is_empty() {
            attachments.push((name, url));
        }
    }

    attachments
}

pub async fn save_session(session: &EstrategiaMilitaresSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        cookie_string: session.cookie_string.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[estrategia_militares] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<EstrategiaMilitaresSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.cookie_string)?;

    tracing::info!("[estrategia_militares] session loaded");

    Ok(Some(EstrategiaMilitaresSession {
        token: saved.token,
        cookie_string: saved.cookie_string,
        client,
    }))
}

pub async fn delete_saved_session() -> anyhow::Result<()> {
    let path = session_file_path()?;
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        tokio::fs::remove_file(&path).await?;
    }
    Ok(())
}
