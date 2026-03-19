use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://api.estrategia.com";

#[derive(Clone)]
pub struct EstrategiaMilitaresSession {
    pub token: String,
    pub cookie_string: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub cookie_string: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub goal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<EstrategiaMilitaresLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstrategiaMilitaresTrack {
    pub url: String,
    pub audio_url: Option<String>,
    pub title: Option<String>,
    pub duration: Option<f64>,
}

pub fn parse_token_input(input: &str) -> (String, String) {
    let trimmed = input.trim();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let cookies_arr = val.get("cookies")
                .and_then(|v| v.as_array())
                .cloned()
                .or_else(|| val.as_array().cloned())
                .unwrap_or_default();

            let mut jwt = String::new();
            let mut cookie_parts: Vec<String> = Vec::new();

            for c in &cookies_arr {
                let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if !name.is_empty() && !value.is_empty() {
                    cookie_parts.push(format!("{}={}", name, value));
                    if name == "__Secure-SID" {
                        jwt = value.to_string();
                    }
                }
            }

            if !jwt.is_empty() {
                return (jwt, cookie_parts.join("; "));
            }
        }
    }

    if trimmed.contains("; ") || trimmed.contains("__Secure-SID=") || trimmed.contains("_cfuvid=") {
        let mut jwt = String::new();
        for part in trimmed.split("; ") {
            if let Some(val) = part.strip_prefix("__Secure-SID=") {
                jwt = val.to_string();
            }
        }
        if !jwt.is_empty() {
            return (jwt, trimmed.to_string());
        }
        return (trimmed.to_string(), format!("__Secure-SID={}", trimmed));
    }

    (trimmed.to_string(), format!("__Secure-SID={}", trimmed))
}

fn build_client(token: &str, cookie_string: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "Cookie",
        HeaderValue::from_str(cookie_string)?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert("Accept-Language", HeaderValue::from_static("pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://militares.estrategia.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://militares.estrategia.com/"),
    );
    headers.insert("x-vertical", HeaderValue::from_static("militares"));
    headers.insert("x-requester-id", HeaderValue::from_static("front-student"));

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
    Ok(data_dir.join("omniget").join("estrategia_militares_session.json"))
}

pub async fn validate_token(session: &EstrategiaMilitaresSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/bff/goals/shelves?page=1&per_page=1&name=test", API_BASE))
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    tracing::info!("[estrategia_militares] validate_token status={} body_preview={}", status, &body[..body.len().min(500)]);

    Ok(status.is_success())
}

async fn fetch_ldi_courses_for_goal(session: &EstrategiaMilitaresSession, goal_id: &str) -> Vec<EstrategiaMilitaresCourse> {
    let url = format!("{}/bff/goals/{}/contents/ldi?page=1&per_page=50", API_BASE, goal_id);

    let resp = match session.client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[estrategia_militares] failed to fetch LDI for goal {}: {}", goal_id, e);
            return Vec::new();
        }
    };

    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let body: serde_json::Value = match serde_json::from_str(&body_text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let contents = body
        .get("data")
        .and_then(|d| d.get("contents"))
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();
    for c in &contents {
        let id = c.get("id").map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }).unwrap_or_default();

        let name = c.get("title")
            .or(c.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = c.get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() && !slug.is_empty() {
            courses.push(EstrategiaMilitaresCourse {
                id,
                name,
                slug,
                goal_id: goal_id.to_string(),
            });
        }
    }

    courses
}

pub async fn search_courses(session: &EstrategiaMilitaresSession, query: &str) -> anyhow::Result<Vec<EstrategiaMilitaresCourse>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let url = format!(
        "{}/bff/goals/shelves?page=1&per_page=20&name={}",
        API_BASE,
        urlencoding::encode(query.trim())
    );

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    tracing::info!("[estrategia_militares] search status={} body_len={}", status, body_text.len());

    if !status.is_success() {
        return Err(anyhow!(
            "search_courses returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    let mut all_goal_ids = Vec::new();

    let result_data = body.get("data").unwrap_or(&body);

    if let Some(shelves) = result_data.get("shelves").and_then(|v| v.as_object()) {
        for (_shelf_name, goals_val) in shelves {
            let goals = goals_val.as_array().cloned().unwrap_or_default();
            for goal in &goals {
                let goal_id = goal.get("id").map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::new(),
                }).unwrap_or_default();

                if !goal_id.is_empty() {
                    all_goal_ids.push(goal_id);
                }
            }
        }
    }

    if let Some(highlights) = result_data.get("highlights") {
        let highlight_goals = highlights.get("goals").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for goal in &highlight_goals {
            let goal_id = goal.get("id").map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            }).unwrap_or_default();

            if !goal_id.is_empty() && !all_goal_ids.contains(&goal_id) {
                all_goal_ids.push(goal_id);
            }
        }
    }

    let mut courses = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for goal_id in &all_goal_ids {
        let ldi_courses = fetch_ldi_courses_for_goal(session, goal_id).await;
        for c in ldi_courses {
            if seen_ids.insert(c.id.clone()) {
                courses.push(c);
            }
        }
    }

    tracing::info!("[estrategia_militares] found {} courses across {} goals for query '{}'", courses.len(), all_goal_ids.len(), query);
    Ok(courses)
}

pub async fn list_courses(session: &EstrategiaMilitaresSession) -> anyhow::Result<Vec<EstrategiaMilitaresCourse>> {
    search_courses(session, "militar").await
}

pub async fn get_course_content(
    session: &EstrategiaMilitaresSession,
    course_slug: &str,
) -> anyhow::Result<Vec<EstrategiaMilitaresModule>> {
    let url = format!("{}/v3/mci/courses/slug/{}", API_BASE, course_slug);

    let resp = session.client.get(&url)
        .header("cache-control", "no-cache")
        .send()
        .await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    tracing::info!("[estrategia_militares] get_course_content slug={} status={} len={}", course_slug, status, body_text.len());

    if !status.is_success() {
        return Err(anyhow!(
            "get_course_content returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let course_data = body.get("data").unwrap_or(&body);

    let chapters = course_data
        .get("chapters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut modules = Vec::new();

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_name = chapter
            .get("title")
            .or_else(|| chapter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Chapter")
            .to_string();

        let chapter_id = chapter
            .get("chapter_id")
            .or(chapter.get("id"))
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
                .get("item_id")
                .or(item.get("id"))
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

            lessons.push(EstrategiaMilitaresLesson {
                id: item_id,
                name: item_name,
                order: ii as i64,
            });
        }

        modules.push(EstrategiaMilitaresModule {
            id: chapter_id,
            name: chapter_name,
            order: ci as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub async fn get_item_detail(
    session: &EstrategiaMilitaresSession,
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
    session: &EstrategiaMilitaresSession,
    track_id: &str,
) -> anyhow::Result<EstrategiaMilitaresTrack> {
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

    tracing::info!("[estrategia_militares] track {} url_len={} has_video_files={} has_audio={}", track_id, track_url.len(), !video_files.is_empty(), audio_url.is_some());

    if track_url.is_empty() {
        return Err(anyhow!("No URL found in track response for track_id={}", track_id));
    }

    Ok(EstrategiaMilitaresTrack {
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
                            q.push_str(&format!("<li><strong>{}</strong> ✓</li>", text));
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

pub async fn save_session(session: &EstrategiaMilitaresSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        cookie_string: session.cookie_string.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[estrategia_militares] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<EstrategiaMilitaresSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.cookie_string)?;

    tracing::info!("[estrategia_militares] session loaded");

    Ok(Some(EstrategiaMilitaresSession {
        token: saved.token,
        cookie_string: saved.cookie_string,
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
