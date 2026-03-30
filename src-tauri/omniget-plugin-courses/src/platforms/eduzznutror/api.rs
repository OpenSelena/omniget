use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct NutrorSession {
    pub token: String,
    pub refresh_token: Option<String>,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub refresh_token: Option<String>,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrorCourse {
    pub id: String,
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrorModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<NutrorLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrorLesson {
    pub id: String,
    pub name: String,
    pub hash: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrorLessonDetail {
    pub id: String,
    pub name: String,
    pub video_url: Option<String>,
    pub files: Vec<NutrorFile>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrorFile {
    pub name: String,
    pub url: String,
}

fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert("FrontVersion", HeaderValue::from_static("1458"));

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
    Ok(data_dir.join("omniget").join("nutror_session.json"))
}

pub async fn refresh_token(session: &NutrorSession) -> anyhow::Result<NutrorSession> {
    let refresh = session
        .refresh_token
        .as_deref()
        .ok_or_else(|| anyhow!("No refresh token available"))?;

    let temp_client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let resp = temp_client
        .post("https://learner-api.nutror.com/oauth/refresh?startWhen=requestFailed")
        .header("Authorization", format!("Bearer {}", session.token))
        .header("RefreshToken", refresh)
        .header("Accept", "application/json")
        .header("FrontVersion", "1458")
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Token refresh failed (status {}): {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let new_token = body
        .get("token")
        .or_else(|| body.get("access_token"))
        .or_else(|| body.get("newAuthToken"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No token in refresh response"))?
        .to_string();

    let new_refresh = body
        .get("refreshToken")
        .or_else(|| body.get("refresh_token"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| session.refresh_token.clone());

    let client = build_client(&new_token)?;

    Ok(NutrorSession {
        token: new_token,
        refresh_token: new_refresh,
        client,
    })
}

pub async fn validate_token(session: &NutrorSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://learner-api.nutror.com/learner/course/search?page=1&size=1&status=seeAll&showShelf=true")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &NutrorSession) -> anyhow::Result<Vec<NutrorCourse>> {
    let mut all_courses = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "https://learner-api.nutror.com/learner/course/search?page={}&size=20&status=seeAll&showShelf=true",
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

        let data = body
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if data.is_empty() {
            break;
        }

        for item in &data {
            let id = item
                .get("id")
                .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
                .unwrap_or_default();

            let name = item
                .get("title")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let hash = item
                .get("hash")
                .or_else(|| item.get("courseHash"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            all_courses.push(NutrorCourse {
                id,
                name,
                hash,
            });
        }

        let total_pages = body
            .get("totalPages")
            .or_else(|| body.get("total_pages"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        if page as u64 >= total_pages {
            break;
        }

        page += 1;
    }

    Ok(all_courses)
}

pub async fn get_modules(
    session: &NutrorSession,
    course_hash: &str,
) -> anyhow::Result<Vec<NutrorModule>> {
    let url = format!(
        "https://learner-api.nutror.com/learner/course/{}/lessons/v2",
        course_hash
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

    let data = body
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            body.as_array().cloned().unwrap_or_default()
        });

    let mut modules = Vec::new();

    for (i, item) in data.iter().enumerate() {
        let id = item
            .get("id")
            .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
            .unwrap_or_default();

        let name = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let order = item
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(i as i64);

        let mut lessons = Vec::new();

        let lessons_arr = item
            .get("lessons")
            .or_else(|| item.get("items"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for (j, lesson_val) in lessons_arr.iter().enumerate() {
            let lesson_id = lesson_val
                .get("id")
                .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
                .unwrap_or_default();

            let lesson_name = lesson_val
                .get("title")
                .or_else(|| lesson_val.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_hash = lesson_val
                .get("hash")
                .or_else(|| lesson_val.get("lessonHash"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_order = lesson_val
                .get("order")
                .and_then(|v| v.as_i64())
                .unwrap_or(j as i64);

            lessons.push(NutrorLesson {
                id: lesson_id,
                name: lesson_name,
                hash: lesson_hash,
                order: lesson_order,
            });
        }

        lessons.sort_by_key(|l| l.order);

        modules.push(NutrorModule {
            id,
            name,
            order,
            lessons,
        });
    }

    modules.sort_by_key(|m| m.order);

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &NutrorSession,
    lesson_hash: &str,
) -> anyhow::Result<NutrorLessonDetail> {
    let url = format!(
        "https://learner-api.nutror.com/learner/lessons/{}",
        lesson_hash
    );

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

    let lesson = body.get("data").unwrap_or(&body);

    let id = lesson
        .get("id")
        .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
        .unwrap_or_default();

    let name = lesson
        .get("title")
        .or_else(|| lesson.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut video_url: Option<String> = None;
    let mut files = Vec::new();
    let mut description: Option<String> = None;

    let contents = lesson
        .get("contents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for content in &contents {
        let type_obj = content.get("type");
        let type_id = type_obj
            .and_then(|t| t.get("id").and_then(|v| v.as_i64()))
            .unwrap_or(0);

        let embed = content
            .get("content")
            .or_else(|| content.get("embed"))
            .or_else(|| content.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if type_id == 4 && description.is_none() && !embed.trim().is_empty() {
            description = Some(embed.to_string());
        }

        if video_url.is_none() {
            match type_id {
                1 => {
                    let yt_id = extract_youtube_id(embed);
                    if let Some(yt_id) = yt_id {
                        video_url = Some(format!("https://www.youtube.com/watch?v={}", yt_id));
                    }
                }
                9 => {
                    let safe_url = extract_safevideo_url(embed);
                    if let Some(url) = safe_url {
                        video_url = Some(url);
                    }
                }
                _ => {
                    if embed.contains("youtube.com") || embed.contains("youtu.be") {
                        video_url = Some(embed.to_string());
                    } else if embed.contains("vimeo.com") {
                        video_url = Some(embed.to_string());
                    } else if embed.contains(".mp4") || embed.contains(".m3u8") {
                        video_url = Some(embed.to_string());
                    }
                }
            }
        }
    }

    let lesson_files = lesson
        .get("lesson_files")
        .or_else(|| lesson.get("files"))
        .or_else(|| lesson.get("attachments"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for file in &lesson_files {
        let file_name = file
            .get("name")
            .or_else(|| file.get("title"))
            .or_else(|| file.get("filename"))
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();

        let file_url = file
            .get("url")
            .or_else(|| file.get("download_url"))
            .or_else(|| file.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !file_url.is_empty() {
            files.push(NutrorFile {
                name: file_name,
                url: file_url,
            });
        }
    }

    if description.is_none() {
        description = lesson
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(String::from);
    }

    Ok(NutrorLessonDetail {
        id,
        name,
        video_url,
        files,
        description,
    })
}

fn extract_youtube_id(embed: &str) -> Option<String> {
    if let Some(pos) = embed.find("youtube.com/embed/") {
        let rest = &embed[pos + 18..];
        let end = rest.find(|c: char| c == '"' || c == '?' || c == '&' || c == '\'').unwrap_or(rest.len());
        let id = &rest[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    if let Some(pos) = embed.find("watch?v=") {
        let rest = &embed[pos + 8..];
        let end = rest.find(|c: char| c == '&' || c == '"' || c == '\'').unwrap_or(rest.len());
        let id = &rest[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    if let Some(pos) = embed.find("youtu.be/") {
        let rest = &embed[pos + 9..];
        let end = rest.find(|c: char| c == '?' || c == '"' || c == '\'').unwrap_or(rest.len());
        let id = &rest[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

fn extract_safevideo_url(embed: &str) -> Option<String> {
    if let Some(pos) = embed.find("src=\"") {
        let rest = &embed[pos + 5..];
        let end = rest.find('"').unwrap_or(rest.len());
        let url = &rest[..end];
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    if let Some(pos) = embed.find("src='") {
        let rest = &embed[pos + 5..];
        let end = rest.find('\'').unwrap_or(rest.len());
        let url = &rest[..end];
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    if embed.starts_with("http") && !embed.contains('<') {
        return Some(embed.to_string());
    }
    None
}

pub async fn save_session(session: &NutrorSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        refresh_token: session.refresh_token.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[nutror] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<NutrorSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[nutror] session loaded");

    Ok(Some(NutrorSession {
        token: saved.token,
        refresh_token: saved.refresh_token,
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
