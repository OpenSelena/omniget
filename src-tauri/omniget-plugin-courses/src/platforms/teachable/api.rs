use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "ktor-client";
const CLIENT_TOKEN: &str = "9e44e885ac601aae4ee7109baec9ee0a503bfbb4fd11cbcb7d1de9d5e84f395b37d1521b08add19c2604dbe3c1d6c986bbd62a2513884e04e5b40704e77944e4";
const BASE_URL: &str = "https://mobile-service.learning.teachable.com/api/v1";

#[derive(Clone)]
pub struct TeachableSession {
    pub token: String,
    pub school_id: Option<String>,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub school_id: Option<String>,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableSchool {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableCourse {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub school_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<TeachableLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableLessonDetail {
    pub id: String,
    pub name: String,
    pub video_url: Option<String>,
    pub video_referer: Option<String>,
    pub files: Vec<TeachableFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableFile {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "CLIENT-TOKEN",
        HeaderValue::from_static(CLIENT_TOKEN),
    );
    headers.insert(
        "X-APP-VERSION",
        HeaderValue::from_static("2.3.0"),
    );
    headers.insert(
        "X-DEVICE-OS",
        HeaderValue::from_static("Android 35"),
    );
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
    Ok(data_dir.join("omniget").join("teachable_session.json"))
}

pub async fn request_otp(email: &str) -> anyhow::Result<()> {
    let temp_client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let payload = serde_json::json!({
        "email": email,
    });

    let resp = temp_client
        .post("https://sso.teachable.com/api/v2/auth/otp/request")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();

    if !status.is_success() {
        let body_text = resp.text().await?;
        return Err(anyhow!(
            "OTP request failed (status {}): {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    Ok(())
}

pub async fn verify_otp(email: &str, otp_code: &str) -> anyhow::Result<TeachableSession> {
    let temp_client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let payload = serde_json::json!({
        "email": email,
        "otp_code": otp_code,
    });

    let resp = temp_client
        .post("https://sso.teachable.com/api/v2/auth/otp/verify")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "OTP verification failed (status {}): {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let token = body
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("token not found in OTP verify response"))?
        .to_string();

    let client = build_client(&token)?;

    Ok(TeachableSession {
        token,
        school_id: None,
        client,
    })
}

pub async fn validate_token(session: &TeachableSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(&format!("{}/schools", BASE_URL))
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_schools(session: &TeachableSession) -> anyhow::Result<Vec<TeachableSchool>> {
    let resp = session
        .client
        .get(&format!("{}/schools", BASE_URL))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_schools returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let schools_arr = body
        .get("schools")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();

    let mut schools = Vec::new();

    for item in &schools_arr {
        let id = item
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let image_url = item
            .get("image_url")
            .or_else(|| item.get("logo_url"))
            .and_then(|v| v.as_str())
            .map(String::from);

        schools.push(TeachableSchool {
            id,
            name,
            image_url,
        });
    }

    Ok(schools)
}

pub async fn list_courses(
    session: &TeachableSession,
    school_id: &str,
) -> anyhow::Result<Vec<TeachableCourse>> {
    let url = format!("{}/schools/{}/courses", BASE_URL, school_id);

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

    let courses_arr = body
        .get("courses")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();

    for item in &courses_arr {
        let id = item
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let image_url = item
            .get("image_url")
            .or_else(|| item.get("thumbnail_url"))
            .and_then(|v| v.as_str())
            .map(String::from);

        courses.push(TeachableCourse {
            id,
            name,
            image_url,
            school_id: school_id.to_string(),
        });
    }

    Ok(courses)
}

pub async fn get_syllabus(
    session: &TeachableSession,
    school_id: &str,
    course_id: &str,
) -> anyhow::Result<Vec<TeachableModule>> {
    let url = format!(
        "{}/schools/{}/courses/{}/syllabus",
        BASE_URL, school_id, course_id
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_syllabus returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let sections_arr = body
        .get("sections")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (i, section) in sections_arr.iter().enumerate() {
        let section_id = section
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => format!("{}", i),
            })
            .unwrap_or_else(|| format!("{}", i));

        let section_name = section
            .get("name")
            .or_else(|| section.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let lectures_arr = section
            .get("lectures")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (j, lecture) in lectures_arr.iter().enumerate() {
            let lecture_id = lecture
                .get("id")
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => format!("{}", j),
                })
                .unwrap_or_else(|| format!("{}", j));

            let lecture_name = lecture
                .get("name")
                .or_else(|| lecture.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            lessons.push(TeachableLesson {
                id: lecture_id,
                name: lecture_name,
                order: j as i64,
            });
        }

        modules.push(TeachableModule {
            id: section_id,
            name: section_name,
            order: i as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_lecture_detail(
    session: &TeachableSession,
    school_id: &str,
    course_id: &str,
    lecture_id: &str,
) -> anyhow::Result<TeachableLessonDetail> {
    let url = format!(
        "{}/schools/{}/courses/{}/lectures/{}",
        BASE_URL, school_id, course_id, lecture_id
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_lecture_detail returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let lecture = body.get("lecture").unwrap_or(&body);

    let id = lecture
        .get("id")
        .map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => lecture_id.to_string(),
        })
        .unwrap_or_else(|| lecture_id.to_string());

    let name = lecture
        .get("name")
        .or_else(|| lecture.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let attachments = lecture
        .get("attachments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut video_url = None;
    let mut video_referer = None;
    let mut files = Vec::new();

    for att in &attachments {
        let kind = att
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match kind {
            "video" => {
                let data = att.get("data").unwrap_or(att);
                video_url = data
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                video_referer = data
                    .get("referer")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }
            "pdf_embed" | "generic" => {
                let data = att.get("data").unwrap_or(att);
                let file_id = att
                    .get("id")
                    .map(|v| match v {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();

                let file_name = data
                    .get("name")
                    .or_else(|| data.get("filename"))
                    .or_else(|| att.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("file")
                    .to_string();

                let file_url = data
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                files.push(TeachableFile {
                    id: file_id,
                    name: file_name,
                    url: file_url,
                });
            }
            _ => {}
        }
    }

    Ok(TeachableLessonDetail {
        id,
        name,
        video_url,
        video_referer,
        files,
    })
}

pub async fn save_session(session: &TeachableSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        school_id: session.school_id.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[teachable] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<TeachableSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[teachable] session loaded");

    Ok(Some(TeachableSession {
        token: saved.token,
        school_id: saved.school_id,
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
