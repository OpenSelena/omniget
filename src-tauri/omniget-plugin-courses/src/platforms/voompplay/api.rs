use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct VoompSession {
    pub token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoompCourse {
    pub id: i64,
    pub name: String,
    pub lessons_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoompModule {
    pub id: i64,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<VoompLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoompLesson {
    pub id: i64,
    pub name: String,
    pub order: i64,
    pub media_type: Option<String>,
    pub source: Option<String>,
    pub content: Option<String>,
    pub duration: Option<f64>,
    pub attachments: Vec<VoompAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoompAttachment {
    pub url: String,
    pub name: String,
    pub size: Option<u64>,
}

fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(token)?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://voompplay.com.br"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://voompplay.com.br/"),
    );

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
    Ok(data_dir.join("omniget").join("voomp_session.json"))
}

pub async fn validate_token(session: &VoompSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://api.voompplay.com.br/member/me")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &VoompSession) -> anyhow::Result<Vec<VoompCourse>> {
    let mut all_courses = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "https://api.voompplay.com.br/member/me/all-courses?page={}",
            page
        );

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

        let last_page = body
            .get("meta")
            .and_then(|m| m.get("last_page"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let courses_obj = body
            .get("courses")
            .and_then(|v| v.as_object());

        if let Some(sites) = courses_obj {
            for (_site_name, courses_val) in sites {
                let courses_arr = match courses_val.as_array() {
                    Some(arr) => arr,
                    None => continue,
                };

                for item in courses_arr {
                    let has_access = item
                        .get("has_access")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if !has_access {
                        continue;
                    }

                    let id = item
                        .get("id")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let name = item
                        .get("title")
                        .or_else(|| item.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let lessons_count = item
                        .get("lessons_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    all_courses.push(VoompCourse {
                        id,
                        name,
                        lessons_count,
                    });
                }
            }
        }

        if page >= last_page {
            break;
        }

        page += 1;
    }

    Ok(all_courses)
}

pub async fn get_modules(
    session: &VoompSession,
    course_id: i64,
) -> anyhow::Result<Vec<VoompModule>> {
    let url = format!(
        "https://api.voompplay.com.br/course/{}/watch?data[]=course&data[]=modules",
        course_id
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_modules returned status {}: {}",
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

    for item in &modules_arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let name = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let order = item
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        modules.push(VoompModule {
            id,
            name,
            order,
            lessons: Vec::new(),
        });
    }

    modules.sort_by_key(|m| m.order);

    Ok(modules)
}

pub async fn get_lessons(
    session: &VoompSession,
    course_id: i64,
    module_id: i64,
) -> anyhow::Result<Vec<VoompLesson>> {
    let url = format!(
        "https://api.voompplay.com.br/course/{}/watch?data[]=currentModuleLessons&current_module_id={}",
        course_id, module_id
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_lessons returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let lessons_arr = body
        .get("currentModuleLessons")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lessons = Vec::new();

    for item in &lessons_arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let name = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let order = item
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let media_type = item
            .get("mediaType")
            .and_then(|v| v.as_str())
            .map(String::from);

        let source = item
            .get("source")
            .and_then(|v| v.as_str())
            .map(String::from);

        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .map(String::from);

        let duration = item
            .get("duration")
            .and_then(|v| v.as_f64());

        let attachments = item
            .get("attachments")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let url = a
                            .get("cdn_url")
                            .or_else(|| a.get("path"))
                            .and_then(|v| v.as_str())?
                            .to_string();

                        let att_name = a
                            .get("title")
                            .or_else(|| a.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("attachment")
                            .to_string();

                        let size = a
                            .get("size")
                            .and_then(|v| v.as_u64());

                        Some(VoompAttachment {
                            url,
                            name: att_name,
                            size,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        lessons.push(VoompLesson {
            id,
            name,
            order,
            media_type,
            source,
            content,
            duration,
            attachments,
        });
    }

    lessons.sort_by_key(|l| l.order);

    Ok(lessons)
}

pub fn extract_panda_video_id(source: &str) -> Option<String> {
    let re = regex::Regex::new(r"[?&]v=([a-f0-9-]+)").ok()?;
    re.captures(source).map(|c| c[1].to_string())
}

pub async fn save_session(session: &VoompSession) -> anyhow::Result<()> {
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
    tracing::info!("[voomp] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<VoompSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[voomp] session loaded");

    Ok(Some(VoompSession {
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
