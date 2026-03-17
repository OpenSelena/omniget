use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct AreaDeMembrosSession {
    pub cookies: String,
    pub site_url: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub cookies: String,
    pub site_url: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDeMembrosApiCourse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDeMembrosModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<AreaDeMembrosLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDeMembrosLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub url: String,
}

fn build_client(cookies: &str, site_url: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cookie",
        HeaderValue::from_str(cookies)?,
    );
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", site_url))?,
    );
    headers.insert("Accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"));

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
    Ok(data_dir.join("omniget").join("areademembros_session.json"))
}

pub async fn validate_token(session: &AreaDeMembrosSession) -> anyhow::Result<bool> {
    let url = format!("{}/area/vitrine", session.site_url);
    let resp = session.client.get(&url).send().await?;
    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &AreaDeMembrosSession) -> anyhow::Result<Vec<AreaDeMembrosApiCourse>> {
    let url = format!("{}/area/vitrine", session.site_url);
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

    let mut courses = Vec::new();

    for line in body_text.lines() {
        if let Some(pos) = line.find("id=\"grupo-vitrine-") {
            let rest = &line[pos + 18..];
            if let Some(end) = rest.find('"') {
                let id = rest[..end].to_string();
                let name = extract_text_after(rest, ">")
                    .unwrap_or_else(|| id.clone());
                courses.push(AreaDeMembrosApiCourse {
                    id: id.clone(),
                    name: if name.trim().is_empty() { id } else { name.trim().to_string() },
                });
            }
        }
    }

    if courses.is_empty() {
        for line in body_text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("grupo-vitrine") || trimmed.contains("vitrine-produto") {
                if let Some(href) = extract_attr(trimmed, "href") {
                    let id = href.rsplit('/').next().unwrap_or(&href).to_string();
                    let name = extract_text_content(trimmed).unwrap_or_else(|| id.clone());
                    if !courses.iter().any(|c: &AreaDeMembrosApiCourse| c.id == id) {
                        courses.push(AreaDeMembrosApiCourse { id, name });
                    }
                }
            }
        }
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &AreaDeMembrosSession,
    course_id: &str,
) -> anyhow::Result<Vec<AreaDeMembrosModule>> {
    let url = format!("{}/area/produto/{}", session.site_url, course_id);
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

    let mut modules = Vec::new();
    let mut current_module: Option<(String, String, Vec<AreaDeMembrosLesson>)> = None;
    let mut module_order = 0i64;
    let mut lesson_order = 0i64;

    for line in body_text.lines() {
        let trimmed = line.trim();

        if trimmed.contains("section-group-titulo") && trimmed.contains("data-target") {
            if let Some((id, name, lessons)) = current_module.take() {
                modules.push(AreaDeMembrosModule {
                    id,
                    name,
                    order: module_order,
                    lessons,
                });
            }
            module_order += 1;
            lesson_order = 0;
            let mod_id = extract_attr(trimmed, "data-target")
                .unwrap_or_else(|| format!("mod-{}", module_order));
            let mod_name = extract_text_content(trimmed)
                .unwrap_or_else(|| format!("Module {}", module_order));
            current_module = Some((mod_id, mod_name, Vec::new()));
        }

        if trimmed.contains("layer-link") && trimmed.contains("/area/produto/item/") {
            if let Some(href) = extract_attr(trimmed, "href") {
                lesson_order += 1;
                let lesson_id = href.rsplit('/').next().unwrap_or(&href).to_string();
                let lesson_name = extract_text_content(trimmed)
                    .unwrap_or_else(|| format!("Lesson {}", lesson_order));
                let full_url = if href.starts_with("http") {
                    href
                } else {
                    format!("{}{}", session.site_url, href)
                };
                if let Some(ref mut m) = current_module {
                    m.2.push(AreaDeMembrosLesson {
                        id: lesson_id,
                        name: lesson_name,
                        order: lesson_order,
                        url: full_url,
                    });
                }
            }
        }
    }

    if let Some((id, name, lessons)) = current_module.take() {
        modules.push(AreaDeMembrosModule {
            id,
            name,
            order: module_order,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_lesson_video_url(
    session: &AreaDeMembrosSession,
    lesson_url: &str,
) -> anyhow::Result<Option<String>> {
    let resp = session.client.get(lesson_url).send().await?;
    let body = resp.text().await?;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.contains("video-container") || trimmed.contains("<iframe") {
            if let Some(src) = extract_attr(trimmed, "src") {
                if src.contains("youtube") || src.contains("vimeo") || src.contains("player") || src.contains("embed") {
                    return Ok(Some(src));
                }
            }
        }
    }

    if let Some(pos) = body.find("<iframe") {
        let rest = &body[pos..];
        if let Some(src) = extract_attr(rest, "src") {
            return Ok(Some(src));
        }
    }

    Ok(None)
}

fn extract_attr(html: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(pos) = html.find(&pattern) {
        let rest = &html[pos + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let pattern2 = format!("{}='", attr);
    if let Some(pos) = html.find(&pattern2) {
        let rest = &html[pos + pattern2.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_text_content(html: &str) -> Option<String> {
    if let Some(pos) = html.rfind('>') {
        let rest = &html[pos + 1..];
        if let Some(end) = rest.find('<') {
            let text = rest[..end].trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn extract_text_after(html: &str, pattern: &str) -> Option<String> {
    if let Some(pos) = html.find(pattern) {
        let rest = &html[pos + pattern.len()..];
        if let Some(end) = rest.find('<') {
            let text = rest[..end].trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

pub async fn save_session(session: &AreaDeMembrosSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        cookies: session.cookies.clone(),
        site_url: session.site_url.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[areademembros] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<AreaDeMembrosSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookies, &saved.site_url)?;

    tracing::info!("[areademembros] session loaded");

    Ok(Some(AreaDeMembrosSession {
        cookies: saved.cookies,
        site_url: saved.site_url,
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
