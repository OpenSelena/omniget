use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct CurseducaSession {
    pub token: String,
    pub api_key: String,
    pub site_url: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub api_key: String,
    pub site_url: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseducaCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseducaModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<CurseducaLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseducaLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub uuid: Option<String>,
    pub video_type: Option<i64>,
}

fn build_client(token: &str, api_key: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "api_key",
        HeaderValue::from_str(api_key)?,
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));

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
    Ok(data_dir.join("omniget").join("curseduca_session.json"))
}

pub async fn discover_api_key(site_url: &str) -> anyhow::Result<String> {
    let temp_client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let clean_url = site_url.trim_end_matches('/');

    let resp = temp_client
        .get("https://application.curseduca.pro/platform-by-url")
        .query(&[("url", clean_url)])
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "discover_api_key returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let key = body
        .get("key")
        .or_else(|| body.get("apiKey"))
        .or_else(|| body.get("api_key"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No API key in platform discovery response"))?
        .to_string();

    Ok(key)
}

pub async fn authenticate(site_url: &str, username: &str, password: &str) -> anyhow::Result<CurseducaSession> {
    let api_key = discover_api_key(site_url).await?;

    let temp_client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let payload = serde_json::json!({
        "username": username,
        "password": password,
    });

    let resp = temp_client
        .post("https://prof.curseduca.pro/login")
        .header("api_key", &api_key)
        .header("Content-Type", "application/json")
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
        .get("accessToken")
        .or_else(|| body.get("token"))
        .or_else(|| body.get("access_token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No access token in login response"))?
        .to_string();

    let client = build_client(&token, &api_key)?;

    Ok(CurseducaSession {
        token,
        api_key,
        site_url: site_url.to_string(),
        client,
    })
}

pub async fn validate_token(session: &CurseducaSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://prof.curseduca.pro/me/access")
        .send()
        .await?;
    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &CurseducaSession) -> anyhow::Result<Vec<CurseducaCourse>> {
    let resp = session
        .client
        .get("https://prof.curseduca.pro/me/access")
        .send()
        .await?;

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

    let courses_arr = if body.is_array() {
        body.as_array().cloned().unwrap_or_default()
    } else {
        body.get("courses")
            .or_else(|| body.get("data"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let mut courses = Vec::new();

    for item in &courses_arr {
        let id = item
            .get("id")
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let name = item
            .get("name")
            .or_else(|| item.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let slug = item
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        if !id.is_empty() {
            courses.push(CurseducaCourse { id, name, slug });
        }
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &CurseducaSession,
    course_slug: &str,
) -> anyhow::Result<Vec<CurseducaModule>> {
    let url = format!("{}/m/lessons/{}", session.site_url.trim_end_matches('/'), course_slug);
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
    let mut module_order = 0i64;

    let mut search_pos = 0;
    while let Some(script_start) = body_text[search_pos..].find("<script") {
        let abs_start = search_pos + script_start;
        if let Some(script_end) = body_text[abs_start..].find("</script>") {
            let script_content = &body_text[abs_start..abs_start + script_end];
            search_pos = abs_start + script_end + 9;

            if script_content.contains("modules") || script_content.contains("lessons") {
                if let Some(json_start) = script_content.find('{') {
                    let json_str = &script_content[json_start..];
                    if let Some(end) = find_matching_brace(json_str) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str[..=end]) {
                            if let Some(mods) = extract_modules_from_data(&data) {
                                for m in mods {
                                    modules.push(m);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    if modules.is_empty() {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body_text) {
            if let Some(mods) = extract_modules_from_data(&data) {
                modules = mods;
            }
        }
    }

    for m in &mut modules {
        module_order += 1;
        m.order = module_order;
    }

    Ok(modules)
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_modules_from_data(data: &serde_json::Value) -> Option<Vec<CurseducaModule>> {
    let modules_arr = data.get("modules")
        .or_else(|| data.get("content"))
        .and_then(|v| v.as_array())?;

    let mut modules = Vec::new();

    for (mi, mod_val) in modules_arr.iter().enumerate() {
        let mod_id = mod_val.get("id")
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => format!("{}", mi),
            })
            .unwrap_or_else(|| format!("{}", mi));

        let mod_name = mod_val.get("name")
            .or_else(|| mod_val.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let lessons_arr = mod_val.get("lessons")
            .or_else(|| mod_val.get("aulas"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut lessons = Vec::new();

        for (li, lesson_val) in lessons_arr.iter().enumerate() {
            let lesson_id = lesson_val.get("id")
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => format!("{}", li),
                })
                .unwrap_or_else(|| format!("{}", li));

            let lesson_name = lesson_val.get("name")
                .or_else(|| lesson_val.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let uuid = lesson_val.get("uuid")
                .or_else(|| lesson_val.get("externalId"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let video_type = lesson_val.get("type")
                .or_else(|| lesson_val.get("videoType"))
                .and_then(|v| v.as_i64());

            lessons.push(CurseducaLesson {
                id: lesson_id,
                name: lesson_name,
                order: li as i64,
                uuid,
                video_type,
            });
        }

        modules.push(CurseducaModule {
            id: mod_id,
            name: mod_name,
            order: mi as i64,
            lessons,
        });
    }

    Some(modules)
}

pub async fn get_lesson_video_url(
    session: &CurseducaSession,
    lesson_uuid: &str,
) -> anyhow::Result<Option<String>> {
    let url = format!("https://clas.curseduca.pro/bff/aulas/{}/watch", lesson_uuid);
    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_lesson_video_url returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let video_url = body
        .get("url")
        .or_else(|| body.get("videoUrl"))
        .or_else(|| body.get("video_url"))
        .or_else(|| body.get("playbackUrl"))
        .and_then(|v| v.as_str())
        .map(String::from);

    if video_url.is_some() {
        return Ok(video_url);
    }

    let video_type = body.get("type").and_then(|v| v.as_i64()).unwrap_or(0);

    match video_type {
        4 => {
            if let Some(vid_id) = body.get("externalId").or_else(|| body.get("videoId")).and_then(|v| v.as_str()) {
                return Ok(Some(format!("https://www.youtube.com/watch?v={}", vid_id)));
            }
        }
        7 => {
            if let Some(vid_id) = body.get("externalId").or_else(|| body.get("videoId")).and_then(|v| v.as_str()) {
                return Ok(Some(format!("https://player.vimeo.com/video/{}", vid_id)));
            }
        }
        20 => {
            if let Some(hls) = body.get("hlsUrl").or_else(|| body.get("streamUrl")).and_then(|v| v.as_str()) {
                return Ok(Some(hls.to_string()));
            }
        }
        22 => {
            if let Some(url) = body.get("scaleupUrl").or_else(|| body.get("playerUrl")).and_then(|v| v.as_str()) {
                return Ok(Some(url.to_string()));
            }
        }
        _ => {}
    }

    Ok(None)
}

pub async fn save_session(session: &CurseducaSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        api_key: session.api_key.clone(),
        site_url: session.site_url.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[curseduca] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<CurseducaSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.api_key)?;

    tracing::info!("[curseduca] session loaded");

    Ok(Some(CurseducaSession {
        token: saved.token,
        api_key: saved.api_key,
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
