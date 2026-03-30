use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct SkoolSession {
    pub cookie_token: String,
    pub build_id: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub cookie_token: String,
    pub build_id: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolGroup {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub image_url: Option<String>,
    pub num_courses: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<SkoolLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolLessonDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub video_url: Option<String>,
    pub files: Vec<SkoolFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolFile {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

fn build_client(cookie_token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!("skooltok={}", cookie_token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.skool.com/"),
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
    Ok(data_dir.join("omniget").join("skool_session.json"))
}

async fn fetch_build_id(client: &reqwest::Client) -> anyhow::Result<String> {
    let resp = client
        .get("https://www.skool.com")
        .send()
        .await?;

    let html = resp.text().await?;

    let re = regex::Regex::new(r#""buildId"\s*:\s*"([^"]+)""#)?;
    let build_id = re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("Could not extract buildId from Skool HTML"))?;

    Ok(build_id)
}

pub async fn authenticate(email: &str, password: &str) -> anyhow::Result<SkoolSession> {
    let temp_client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let payload = serde_json::json!({
        "email": email,
        "password": password,
    });

    let resp = temp_client
        .post("https://api2.skool.com/auth/login")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();

    let mut cookie_token = String::new();
    for cookie in resp.cookies() {
        if cookie.name() == "skooltok" {
            cookie_token = cookie.value().to_string();
        }
    }

    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Authentication failed (status {}): {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    if cookie_token.is_empty() {
        let body: serde_json::Value = serde_json::from_str(&body_text).unwrap_or_default();
        cookie_token = body
            .get("token")
            .or_else(|| body.get("skooltok"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    if cookie_token.is_empty() {
        return Err(anyhow!("No skooltok cookie or token in login response"));
    }

    let client = build_client(&cookie_token)?;
    let build_id = fetch_build_id(&client).await?;

    Ok(SkoolSession {
        cookie_token,
        build_id,
        client,
    })
}

pub async fn validate_token(session: &SkoolSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://api2.skool.com/self/groups?limit=1&prefs=false")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_groups(session: &SkoolSession) -> anyhow::Result<Vec<SkoolGroup>> {
    let mut all_groups = Vec::new();

    let url = "https://api2.skool.com/self/groups?limit=50&prefs=false";

    let resp = session.client.get(url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_groups returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let groups_arr = body
        .get("groups")
        .or_else(|| body.as_array().map(|_| &body))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for item in &groups_arr {
        let metadata = item.get("metadata").unwrap_or(item);

        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = metadata
            .get("display_name")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("display_name").and_then(|v| v.as_str()))
            .unwrap_or(&slug)
            .to_string();

        let image_url = metadata
            .get("cover_small_url")
            .and_then(|v| v.as_str())
            .map(String::from);

        let num_courses = metadata
            .get("num_courses")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        all_groups.push(SkoolGroup {
            id,
            slug,
            name,
            image_url,
            num_courses,
        });
    }

    Ok(all_groups)
}

pub async fn get_group_courses(
    session: &SkoolSession,
    group_slug: &str,
) -> anyhow::Result<Vec<SkoolModule>> {
    let url = format!(
        "https://www.skool.com/_next/data/{}/{}/classroom.json?group={}",
        session.build_id, group_slug, group_slug
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_group_courses returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let courses_arr = body
        .pointer("/pageProps/allCourses")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (i, course) in courses_arr.iter().enumerate() {
        let course_id = course
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let course_slug = course
            .get("slug")
            .or_else(|| course.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let metadata = course.get("metadata").unwrap_or(course);

        let course_name = metadata
            .get("title")
            .or_else(|| metadata.get("display_name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&course_slug)
            .to_string();

        let lessons = get_course_lessons(session, group_slug, &course_slug).await.unwrap_or_default();

        modules.push(SkoolModule {
            id: if course_id.is_empty() { course_slug } else { course_id },
            name: course_name,
            order: i as i64,
            lessons,
        });
    }

    Ok(modules)
}

async fn get_course_lessons(
    session: &SkoolSession,
    group_slug: &str,
    course_slug: &str,
) -> anyhow::Result<Vec<SkoolLesson>> {
    let url = format!(
        "https://www.skool.com/_next/data/{}/{}/classroom/{}.json?group={}&course={}",
        session.build_id, group_slug, course_slug, group_slug, course_slug
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_course_lessons returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let children = body
        .pointer("/pageProps/course/children")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lessons = Vec::new();

    for (i, child) in children.iter().enumerate() {
        let metadata = child.get("metadata").unwrap_or(child);

        let lesson_id = child
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let lesson_name = metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        lessons.push(SkoolLesson {
            id: if lesson_id.is_empty() { format!("{}", i) } else { lesson_id },
            name: lesson_name,
            order: i as i64,
        });
    }

    Ok(lessons)
}

pub async fn get_lesson_detail(
    _session: &SkoolSession,
    _group_slug: &str,
    _course_slug: &str,
    lesson_id: &str,
    children: &[serde_json::Value],
) -> anyhow::Result<SkoolLessonDetail> {
    let child = children
        .iter()
        .find(|c| {
            c.get("id")
                .and_then(|v| v.as_str())
                .map(|id| id == lesson_id)
                .unwrap_or(false)
        })
        .cloned();

    let metadata = child
        .as_ref()
        .and_then(|c| c.get("metadata"))
        .unwrap_or(&serde_json::Value::Null);

    let name = metadata
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let description = metadata
        .get("description")
        .or_else(|| metadata.get("content"))
        .or_else(|| metadata.get("body"))
        .and_then(|v| {
            if v.is_string() {
                v.as_str().map(String::from)
            } else {
                Some(serde_json::to_string_pretty(v).unwrap_or_default())
            }
        })
        .filter(|s| !s.trim().is_empty());

    let video_url = metadata
        .get("videoLink")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut files = Vec::new();

    if let Some(attachments) = metadata.get("attachments").and_then(|v| v.as_array()) {
        for att in attachments {
            let file_id = att
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let file_name = att
                .get("name")
                .or_else(|| att.get("filename"))
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_string();

            files.push(SkoolFile {
                id: file_id,
                name: file_name,
                url: None,
            });
        }
    }

    Ok(SkoolLessonDetail {
        id: lesson_id.to_string(),
        name,
        description,
        video_url,
        files,
    })
}

pub async fn get_file_download_url(
    session: &SkoolSession,
    file_id: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://api2.skool.com/files/{}/download-url?expire=28800",
        file_id
    );

    let resp = session.client.post(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_file_download_url returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let download_url = body
        .get("url")
        .or_else(|| body.get("download_url"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No download URL in file response"))?
        .to_string();

    Ok(download_url)
}

pub async fn save_session(session: &SkoolSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        cookie_token: session.cookie_token.clone(),
        build_id: session.build_id.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[skool] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<SkoolSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookie_token)?;

    tracing::info!("[skool] session loaded");

    Ok(Some(SkoolSession {
        cookie_token: saved.cookie_token,
        build_id: saved.build_id,
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
