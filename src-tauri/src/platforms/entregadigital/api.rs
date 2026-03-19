use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct EntregaDigitalSession {
    pub token: String,
    pub api_base: String,
    pub app_version: String,
    pub device_id: String,
    pub os_value: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub api_base: String,
    pub app_version: String,
    pub device_id: String,
    pub os_value: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntregaDigitalCourse {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntregaDigitalModule {
    pub id: i64,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<EntregaDigitalLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntregaDigitalLesson {
    pub id: i64,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntregaDigitalLessonDetail {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub video_url: Option<String>,
    pub attachments: Vec<EntregaDigitalAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntregaDigitalAttachment {
    pub url: String,
    pub name: String,
    pub size: Option<u64>,
}

pub fn build_api_base(site_url: &str) -> anyhow::Result<String> {
    let parsed = url::Url::parse(site_url)
        .map_err(|e| anyhow!("Invalid site URL: {}", e))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("No host in URL"))?;

    if !host.contains("entregadigital.app.br") {
        return Err(anyhow!("Not an Entrega Digital URL"));
    }

    let subdomain = host
        .strip_suffix(".entregadigital.app.br")
        .unwrap_or(host);

    Ok(format!(
        "https://api-{}.entregadigital.app.br/api/v1/app",
        subdomain
    ))
}

fn build_client(
    token: &str,
    app_version: &str,
    device_id: &str,
    os_value: &str,
) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "app-version",
        HeaderValue::from_str(app_version)?,
    );
    headers.insert(
        "device-id",
        HeaderValue::from_str(device_id)?,
    );
    headers.insert(
        "os",
        HeaderValue::from_str(os_value)?,
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
    Ok(data_dir.join("omniget").join("entregadigital_session.json"))
}

pub async fn validate_token(session: &EntregaDigitalSession) -> anyhow::Result<bool> {
    let url = format!("{}/products", session.api_base);

    let resp = session
        .client
        .get(&url)
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &EntregaDigitalSession) -> anyhow::Result<Vec<EntregaDigitalCourse>> {
    let url = format!("{}/products", session.api_base);

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

    let products = body
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();

    for item in &products {
        let id = item
            .get("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let name = item
            .get("name")
            .or_else(|| item.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        courses.push(EntregaDigitalCourse { id, name });
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &EntregaDigitalSession,
    course_id: i64,
) -> anyhow::Result<Vec<EntregaDigitalModule>> {
    let url = format!("{}/products/{}", session.api_base, course_id);

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
        let mod_id = module_val
            .get("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let mod_name = module_val
            .get("name")
            .or_else(|| module_val.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mod_order = module_val
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(mi as i64);

        let lessons_arr = module_val
            .get("lessons")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (li, lesson_val) in lessons_arr.iter().enumerate() {
            let lesson_id = lesson_val
                .get("id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let lesson_name = lesson_val
                .get("name")
                .or_else(|| lesson_val.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_order = lesson_val
                .get("order")
                .and_then(|v| v.as_i64())
                .unwrap_or(li as i64);

            lessons.push(EntregaDigitalLesson {
                id: lesson_id,
                name: lesson_name,
                order: lesson_order,
            });
        }

        lessons.sort_by_key(|l| l.order);

        modules.push(EntregaDigitalModule {
            id: mod_id,
            name: mod_name,
            order: mod_order,
            lessons,
        });
    }

    modules.sort_by_key(|m| m.order);

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &EntregaDigitalSession,
    lesson_id: i64,
) -> anyhow::Result<EntregaDigitalLessonDetail> {
    let url = format!("{}/lessons/{}/auth", session.api_base, lesson_id);

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_lesson_detail returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let name = body
        .get("name")
        .or_else(|| body.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    let detail = body.get("detail");

    let video_url = detail
        .and_then(|d| {
            d.get("panda")
                .and_then(|p| p.get("video_hls"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .or_else(|| {
            detail.and_then(|d| {
                d.get("panda-live")
                    .and_then(|p| p.get("video_hls"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
        })
        .or_else(|| {
            body.get("video_player")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .or_else(|| {
            body.get("action")
                .and_then(|a| a.get("url"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });

    let attachments = body
        .get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let att_url = a
                        .get("url")
                        .and_then(|v| v.as_str())?
                        .to_string();

                    let att_name = a
                        .get("name")
                        .or_else(|| a.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("attachment")
                        .to_string();

                    let size = a
                        .get("size")
                        .and_then(|v| v.as_u64());

                    Some(EntregaDigitalAttachment {
                        url: att_url,
                        name: att_name,
                        size,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(EntregaDigitalLessonDetail {
        id: lesson_id,
        name,
        description,
        video_url,
        attachments,
    })
}

pub async fn save_session(session: &EntregaDigitalSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        api_base: session.api_base.clone(),
        app_version: session.app_version.clone(),
        device_id: session.device_id.clone(),
        os_value: session.os_value.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[entregadigital] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<EntregaDigitalSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.app_version, &saved.device_id, &saved.os_value)?;

    tracing::info!("[entregadigital] session loaded");

    Ok(Some(EntregaDigitalSession {
        token: saved.token,
        api_base: saved.api_base,
        app_version: saved.app_version,
        device_id: saved.device_id,
        os_value: saved.os_value,
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
