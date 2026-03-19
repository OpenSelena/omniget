use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://api.estrategia.com";

#[derive(Clone)]
pub struct EstrategiaLdiSession {
    pub token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiCourse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<EstrategiaLdiLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaLdiTrack {
    pub url: String,
    pub audio_url: Option<String>,
    pub title: Option<String>,
    pub duration: Option<f64>,
}

fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!("__Secure-SID={}", token))?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://concursos.estrategia.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://concursos.estrategia.com/"),
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
    Ok(data_dir.join("omniget").join("estrategia_ldi_session.json"))
}

pub async fn validate_token(session: &EstrategiaLdiSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/bff/goals/shelves?page=1&per_page=1", API_BASE))
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn search_courses(session: &EstrategiaLdiSession, query: &str) -> anyhow::Result<Vec<EstrategiaLdiCourse>> {
    let mut courses = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "{}/bff/goals/shelves?page={}&per_page=20&name={}",
            API_BASE,
            page,
            urlencoding::encode(query)
        );

        let resp = session.client.get(&url).send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!(
                "search_courses returned status {}: {}",
                status,
                &body_text[..body_text.len().min(300)]
            ));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;

        let goals = body
            .get("goals")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if goals.is_empty() {
            break;
        }

        for goal in &goals {
            let id = goal
                .get("id")
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            let name = goal
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !id.is_empty() {
                courses.push(EstrategiaLdiCourse { id, name });
            }
        }

        page += 1;
        if page > 50 {
            break;
        }
    }

    Ok(courses)
}

pub async fn list_courses(session: &EstrategiaLdiSession) -> anyhow::Result<Vec<EstrategiaLdiCourse>> {
    search_courses(session, "").await
}

pub async fn get_course_content(
    session: &EstrategiaLdiSession,
    goal_id: &str,
) -> anyhow::Result<Vec<EstrategiaLdiModule>> {
    let url = format!("{}/bff/goals/{}/contents/ldi", API_BASE, goal_id);

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
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            body.as_array().cloned().unwrap_or_default()
        });

    let mut modules = Vec::new();

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_name = chapter
            .get("title")
            .or_else(|| chapter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Chapter")
            .to_string();

        let chapter_id = chapter
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => format!("{}", ci),
            })
            .unwrap_or_else(|| format!("{}", ci));

        let items = chapter
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (ii, item) in items.iter().enumerate() {
            let item_id = item
                .get("id")
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => format!("{}-{}", ci, ii),
                })
                .unwrap_or_else(|| format!("{}-{}", ci, ii));

            let item_name = item
                .get("title")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            lessons.push(EstrategiaLdiLesson {
                id: item_id,
                name: item_name,
                order: ii as i64,
            });
        }

        modules.push(EstrategiaLdiModule {
            id: chapter_id,
            name: chapter_name,
            order: ci as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_item_detail(
    session: &EstrategiaLdiSession,
    item_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut all_sub_blocks = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!("{}/v3/mci/items/{}", API_BASE, item_id);

        let resp = session.client.get(&url)
            .query(&[
                ("page", page.to_string().as_str()),
                ("order", "asc"),
                ("per_page", "30"),
                ("view_mode", "complete"),
                ("should_load_metadata", "true"),
                ("video_only", "false"),
                ("text_only", "false"),
                ("question_only", "false"),
                ("cast_only", "false"),
                ("attachment_only", "false"),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            if all_sub_blocks.is_empty() {
                return Err(anyhow!(
                    "get_item_detail returned status {}: {}",
                    status,
                    &body_text[..body_text.len().min(300)]
                ));
            }
            break;
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;
        let item_data = body.get("data").unwrap_or(&body);

        if let Some(blocks) = item_data.get("sub_blocks").and_then(|v| v.as_array()) {
            all_sub_blocks.extend(blocks.clone());
        }

        let total_pages = body
            .get("meta")
            .and_then(|m| m.get("last_page"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        if page >= total_pages {
            break;
        }
        page += 1;
    }

    let result = serde_json::json!({
        "sub_blocks": all_sub_blocks
    });
    Ok(result)
}

pub async fn get_track_info(
    session: &EstrategiaLdiSession,
    track_id: &str,
) -> anyhow::Result<EstrategiaLdiTrack> {
    let url = format!("{}/v2/tracks/{}", API_BASE, track_id);

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_track_info returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    let track_data = body.get("data").unwrap_or(&body);

    let mut video_files: Vec<(i64, String)> = Vec::new();
    if let Some(files) = track_data.get("video_files").and_then(|v| v.as_array()) {
        for vf in files {
            let height = vf.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
            let link = vf.get("link").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !link.is_empty() {
                video_files.push((height, link));
            }
        }
    }

    video_files.sort_by(|a, b| b.0.cmp(&a.0));

    let track_url = if let Some((_, best_url)) = video_files.first() {
        best_url.clone()
    } else {
        track_data
            .get("url")
            .or_else(|| track_data.get("source"))
            .or_else(|| track_data.get("link"))
            .or_else(|| body.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let duration = track_data
        .get("duration")
        .and_then(|v| v.as_f64());

    let audio_url = track_data
        .get("audio_url")
        .or_else(|| track_data.get("audio"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let title = track_data
        .get("name")
        .or_else(|| track_data.get("title"))
        .and_then(|v| v.as_str())
        .map(String::from);

    if track_url.is_empty() {
        return Err(anyhow!("No URL found in track response for track_id={}", track_id));
    }

    Ok(EstrategiaLdiTrack {
        url: track_url,
        audio_url,
        title,
        duration,
    })
}

pub fn extract_track_ids(item_detail: &serde_json::Value) -> Vec<String> {
    let mut track_ids = Vec::new();

    let sub_blocks = item_detail
        .get("sub_blocks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for block in &sub_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if block_type == "cast" {
            let data = block.get("data")
                .or_else(|| block.get("simple_data"))
                .cloned()
                .unwrap_or_default();

            let track_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let track_value = data.get("value").and_then(|v| v.as_str()).unwrap_or("");

            if track_type == "track" && !track_value.is_empty() {
                track_ids.push(track_value.to_string());
            }
        } else if block_type == "videoMyDocuments" {
            let data = block.get("data")
                .or_else(|| block.get("simple_data"))
                .cloned()
                .unwrap_or_default();

            let resolved = data.get("resolved").cloned().unwrap_or_default();
            let video_url = resolved.get("data")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !video_url.is_empty() {
                track_ids.push(format!("direct:{}", video_url));
            }
        }
    }

    if track_ids.is_empty() {
        let tracks = item_detail
            .get("tracks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for track in &tracks {
            if let Some(id) = track.get("id").map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            }) {
                if !id.is_empty() {
                    track_ids.push(id);
                }
            }
        }
    }

    if track_ids.is_empty() {
        if let Some(track_id) = item_detail.get("track_id").map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }) {
            if !track_id.is_empty() {
                track_ids.push(track_id);
            }
        }
    }

    track_ids
}

pub fn extract_attachment_urls(item_detail: &serde_json::Value) -> Vec<(String, String)> {
    let mut attachments = Vec::new();

    let sub_blocks = item_detail
        .get("sub_blocks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for block in &sub_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if block_type == "attachment" || block_type == "pdfMyDocuments" {
            let data = block.get("data")
                .or_else(|| block.get("simple_data"))
                .cloned()
                .unwrap_or_default();

            let resolved = data.get("resolved").cloned().unwrap_or(data.clone());

            let url = resolved.get("url")
                .or_else(|| resolved.get("data"))
                .or_else(|| resolved.get("file"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let name = resolved.get("name")
                .or_else(|| resolved.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("attachment")
                .to_string();

            if !url.is_empty() {
                attachments.push((name, url));
            }
        }
    }

    let atts = item_detail
        .get("attachments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for att in &atts {
        let url = att
            .get("url")
            .or_else(|| att.get("file"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = att
            .get("name")
            .or_else(|| att.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("attachment")
            .to_string();

        if !url.is_empty() {
            attachments.push((name, url));
        }
    }

    attachments
}

pub fn extract_description(item_detail: &serde_json::Value) -> String {
    let sub_blocks = item_detail.get("sub_blocks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut parts = Vec::new();

    for block in &sub_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if block_type == "tiptap" {
            let data = block.get("data").or_else(|| block.get("simple_data")).cloned().unwrap_or_default();
            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                parts.push(content.to_string());
            } else if let Some(html) = data.as_str() {
                parts.push(html.to_string());
            }
        } else if block_type == "question" {
            let data = block.get("data").or_else(|| block.get("simple_data")).cloned().unwrap_or_default();
            let resolved = data.get("resolved").cloned().unwrap_or(data.clone());

            let statement = resolved.get("statement")
                .or_else(|| resolved.get("enunciado"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !statement.is_empty() {
                let mut q = format!("<div class=\"question\"><p><strong>Questao:</strong> {}</p>", statement);

                if let Some(alts) = resolved.get("alternatives").and_then(|v| v.as_array()) {
                    q.push_str("<ol type=\"A\">");
                    for alt in alts {
                        let text = alt.get("text")
                            .or_else(|| alt.get("content"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let is_correct = alt.get("is_correct")
                            .or_else(|| alt.get("correct"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if is_correct {
                            q.push_str(&format!("<li><strong>{}</strong></li>", text));
                        } else {
                            q.push_str(&format!("<li>{}</li>", text));
                        }
                    }
                    q.push_str("</ol>");
                }

                let answer = resolved.get("answer")
                    .or_else(|| resolved.get("resposta"))
                    .or_else(|| resolved.get("gabarito"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !answer.is_empty() {
                    q.push_str(&format!("<p><strong>Resposta:</strong> {}</p>", answer));
                }

                let explanation = resolved.get("explanation")
                    .or_else(|| resolved.get("explicacao"))
                    .or_else(|| resolved.get("comment"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !explanation.is_empty() {
                    q.push_str(&format!("<p><strong>Explicacao:</strong> {}</p>", explanation));
                }

                q.push_str("</div>");
                parts.push(q);
            }
        }
    }

    parts.join("\n\n")
}

pub fn extract_audio_urls(item_detail: &serde_json::Value) -> Vec<(String, String)> {
    let sub_blocks = item_detail.get("sub_blocks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut audios = Vec::new();

    for block in &sub_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if block_type == "cast" {
            let data = block.get("data").or_else(|| block.get("simple_data")).cloned().unwrap_or_default();
            let track_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let track_value = data.get("value").and_then(|v| v.as_str()).unwrap_or("");

            if track_type == "track" && !track_value.is_empty() {
                audios.push(("track".to_string(), track_value.to_string()));
            }
        }
    }

    audios
}

pub async fn save_session(session: &EstrategiaLdiSession) -> anyhow::Result<()> {
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
    tracing::info!("[estrategia_ldi] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<EstrategiaLdiSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[estrategia_ldi] session loaded");

    Ok(Some(EstrategiaLdiSession {
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
