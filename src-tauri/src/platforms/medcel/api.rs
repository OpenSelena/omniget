use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const API_BASE: &str = "https://service.medcel.com.br";
const ORIGIN_URL: &str = "https://areaaluno.medcel.com.br";

#[derive(Clone)]
pub struct MedcelSession {
    pub token: String,
    pub api_key: String,
    pub student_id: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub token: String,
    pub api_key: String,
    pub student_id: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedcelCourse {
    pub id: String,
    pub name: String,
    pub product_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedcelModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<MedcelLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedcelLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub type_id: String,
    pub product_id: String,
    pub materials: Vec<MedcelMaterial>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedcelMaterial {
    pub key: String,
    pub title: String,
    pub url: String,
}

fn build_client(token: &str, api_key: &str, origin: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        "X-Api-Key",
        HeaderValue::from_str(api_key)?,
    );
    headers.insert(
        "X-Amz-User-Agent",
        HeaderValue::from_static(USER_AGENT),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", origin))?,
    );
    headers.insert(
        "Origin",
        HeaderValue::from_str(origin)?,
    );
    headers.insert(
        "Accept",
        HeaderValue::from_static("application/json, text/plain, */*"),
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

pub fn build_client_with_origin(token: &str, api_key: &str, origin: &str) -> anyhow::Result<reqwest::Client> {
    build_client(token, api_key, origin)
}

fn session_file_path() -> anyhow::Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow!("Could not find app data directory"))?;
    Ok(data_dir.join("omniget").join("medcel_session.json"))
}

pub async fn authenticate(email: &str, password: &str, api_key: &str) -> anyhow::Result<MedcelSession> {
    let temp_client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Api-Key",
        HeaderValue::from_str(api_key)?,
    );
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", ORIGIN_URL))?,
    );
    headers.insert(
        "Origin",
        HeaderValue::from_str(ORIGIN_URL)?,
    );

    let payload = serde_json::json!({
        "email": email,
        "password": password,
    });

    let resp = temp_client
        .post(format!("{}/m1/students/auth", API_BASE))
        .headers(headers)
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

    let student_id = body
        .get("_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let session_token = body
        .get("sessionToken")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("sessionToken not found in auth response"))?
        .to_string();

    let client = build_client(&session_token, api_key, ORIGIN_URL)?;

    Ok(MedcelSession {
        token: session_token,
        api_key: api_key.to_string(),
        student_id,
        client,
    })
}

pub async fn validate_token(session: &MedcelSession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get(format!("{}/m1/contracts/getContractsByStudent?_id=me", API_BASE))
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn fetch_student_id(session: &MedcelSession) -> anyhow::Result<String> {
    let resp = session
        .client
        .get(format!("{}/m1/contracts/getContractsByStudent?_id=me", API_BASE))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!("fetch_student_id failed (status {})", status));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let id = body
        .get("_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(id)
}

pub async fn list_courses(session: &MedcelSession) -> anyhow::Result<Vec<MedcelCourse>> {
    let url = format!(
        "{}/m1/contracts/getContractsByStudent?_id={}",
        API_BASE, session.student_id
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

    let contracts = body
        .get("contracts")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();

    for contract in &contracts {
        let product = match contract.get("product") {
            Some(p) => p,
            None => continue,
        };

        let product_id = match product.get("_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let product_name = product
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Curso")
            .to_string();

        let content_available = contract
            .get("overallStatus")
            .and_then(|s| s.get("content"))
            .and_then(|c| c.get("available"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if !content_available {
            continue;
        }

        courses.push(MedcelCourse {
            id: product_id.clone(),
            name: product_name,
            product_id,
        });
    }

    Ok(courses)
}

pub async fn get_modules(session: &MedcelSession, product_id: &str) -> anyhow::Result<Vec<MedcelModule>> {
    let mut all_playlists = Vec::new();
    let mut page = 1u32;
    let limit = 100u32;

    loop {
        let url = format!(
            "{}/m9/subjectPlaylists/getSubjectPlaylists?studentId={}&productId={}&specialtyId=&sortMode=incidence&trial=false&page={}&limit={}",
            API_BASE, session.student_id, product_id, page, limit
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

        let items = body
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if items.is_empty() {
            break;
        }

        all_playlists.extend(items);

        let total_pages = body
            .get("pagination")
            .and_then(|p| p.get("pageTotal"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        if page as u64 >= total_pages {
            break;
        }
        page += 1;
    }

    let mut modules = Vec::new();

    for (i, playlist) in all_playlists.iter().enumerate() {
        let playlist_id = match playlist.get("_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let name = playlist
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        let order = playlist
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(i as i64 + 1);

        let lessons = get_playlist_lessons(session, &playlist_id, product_id).await?;

        modules.push(MedcelModule {
            id: playlist_id,
            name,
            order,
            lessons,
        });
    }

    modules.sort_by_key(|m| m.order);

    Ok(modules)
}

async fn get_playlist_lessons(
    session: &MedcelSession,
    playlist_id: &str,
    product_id: &str,
) -> anyhow::Result<Vec<MedcelLesson>> {
    let url = format!(
        "{}/m9/subjectPlaylists/getPlaylistContents?playlistId={}&studentId={}",
        API_BASE, playlist_id, session.student_id
    );

    let resp = session.client.get(&url).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_playlist_lessons returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let playlist_contents = body
        .get("playlistContents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lessons = Vec::new();

    for (ci, content) in playlist_contents.iter().enumerate() {
        let content_name = content
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        let classes = content
            .get("classes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if classes.is_empty() {
            continue;
        }

        let mut video_class: Option<&serde_json::Value> = None;
        let mut all_materials = Vec::new();

        for class_item in &classes {
            let type_id = class_item
                .get("subTypeLearningObject")
                .and_then(|s| s.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(mats) = class_item.get("complementMaterial").and_then(|v| v.as_array()) {
                for mat in mats {
                    let mat_url = mat.get("location").and_then(|v| v.as_str()).unwrap_or("");
                    let mat_title = mat.get("title").and_then(|v| v.as_str()).unwrap_or("Material");
                    let mat_key = mat.get("key").and_then(|v| v.as_str()).unwrap_or("");

                    if !mat_url.is_empty() {
                        all_materials.push(MedcelMaterial {
                            key: mat_key.to_string(),
                            title: mat_title.to_string(),
                            url: mat_url.to_string(),
                        });
                    }
                }
            }

            if type_id == "videoClass" && video_class.is_none() {
                video_class = Some(class_item);
            }
        }

        let chosen = video_class.unwrap_or(&classes[0]);

        let class_id = chosen
            .get("id")
            .or_else(|| chosen.get("_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if class_id.is_empty() {
            continue;
        }

        let type_id = chosen
            .get("subTypeLearningObject")
            .and_then(|s| s.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        lessons.push(MedcelLesson {
            id: class_id,
            name: content_name,
            order: (ci + 1) as i64,
            type_id,
            product_id: product_id.to_string(),
            materials: all_materials,
        });
    }

    Ok(lessons)
}

pub async fn get_video_url(
    session: &MedcelSession,
    class_id: &str,
    product_id: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "{}/m2/videos/getVideoToPlay?id={}&student={}&product={}",
        API_BASE, class_id, session.student_id, product_id
    );

    let resp = session
        .client
        .get(&url)
        .header("X-Host-Origin", ORIGIN_URL)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_video_url returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let data = body.get("data").unwrap_or(&body);

    let uri = data
        .get("uri")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let signature = data
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let video_url = format!("{}{}", uri, signature);

    if video_url.is_empty() {
        return Err(anyhow!("No video URL found in response"));
    }

    Ok(video_url)
}

pub async fn save_session(session: &MedcelSession) -> anyhow::Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let saved = SavedSession {
        token: session.token.clone(),
        api_key: session.api_key.clone(),
        student_id: session.student_id.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, json).await?;
    tracing::info!("[medcel] session saved");
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<MedcelSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token, &saved.api_key, ORIGIN_URL)?;

    tracing::info!("[medcel] session loaded");

    Ok(Some(MedcelSession {
        token: saved.token,
        api_key: saved.api_key,
        student_id: saved.student_id,
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
