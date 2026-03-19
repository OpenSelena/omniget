use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://api.cakto.com.br";

#[derive(Clone)]
pub struct CaktoMembersSession {
    pub cookie: String,
    pub access_token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub cookie: String,
    pub access_token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaktoMembersCourse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaktoMembersModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<CaktoMembersLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaktoMembersLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub video_uuid: Option<String>,
    pub description: Option<String>,
}

fn build_client(access_token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", access_token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://app.cakto.com.br"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://app.cakto.com.br/"),
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

fn build_cookie_client(cookie: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!("sessionid={}", cookie))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    Ok(client)
}

fn session_file_path() -> anyhow::Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow!("Could not find app data directory"))?;
    Ok(data_dir.join("omniget").join("caktomembers_session.json"))
}

pub async fn fetch_access_token(cookie: &str) -> anyhow::Result<String> {
    let cookie_client = build_cookie_client(cookie)?;

    let url = format!("{}/api/members/courses/token/", API_BASE);
    let resp = cookie_client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "fetch_access_token returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let token = body
        .get("accessToken")
        .or_else(|| body.get("access_token"))
        .or_else(|| body.get("token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No accessToken in response"))?
        .to_string();

    Ok(token)
}

pub async fn validate_token(session: &CaktoMembersSession) -> anyhow::Result<bool> {
    let url = format!("{}/api/members/courses/", API_BASE);
    let resp = session.client.get(&url).send().await?;
    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &CaktoMembersSession) -> anyhow::Result<Vec<CaktoMembersCourse>> {
    let url = format!("{}/api/members/courses/", API_BASE);
    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_courses returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let courses_arr = if body.is_array() {
        body.as_array().cloned().unwrap_or_default()
    } else {
        body.get("results")
            .or_else(|| body.get("courses"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let mut courses = Vec::new();

    for item in &courses_arr {
        let id = item
            .get("externalId")
            .or_else(|| item.get("id"))
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let name = item
            .get("nome")
            .or_else(|| item.get("name"))
            .or_else(|| item.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() {
            courses.push(CaktoMembersCourse { id, name });
        }

        if let Some(modulos) = item.get("modulos").and_then(|v| v.as_array()) {
            let _ = modulos;
        }
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &CaktoMembersSession,
    course_id: &str,
) -> anyhow::Result<Vec<CaktoMembersModule>> {
    let url = format!("{}/api/members/courses/", API_BASE);
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

    let courses_arr = if body.is_array() {
        body.as_array().cloned().unwrap_or_default()
    } else {
        body.get("results")
            .or_else(|| body.get("courses"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let course_data = courses_arr
        .iter()
        .find(|c| {
            let eid = c.get("externalId").or_else(|| c.get("id"));
            if let Some(v) = eid {
                match v {
                    serde_json::Value::String(s) => s == course_id,
                    serde_json::Value::Number(n) => n.to_string() == course_id,
                    _ => false,
                }
            } else {
                false
            }
        })
        .ok_or_else(|| anyhow!("Course not found in response"))?;

    let modulos = course_data
        .get("modulos")
        .or_else(|| course_data.get("modules"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (mi, mod_val) in modulos.iter().enumerate() {
        let mod_id = mod_val
            .get("id")
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => format!("{}", mi),
            })
            .unwrap_or_else(|| format!("{}", mi));

        let mod_name = mod_val
            .get("nome")
            .or_else(|| mod_val.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let aulas = mod_val
            .get("aulas")
            .or_else(|| mod_val.get("lessons"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (li, lesson_val) in aulas.iter().enumerate() {
            let lesson_id = lesson_val
                .get("id")
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => format!("{}", li),
                })
                .unwrap_or_else(|| format!("{}", li));

            let lesson_name = lesson_val
                .get("nome")
                .or_else(|| lesson_val.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let video_uuid = lesson_val
                .get("video")
                .or_else(|| lesson_val.get("video_uuid"))
                .or_else(|| lesson_val.get("videoUuid"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let description = lesson_val
                .get("descricao")
                .or_else(|| lesson_val.get("description"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from);

            lessons.push(CaktoMembersLesson {
                id: lesson_id,
                name: lesson_name,
                order: li as i64,
                video_uuid,
                description,
            });
        }

        modules.push(CaktoMembersModule {
            id: mod_id,
            name: mod_name,
            order: mi as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_lesson_video_url(
    session: &CaktoMembersSession,
    lesson_id: &str,
    video_uuid: Option<&str>,
) -> anyhow::Result<Option<String>> {
    if let Some(uuid) = video_uuid {
        if !uuid.is_empty() {
            let hls_url = format!("https://stream.cakto.com.br/{}/playlist.m3u8", uuid);
            return Ok(Some(hls_url));
        }
    }

    let url = format!(
        "https://us-central1-cakto2.cloudfunctions.net/api/aulas/{}/assistir",
        lesson_id
    );
    let resp = session.client.get(&url).send().await?;

    if resp.status().is_success() {
        let body_text = resp.text().await?;
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(&body_text) {
            if let Some(url) = body.get("video_url")
                .or_else(|| body.get("url"))
                .or_else(|| body.get("videoUrl"))
                .and_then(|v| v.as_str())
            {
                return Ok(Some(url.to_string()));
            }
        }
    }

    Ok(None)
}

pub async fn save_session(session: &CaktoMembersSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        cookie: session.cookie.clone(),
        access_token: session.access_token.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[caktomembers] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<CaktoMembersSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.access_token)?;

    tracing::info!("[caktomembers] session loaded");

    Ok(Some(CaktoMembersSession {
        cookie: saved.cookie,
        access_token: saved.access_token,
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
