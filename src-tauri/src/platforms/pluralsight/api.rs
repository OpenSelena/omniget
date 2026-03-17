use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct PluralsightSession {
    pub cookies: String,
    pub xsrf_token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub cookies: String,
    pub xsrf_token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluralsightCourse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluralsightModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<PluralsightLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluralsightLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub clip_id: String,
    pub version_id: String,
}

fn extract_xsrf_token(cookies: &str) -> Option<String> {
    for part in cookies.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("XSRF-TOKEN=") {
            let decoded = urlencoding::decode(val.trim()).ok()?;
            return Some(decoded.to_string());
        }
    }
    None
}

fn build_client(cookies: &str, xsrf_token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", HeaderValue::from_str(cookies)?);
    headers.insert("x-xsrf-token", HeaderValue::from_str(xsrf_token)?);
    headers.insert("Accept", HeaderValue::from_static("application/json"));

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
    Ok(data_dir.join("omniget").join("pluralsight_session.json"))
}

pub fn authenticate_cookies(cookies: &str) -> anyhow::Result<PluralsightSession> {
    let xsrf_token = extract_xsrf_token(cookies)
        .ok_or_else(|| anyhow!("XSRF-TOKEN not found in cookies. Make sure to include the full Cookie header from browser DevTools."))?;

    let client = build_client(cookies, &xsrf_token)?;

    Ok(PluralsightSession {
        cookies: cookies.to_string(),
        xsrf_token,
        client,
    })
}

pub async fn validate_token(session: &PluralsightSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://app.pluralsight.com/learner/user/user-content-history?dataSource=v2&limit=1")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &PluralsightSession) -> anyhow::Result<Vec<PluralsightCourse>> {
    let resp = session
        .client
        .get("https://app.pluralsight.com/learner/user/user-content-history?dataSource=v2&limit=300")
        .send()
        .await?;

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

    let mut courses = Vec::new();

    let items = body
        .get("data")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();

    for item in &items {
        let id = item
            .get("contentId")
            .or_else(|| item.get("id"))
            .or_else(|| item.get("courseId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = item
            .get("title")
            .or_else(|| item.get("name"))
            .or_else(|| item.get("contentTitle"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() && !name.is_empty() {
            if !courses.iter().any(|c: &PluralsightCourse| c.id == id) {
                courses.push(PluralsightCourse { id, name });
            }
        }
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &PluralsightSession,
    course_id: &str,
) -> anyhow::Result<Vec<PluralsightModule>> {
    let url = format!(
        "https://app.pluralsight.com/course-player/api/v1/table-of-contents/course/{}",
        course_id
    );

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

    let modules_arr = body
        .get("modules")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (mi, module_val) in modules_arr.iter().enumerate() {
        let module_id = module_val
            .get("id")
            .or_else(|| module_val.get("moduleId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let module_name = module_val
            .get("title")
            .or_else(|| module_val.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let clips = module_val
            .get("contentItems")
            .or_else(|| module_val.get("clips"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (ci, clip_val) in clips.iter().enumerate() {
            let clip_id = clip_val
                .get("id")
                .or_else(|| clip_val.get("clipId"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let clip_name = clip_val
                .get("title")
                .or_else(|| clip_val.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let version_id = clip_val
                .get("version")
                .or_else(|| clip_val.get("versionId"))
                .and_then(|v| match v {
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "1".to_string());

            if !clip_id.is_empty() {
                lessons.push(PluralsightLesson {
                    id: format!("{}-{}", module_id, ci),
                    name: clip_name,
                    order: ci as i64,
                    clip_id,
                    version_id,
                });
            }
        }

        modules.push(PluralsightModule {
            id: if module_id.is_empty() {
                format!("{}", mi)
            } else {
                module_id
            },
            name: module_name,
            order: mi as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_clip_url(
    session: &PluralsightSession,
    clip_id: &str,
    version_id: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://app.pluralsight.com/video/delivery/api/v1/clips/{}/versions/{}",
        clip_id, version_id
    );

    let resp = session
        .client
        .post(&url)
        .header("x-team", "video-services")
        .json(&serde_json::json!({
            "online": true,
            "boundedContext": "course",
            "preferredAudioLanguage": "en"
        }))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_clip_url returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let m3u8_url = body
        .get("urls")
        .and_then(|u| u.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("url").or_else(|| v.get("src")))
        .and_then(|v| v.as_str())
        .or_else(|| body.get("url").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("No video URL found in clip response"))?
        .to_string();

    Ok(m3u8_url)
}

pub async fn save_session(session: &PluralsightSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        cookies: session.cookies.clone(),
        xsrf_token: session.xsrf_token.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[pluralsight] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<PluralsightSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookies, &saved.xsrf_token)?;

    tracing::info!("[pluralsight] session loaded");

    Ok(Some(PluralsightSession {
        cookies: saved.cookies,
        xsrf_token: saved.xsrf_token,
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
