use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct FluencySession {
    pub token: String,
    pub email: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub email: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluencyCourse {
    pub id: String,
    pub name: String,
    pub program_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluencyModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<FluencyLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluencyLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluencyLessonDetail {
    pub id: String,
    pub name: String,
    pub video_url: Option<String>,
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
        HeaderValue::from_static("https://academy.fluency.io"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://academy.fluency.io/"),
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
    Ok(data_dir.join("omniget").join("fluency_session.json"))
}

pub async fn authenticate(email: &str, password: &str) -> anyhow::Result<FluencySession> {
    let temp_client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let payload = serde_json::json!({
        "email": email,
        "password": password,
    });

    let resp = temp_client
        .post("https://accounts-api.fluency.io/auth/sign-in/")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Authentication failed (status {}): {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let token = body
        .get("access_token")
        .or_else(|| body.get("accessToken"))
        .or_else(|| body.get("token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("access_token not found in login response"))?
        .to_string();

    let client = build_client(&token)?;

    Ok(FluencySession {
        token,
        email: email.to_string(),
        client,
    })
}

pub async fn validate_token(session: &FluencySession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://bifrost.fluencyacademy.io/programs")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &FluencySession) -> anyhow::Result<Vec<FluencyCourse>> {
    let resp = session
        .client
        .get("https://bifrost.fluencyacademy.io/programs")
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_programs returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let programs: serde_json::Value = serde_json::from_str(&body_text)?;
    let programs_arr = programs.as_array().cloned().unwrap_or_default();

    let mut all_courses = Vec::new();

    for program in &programs_arr {
        let program_id = program
            .get("id")
            .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
            .unwrap_or_default();

        let program_name = program
            .get("name")
            .or_else(|| program.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        for endpoint in &["main", "complement"] {
            let url = format!(
                "https://bifrost.fluencyacademy.io/programs/{}/courses/{}",
                program_id, endpoint
            );

            let resp = session.client.get(&url).send().await;

            if let Ok(resp) = resp {
                if resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    let courses: serde_json::Value =
                        serde_json::from_str(&text).unwrap_or_default();

                    let courses_arr = courses.as_array().cloned().unwrap_or_default();

                    for course in &courses_arr {
                        let id = course
                            .get("id")
                            .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
                            .unwrap_or_default();

                        let name = course
                            .get("name")
                            .or_else(|| course.get("title"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let display_name = if program_name.is_empty() {
                            name
                        } else {
                            format!("{} - {}", program_name, name)
                        };

                        all_courses.push(FluencyCourse {
                            id,
                            name: display_name,
                            program_id: program_id.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(all_courses)
}

pub async fn get_modules(
    session: &FluencySession,
    course_id: &str,
) -> anyhow::Result<Vec<FluencyModule>> {
    let url = format!(
        "https://bifrost.fluencyacademy.io/courses/{}/learning-path?size=50",
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

    let content_arr = body
        .get("content")
        .or_else(|| body.get("modules"))
        .or_else(|| body.get("units"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            if body.is_array() {
                body.as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            }
        });

    let mut modules = Vec::new();

    for (i, item) in content_arr.iter().enumerate() {
        let id = item
            .get("id")
            .and_then(|v| v.as_str().map(String::from).or_else(|| Some(v.to_string())))
            .unwrap_or_default();

        let name = item
            .get("name")
            .or_else(|| item.get("title"))
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
            .or_else(|| item.get("units"))
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
                .get("name")
                .or_else(|| lesson_val.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_order = lesson_val
                .get("order")
                .and_then(|v| v.as_i64())
                .unwrap_or(j as i64);

            lessons.push(FluencyLesson {
                id: lesson_id,
                name: lesson_name,
                order: lesson_order,
            });
        }

        lessons.sort_by_key(|l| l.order);

        modules.push(FluencyModule {
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
    session: &FluencySession,
    lesson_id: &str,
) -> anyhow::Result<FluencyLessonDetail> {
    let url = format!(
        "https://bifrost.fluencyacademy.io/lessons/{}/tasks",
        lesson_id
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

    let tasks = body
        .as_array()
        .or_else(|| body.get("tasks").and_then(|v| v.as_array()))
        .cloned()
        .unwrap_or_default();

    let mut video_url: Option<String> = None;
    let mut lesson_name = String::new();

    for task in &tasks {
        if lesson_name.is_empty() {
            if let Some(n) = task.get("name").or_else(|| task.get("title")).and_then(|v| v.as_str()) {
                lesson_name = n.to_string();
            }
        }

        let meta = task.get("meta");

        if let Some(meta) = meta {
            let sources = meta.get("sources").and_then(|v| v.as_array());

            if let Some(sources) = sources {
                let mut best_mp4: Option<String> = None;
                let mut best_height: u64 = 0;
                let mut hls_url: Option<String> = None;

                for source in sources {
                    let src_url = source
                        .get("src")
                        .or_else(|| source.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let src_type = source
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let quality = source
                        .get("quality")
                        .or_else(|| source.get("label"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let height: u64 = quality
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0);

                    if src_type.contains("mp4") || src_url.contains(".mp4") {
                        if height > best_height {
                            best_height = height;
                            best_mp4 = Some(src_url.to_string());
                        }
                    } else if src_type.contains("m3u8")
                        || src_type.contains("hls")
                        || src_url.contains(".m3u8")
                    {
                        hls_url = Some(src_url.to_string());
                    }
                }

                if best_mp4.is_some() {
                    video_url = best_mp4;
                } else if hls_url.is_some() {
                    video_url = hls_url;
                }
            }

            if video_url.is_none() {
                if let Some(url) = meta.get("url").or_else(|| meta.get("video_url")).and_then(|v| v.as_str()) {
                    if !url.is_empty() {
                        video_url = Some(url.to_string());
                    }
                }
            }
        }

        if video_url.is_some() {
            break;
        }
    }

    Ok(FluencyLessonDetail {
        id: lesson_id.to_string(),
        name: lesson_name,
        video_url,
    })
}

pub async fn save_session(session: &FluencySession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        email: session.email.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[fluency] session saved for {}", session.email);
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<FluencySession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[fluency] session loaded for {}", saved.email);

    Ok(Some(FluencySession {
        token: saved.token,
        email: saved.email,
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
