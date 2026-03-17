use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://api.estrategia.com";

#[derive(Clone)]
pub struct EstrategiaLdiSession {
    pub token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiCourse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<EstrategiaLdiLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiTrack {
    pub url: String,
    pub duration: Option<f64>,
}

fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://concursos.estrategia.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://concursos.estrategia.com/"),
    );

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
    Ok(data_dir.join("omniget").join("estrategia_ldi_session.json"))
}

pub async fn validate_token(session: &EstrategiaLdiSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/bff/goals/shelves?page=1&per_page=1", API_BASE))
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn search_courses(session: &EstrategiaLdiSession, query: &str) -> anyhow::Result<Vec<EstrategiaLdiCourse>> {
    let mut courses = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "{}/bff/goals/shelves?page={}&per_page=20&name={}",
            API_BASE,
            page,
            urlencoding::encode(query)
        );

        let resp = session.client.get(&url).send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!(
                "search_courses returned status {}: {}",
                status,
                &body_text[..body_text.len().min(300)]
            ));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;

        let goals = body
            .get("goals")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if goals.is_empty() {
            break;
        }

        for goal in &goals {
            let id = goal
                .get("id")
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            let name = goal
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !id.is_empty() {
                courses.push(EstrategiaLdiCourse { id, name });
            }
        }

        page += 1;
        if page > 50 {
            break;
        }
    }

    Ok(courses)
}

pub async fn list_courses(session: &EstrategiaLdiSession) -> anyhow::Result<Vec<EstrategiaLdiCourse>> {
    search_courses(session, "").await
}

pub async fn get_course_content(
    session: &EstrategiaLdiSession,
    goal_id: &str,
) -> anyhow::Result<Vec<EstrategiaLdiModule>> {
    let url = format!("{}/bff/goals/{}/contents/ldi", API_BASE, goal_id);

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_course_content returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let chapters = body
        .get("chapters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            body.as_array().cloned().unwrap_or_default()
        });

    let mut modules = Vec::new();

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_name = chapter
            .get("title")
            .or_else(|| chapter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Chapter")
            .to_string();

        let chapter_id = chapter
            .get("id")
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
                .get("id")
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

            lessons.push(EstrategiaLdiLesson {
                id: item_id,
                name: item_name,
                order: ii as i64,
            });
        }

        modules.push(EstrategiaLdiModule {
            id: chapter_id,
            name: chapter_name,
            order: ci as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_item_detail(
    session: &EstrategiaLdiSession,
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
    session: &EstrategiaLdiSession,
    track_id: &str,
) -> anyhow::Result<EstrategiaLdiTrack> {
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

    Ok(EstrategiaLdiTrack {
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

pub async fn save_session(session: &EstrategiaLdiSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[estrategia_ldi] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<EstrategiaLdiSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[estrategia_ldi] session loaded");

    Ok(Some(EstrategiaLdiSession {
        token: saved.token,
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
