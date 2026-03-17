use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const BASE_URL: &str = "https://app.rocketseat.com.br";
const API_URL: &str = "https://skylab-api.rocketseat.com.br";
const BUNNY_LIBRARY_ID: &str = "212524";

#[derive(Clone)]
pub struct RocketseatSession {
    pub token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketseatCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketseatModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<RocketseatLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketseatLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub slug: String,
    pub lesson_type: String,
    pub video_id: Option<String>,
    pub duration: Option<i64>,
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
        HeaderValue::from_static(BASE_URL),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://app.rocketseat.com.br/"),
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
    Ok(data_dir.join("omniget").join("rocketseat_session.json"))
}

pub fn create_session(token: &str) -> anyhow::Result<RocketseatSession> {
    let client = build_client(token)?;
    Ok(RocketseatSession {
        token: token.to_string(),
        client,
    })
}

pub async fn validate_token(session: &RocketseatSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(&format!("{}/v2/users/me", API_URL))
        .send()
        .await?;
    Ok(resp.status().is_success())
}

pub async fn search_courses(
    session: &RocketseatSession,
    query: &str,
) -> anyhow::Result<Vec<RocketseatCourse>> {
    let mut all_courses = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut page = 1u32;

    loop {
        let url = format!("{}/v2/search/multi-search", API_URL);
        let resp = session
            .client
            .get(&url)
            .query(&[("query", query), ("page", &page.to_string())])
            .header("Host", "skylab-api.rocketseat.com.br")
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            if all_courses.is_empty() {
                return Err(anyhow!(
                    "search_courses returned status {}: {}",
                    status,
                    &body_text[..body_text.len().min(300)]
                ));
            }
            break;
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;

        let journeys = body
            .get("journeys")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if journeys.is_empty() {
            break;
        }

        let mut new_count = 0;
        for item in &journeys {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if id.is_empty() || seen.contains(&id) {
                continue;
            }
            seen.insert(id.clone());
            new_count += 1;

            let name = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let slug = item
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);

            all_courses.push(RocketseatCourse {
                id,
                name,
                slug,
                description,
            });
        }

        if new_count == 0 {
            break;
        }

        let has_more = body
            .pointer("/meta/journeys/hasMore")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !has_more || page > 50 {
            break;
        }

        page += 1;
    }

    Ok(all_courses)
}

pub async fn list_courses(
    session: &RocketseatSession,
) -> anyhow::Result<Vec<RocketseatCourse>> {
    let mut courses = Vec::new();

    for endpoint in &[
        "/v2/journeys",
        "/v2/users/me/journeys",
    ] {
        let url = format!("{}{}", API_URL, endpoint);
        let resp = match session.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let items = match &body {
            serde_json::Value::Array(arr) => arr.clone(),
            serde_json::Value::Object(map) => {
                for key in &["journeys", "items", "data"] {
                    if let Some(serde_json::Value::Array(arr)) = map.get(*key) {
                        courses = parse_journey_items(arr);
                        if !courses.is_empty() {
                            return Ok(courses);
                        }
                    }
                }
                continue;
            }
            _ => continue,
        };

        courses = parse_journey_items(&items);
        if !courses.is_empty() {
            return Ok(courses);
        }
    }

    if courses.is_empty() {
        courses = search_courses(session, "").await.unwrap_or_default();
    }

    Ok(courses)
}

fn parse_journey_items(items: &[serde_json::Value]) -> Vec<RocketseatCourse> {
    let mut courses = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in items {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if id.is_empty() || seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let name = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = item
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        courses.push(RocketseatCourse {
            id,
            name,
            slug,
            description,
        });
    }

    courses
}

fn parse_rsc_response(text: &str) -> serde_json::Value {
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 {
            continue;
        }

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(parts[1]) {
            if let Some(arr) = data.as_array() {
                if arr.len() > 3 {
                    if let Some(payload) = arr.get(3).and_then(|v| v.as_object()) {
                        let payload_val = serde_json::Value::Object(payload.clone());
                        if payload_val.get("journeyContents").is_some()
                            || payload_val.get("lessonGroups").is_some()
                        {
                            return payload_val;
                        }
                        if let Some(inner) = payload_val.get("data").and_then(|v| v.as_object()) {
                            let inner_val = serde_json::Value::Object(inner.clone());
                            if inner_val.get("journeyContents").is_some()
                                || inner_val.get("lessonGroups").is_some()
                            {
                                return inner_val;
                            }
                        }
                    }
                }
            }
        }
    }

    serde_json::Value::Object(serde_json::Map::new())
}

pub async fn get_course_content(
    session: &RocketseatSession,
    course: &RocketseatCourse,
) -> anyhow::Result<Vec<RocketseatModule>> {
    let url = format!("{}/jornada/{}/conteudos", BASE_URL, course.slug);

    let resp = session
        .client
        .get(&url)
        .query(&[("_rsc", "1")])
        .header("RSC", "1")
        .header("Accept", "*/*")
        .header("Host", "app.rocketseat.com.br")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "get_course_content returned status {}",
            resp.status()
        ));
    }

    let text = resp.text().await?;
    let data = parse_rsc_response(&text);

    let nodes = data
        .pointer("/journeyContents/nodes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();
    let mut module_idx = 0i64;

    for node in &nodes {
        let sub_contents = node
            .get("contents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| vec![node.clone()]);

        for sub in &sub_contents {
            let module_slug = sub
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if module_slug.is_empty() {
                continue;
            }

            let module_title = sub
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(module_slug)
                .to_string();

            let lessons = fetch_module_lessons(session, &course.slug, module_slug).await?;

            if !lessons.is_empty() {
                module_idx += 1;
                modules.push(RocketseatModule {
                    id: sub
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(module_slug)
                        .to_string(),
                    name: module_title,
                    order: module_idx,
                    lessons,
                });
            }
        }
    }

    Ok(modules)
}

async fn fetch_module_lessons(
    session: &RocketseatSession,
    journey_slug: &str,
    module_slug: &str,
) -> anyhow::Result<Vec<RocketseatLesson>> {
    let url = format!("{}/jornada/{}/sala/{}", BASE_URL, journey_slug, module_slug);

    let resp = session
        .client
        .get(&url)
        .query(&[("_rsc", "1")])
        .header("RSC", "1")
        .header("Accept", "*/*")
        .header("Host", "app.rocketseat.com.br")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let text = resp.text().await?;
    let data = parse_rsc_response(&text);

    let lesson_groups = data
        .get("lessonGroups")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lessons = Vec::new();
    let mut order = 0i64;

    for group in &lesson_groups {
        let group_lessons = group
            .get("lessons")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for lesson in &group_lessons {
            order += 1;

            let id = lesson
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = lesson
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let slug = lesson
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let lesson_type = lesson
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("VIDEO")
                .to_string();

            let video_id = lesson
                .get("video")
                .and_then(|v| v.get("jupiterVideoId"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let duration = lesson
                .get("duration")
                .and_then(|v| v.as_i64());

            lessons.push(RocketseatLesson {
                id,
                name: title,
                order,
                slug,
                lesson_type,
                video_id,
                duration,
            });
        }
    }

    Ok(lessons)
}

pub fn get_video_embed_url(video_id: &str) -> String {
    format!(
        "https://iframe.mediadelivery.net/embed/{}/{}",
        BUNNY_LIBRARY_ID, video_id
    )
}

pub async fn save_session(session: &RocketseatSession) -> anyhow::Result<()> {
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
    tracing::info!("[rocketseat] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<RocketseatSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[rocketseat] session loaded");

    Ok(Some(RocketseatSession {
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
