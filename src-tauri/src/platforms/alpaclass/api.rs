use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://learner-api.alpaclass.com";

#[derive(Clone)]
pub struct AlpaclassSession {
    pub token: String,
    pub subdomain: String,
    pub origin_url: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub subdomain: String,
    pub origin_url: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassCourse {
    pub slug: String,
    pub name: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassCategory {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassModule {
    pub slug: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<AlpaclassLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassLesson {
    pub slug: String,
    pub name: String,
    pub lesson_type: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassLessonDetail {
    pub slug: String,
    pub name: String,
    pub html_content: Option<String>,
    pub video_url: Option<String>,
    pub files: Vec<AlpaclassFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlpaclassFile {
    pub name: String,
    pub url: String,
}

fn extract_subdomain(platform_url: &str) -> anyhow::Result<String> {
    let parsed = url::Url::parse(platform_url)
        .map_err(|e| anyhow!("Invalid platform URL: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("No host in platform URL"))?;
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        Ok(parts[0].to_string())
    } else {
        Ok(host.to_string())
    }
}

fn build_client(token: &str, subdomain: &str, origin_url: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert("subdomain", HeaderValue::from_str(subdomain)?);
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert("Origin", HeaderValue::from_str(origin_url)?);
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", origin_url))?,
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
    Ok(data_dir.join("omniget").join("alpaclass_session.json"))
}

pub async fn authenticate(token: &str, platform_url: &str) -> anyhow::Result<AlpaclassSession> {
    let origin = platform_url.trim_end_matches('/').to_string();
    let subdomain = extract_subdomain(&origin)?;
    let client = build_client(token, &subdomain, &origin)?;

    let session = AlpaclassSession {
        token: token.to_string(),
        subdomain,
        origin_url: origin,
        client,
    };

    let valid = validate_token(&session).await?;
    if !valid {
        return Err(anyhow!("Invalid token or platform URL"));
    }

    Ok(session)
}

pub async fn validate_token(session: &AlpaclassSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/learner/categories", API_BASE))
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_categories(session: &AlpaclassSession) -> anyhow::Result<Vec<AlpaclassCategory>> {
    let resp = session
        .client
        .get(format!("{}/learner/categories", API_BASE))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_categories returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let cats_arr = body
        .get("categories")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut categories = Vec::new();
    for item in &cats_arr {
        let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        categories.push(AlpaclassCategory { id, name });
    }

    Ok(categories)
}

pub async fn list_courses(session: &AlpaclassSession) -> anyhow::Result<Vec<AlpaclassCourse>> {
    let categories = list_categories(session).await?;
    let mut all_courses = Vec::new();

    for cat in &categories {
        let url = format!(
            "{}/learner/categories/{}/courses?showUnavailableCourses=true",
            API_BASE, cat.id
        );

        let resp = session.client.get(&url).send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            tracing::error!(
                "[alpaclass] list_courses for category {} returned status {}: {}",
                cat.name,
                status,
                &body_text[..body_text.len().min(300)]
            );
            continue;
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;

        let courses_arr = match &body {
            serde_json::Value::Array(arr) => arr.clone(),
            _ => body
                .get("courses")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
        };

        for item in &courses_arr {
            let slug = item
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let name = item
                .get("name")
                .or_else(|| item.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let author = item
                .get("author")
                .and_then(|a| a.get("name").and_then(|n| n.as_str()))
                .unwrap_or("")
                .to_string();

            if !slug.is_empty() {
                all_courses.push(AlpaclassCourse { slug, name, author });
            }
        }
    }

    Ok(all_courses)
}

pub async fn get_course_content(
    session: &AlpaclassSession,
    course_slug: &str,
) -> anyhow::Result<Vec<AlpaclassModule>> {
    let url = format!("{}/learner/courses/{}", API_BASE, course_slug);

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
        let slug = module_val
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = module_val
            .get("name")
            .or_else(|| module_val.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut lessons = Vec::new();

        let lessons_arr = module_val
            .get("lessons")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for (li, lesson_val) in lessons_arr.iter().enumerate() {
            let lesson_slug = lesson_val
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_title = lesson_val
                .get("title")
                .or_else(|| lesson_val.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_type = lesson_val
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            lessons.push(AlpaclassLesson {
                slug: lesson_slug,
                name: lesson_title,
                lesson_type,
                order: li as i64,
            });
        }

        modules.push(AlpaclassModule {
            slug,
            name,
            order: mi as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &AlpaclassSession,
    lesson_slug: &str,
) -> anyhow::Result<AlpaclassLessonDetail> {
    let url = format!("{}/learner/lessons/{}", API_BASE, lesson_slug);

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
        .get("title")
        .or_else(|| body.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(lesson_slug)
        .to_string();

    let html_content = body
        .get("htmlContent")
        .and_then(|v| v.as_str())
        .map(String::from);

    let video_url = body
        .get("content")
        .and_then(|c| {
            let content_type = c.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match content_type {
                "pandavideo" | "safevideo_video" => {
                    c.get("data").and_then(|v| v.as_str()).map(String::from)
                }
                _ => None,
            }
        });

    let files_url = format!("{}/learner/lessons/{}/files", API_BASE, lesson_slug);
    let files = match session.client.get(&files_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let files_text = resp.text().await.unwrap_or_default();
            let files_body: serde_json::Value =
                serde_json::from_str(&files_text).unwrap_or(serde_json::Value::Array(vec![]));

            let files_arr = match &files_body {
                serde_json::Value::Array(arr) => arr.clone(),
                _ => files_body
                    .get("files")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            };

            files_arr
                .iter()
                .filter_map(|f| {
                    let file_name = f
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("file")
                        .to_string();

                    let file_url = f.get("url").and_then(|v| v.as_str()).map(|u| {
                        if u.starts_with("http") {
                            u.to_string()
                        } else {
                            format!("{}{}", API_BASE, u)
                        }
                    });

                    file_url.map(|url| AlpaclassFile {
                        name: file_name,
                        url,
                    })
                })
                .collect()
        }
        _ => vec![],
    };

    Ok(AlpaclassLessonDetail {
        slug: lesson_slug.to_string(),
        name,
        html_content,
        video_url,
        files,
    })
}

pub async fn save_session(session: &AlpaclassSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        subdomain: session.subdomain.clone(),
        origin_url: session.origin_url.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[alpaclass] session saved for {}", session.subdomain);
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<AlpaclassSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.subdomain, &saved.origin_url)?;

    tracing::info!("[alpaclass] session loaded for {}", saved.subdomain);

    Ok(Some(AlpaclassSession {
        token: saved.token,
        subdomain: saved.subdomain,
        origin_url: saved.origin_url,
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
