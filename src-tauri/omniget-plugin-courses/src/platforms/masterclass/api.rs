use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const MC_API_KEY: &str = "b9517f7d8d1f48c2de88100f2c13e77a9d8e524aed204651acca65202ff5c6cb9244c045795b1fafda617ac5eb0a6c50";

#[derive(Clone)]
pub struct MasterClassSession {
    pub cookies: String,
    pub profile_id: Option<String>,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub cookies: String,
    pub profile_id: Option<String>,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterClassCourse {
    pub id: String,
    pub name: String,
    pub instructor: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterClassModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<MasterClassLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterClassLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub media_uuid: Option<String>,
}

fn build_client(cookies: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", HeaderValue::from_str(cookies)?);
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
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
    Ok(data_dir.join("omniget").join("masterclass_session.json"))
}

pub async fn authenticate_cookies(cookies: &str) -> anyhow::Result<MasterClassSession> {
    let client = build_client(cookies)?;

    let session = MasterClassSession {
        cookies: cookies.to_string(),
        profile_id: None,
        client,
    };

    let profile_id = fetch_profile_id(&session).await.ok();

    Ok(MasterClassSession {
        cookies: cookies.to_string(),
        profile_id,
        client: session.client,
    })
}

async fn fetch_profile_id(session: &MasterClassSession) -> anyhow::Result<String> {
    let resp = session
        .client
        .get("https://www.masterclass.com/jsonapi/v1/watch-list-items?deep=true&page%5Bsize%5D=1")
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!("Failed to fetch profile: status {}", status));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let profile_id = body
        .get("meta")
        .and_then(|m| m.get("profile_id").or_else(|| m.get("profileId")))
        .or_else(|| body.get("profile_id"))
        .map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("profile_id not found in response"))?;

    Ok(profile_id)
}

pub async fn validate_token(session: &MasterClassSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://www.masterclass.com/jsonapi/v1/watch-list-items?deep=true&page%5Bsize%5D=1")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &MasterClassSession) -> anyhow::Result<Vec<MasterClassCourse>> {
    let resp = session
        .client
        .get("https://www.masterclass.com/jsonapi/v1/watch-list-items?deep=true")
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
        .cloned()
        .unwrap_or_default();

    for item in &items {
        let attrs = item.get("attributes").unwrap_or(item);

        let id = item
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let name = attrs
            .get("title")
            .or_else(|| attrs.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let instructor = attrs
            .get("instructor_name")
            .or_else(|| attrs.get("instructor"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = attrs
            .get("slug")
            .or_else(|| attrs.get("url_slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() && !name.is_empty() {
            courses.push(MasterClassCourse {
                id,
                name,
                instructor,
                slug,
            });
        }
    }

    Ok(courses)
}

pub async fn get_course_uuid(
    session: &MasterClassSession,
    slug: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://www.masterclass.com/api/v3/unique-identifiers/courses/{}",
        slug
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_course_uuid returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let uuid = body
        .get("uuid")
        .or_else(|| body.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("uuid not found in response"))?
        .to_string();

    Ok(uuid)
}

pub async fn get_course_content(
    session: &MasterClassSession,
    slug: &str,
) -> anyhow::Result<Vec<MasterClassModule>> {
    let uuid = get_course_uuid(session, slug).await?;

    let url = format!(
        "https://www.masterclass.com/api/v3/watch-bundles/{}",
        uuid
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

    let chapters = body
        .get("chapters")
        .or_else(|| body.get("episodes"))
        .or_else(|| body.get("lessons"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_id = chapter
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => format!("{}", ci),
            })
            .unwrap_or_else(|| format!("{}", ci));

        let chapter_name = chapter
            .get("title")
            .or_else(|| chapter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let media_uuid = chapter
            .get("media_uuid")
            .or_else(|| chapter.get("mediaUuid"))
            .or_else(|| chapter.get("video_uuid"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let lessons = vec![MasterClassLesson {
            id: chapter_id.clone(),
            name: chapter_name.clone(),
            order: ci as i64,
            media_uuid,
        }];

        modules.push(MasterClassModule {
            id: chapter_id,
            name: chapter_name,
            order: ci as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_video_metadata(
    session: &MasterClassSession,
    media_uuid: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://edge.masterclass.com/api/v1/media/metadata/{}",
        media_uuid
    );

    let profile_id = session
        .profile_id
        .as_deref()
        .unwrap_or("");

    let resp = session
        .client
        .get(&url)
        .header("X-Api-Key", MC_API_KEY)
        .header("Mc-Profile-Id", profile_id)
        .header("Origin", "https://www.masterclass.com")
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_video_metadata returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let sources = body
        .get("sources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for source in &sources {
        let src = source
            .get("src")
            .or_else(|| source.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !src.is_empty() {
            return Ok(src.to_string());
        }
    }

    let direct_url = body
        .get("url")
        .or_else(|| body.get("src"))
        .or_else(|| body.get("source"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !direct_url.is_empty() {
        return Ok(direct_url.to_string());
    }

    Err(anyhow!("No video URL found in metadata response"))
}

pub async fn save_session(session: &MasterClassSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        cookies: session.cookies.clone(),
        profile_id: session.profile_id.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[masterclass] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<MasterClassSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookies)?;

    tracing::info!("[masterclass] session loaded");

    Ok(Some(MasterClassSession {
        cookies: saved.cookies,
        profile_id: saved.profile_id,
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
