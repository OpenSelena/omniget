use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct ThinkificSession {
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
pub struct ThinkificCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkificModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<ThinkificLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkificLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub contentable_type: String,
    pub contentable_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkificLessonDetail {
    pub id: String,
    pub name: String,
    pub video_url: Option<String>,
    pub html_text: Option<String>,
    pub files: Vec<ThinkificFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkificFile {
    pub name: String,
    pub url: String,
}

fn normalise_site_url(raw: &str) -> String {
    let mut url = raw.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        url = format!("https://{}", url);
    }
    if let Ok(parsed) = reqwest::Url::parse(&url) {
        format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""))
    } else {
        url.trim_end_matches('/').to_string()
    }
}

fn build_client(cookies: &str, site_url: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", HeaderValue::from_str(cookies)?);
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", site_url))?,
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
    Ok(data_dir.join("omniget").join("thinkific_session.json"))
}

pub fn create_session(cookies: &str, site_url: &str) -> anyhow::Result<ThinkificSession> {
    let site_url = normalise_site_url(site_url);
    let client = build_client(cookies, &site_url)?;
    Ok(ThinkificSession {
        cookies: cookies.to_string(),
        site_url,
        client,
    })
}

pub async fn validate_session(session: &ThinkificSession) -> anyhow::Result<bool> {
    let url = format!(
        "{}/api/course_player/v2/enrollments",
        session.site_url
    );
    let resp = session.client.get(&url).send().await?;
    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &ThinkificSession) -> anyhow::Result<Vec<ThinkificCourse>> {
    let mut courses = Vec::new();

    for endpoint in &[
        "/api/course_player/v2/enrollments",
        "/api/course_player/v2/courses",
        "/api/v0/enrollments",
        "/api/v0/courses",
    ] {
        let url = format!("{}{}", session.site_url, endpoint);
        let resp = match session.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let body_text = resp.text().await.unwrap_or_default();
        let parsed = parse_enrollments(&body_text, session).await;
        if !parsed.is_empty() {
            courses = parsed;
            break;
        }
    }

    if courses.is_empty() {
        courses = scrape_enrollments(session).await.unwrap_or_default();
    }

    Ok(courses)
}

async fn parse_enrollments(
    body: &str,
    session: &ThinkificSession,
) -> Vec<ThinkificCourse> {
    let raw: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let items = match &raw {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(map) => {
            for key in &["items", "enrollments", "courses"] {
                if let Some(serde_json::Value::Array(arr)) = map.get(*key) {
                    return parse_items_array(arr, session).await;
                }
            }
            return Vec::new();
        }
        _ => return Vec::new(),
    };

    parse_items_array(&items, session).await
}

async fn parse_items_array(
    items: &[serde_json::Value],
    session: &ThinkificSession,
) -> Vec<ThinkificCourse> {
    let mut courses = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in items {
        let slug = item
            .get("slug")
            .or_else(|| item.get("course_slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if slug.is_empty() || seen.contains(&slug) {
            continue;
        }
        seen.insert(slug.clone());

        let mut name = item
            .get("name")
            .or_else(|| item.get("course_name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&slug)
            .to_string();

        let mut image_url = extract_image(item);

        if name == slug || image_url.is_none() {
            if let Ok(detail) = fetch_course_detail(session, &slug).await {
                if let Some(co) = detail.get("course").and_then(|v| v.as_object()) {
                    let co_val = serde_json::Value::Object(co.clone());
                    if let Some(n) = co_val.get("name").and_then(|v| v.as_str()) {
                        if !n.is_empty() {
                            name = n.to_string();
                        }
                    }
                    if image_url.is_none() {
                        image_url = extract_image(&co_val);
                    }
                } else {
                    if let Some(n) = detail.get("name").and_then(|v| v.as_str()) {
                        if !n.is_empty() && name == slug {
                            name = n.to_string();
                        }
                    }
                    if image_url.is_none() {
                        image_url = extract_image(&detail);
                    }
                }
            }
        }

        courses.push(ThinkificCourse {
            id: slug.clone(),
            name,
            slug,
            image_url,
        });
    }

    courses
}

fn extract_image(obj: &serde_json::Value) -> Option<String> {
    for field in &[
        "logo",
        "course_card_image_url",
        "card_image_url",
        "banner_image_url",
        "image_url",
        "thumbnail_url",
        "cover_image_url",
        "image",
    ] {
        if let Some(v) = obj.get(*field).and_then(|v| v.as_str()) {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

async fn fetch_course_detail(
    session: &ThinkificSession,
    slug: &str,
) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "{}/api/course_player/v2/courses/{}",
        session.site_url, slug
    );
    let resp = session.client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await?;
    Ok(body)
}

async fn scrape_enrollments(
    session: &ThinkificSession,
) -> anyhow::Result<Vec<ThinkificCourse>> {
    let url = format!("{}/enrollments", session.site_url);
    let resp = session.client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let html = resp.text().await?;
    let slug_re = Regex::new(r#"href=["']/?courses/(?:take/|enrolled/|enroll/)?([a-zA-Z][a-zA-Z0-9_-]{2,})["']"#)?;

    let skip_set: std::collections::HashSet<&str> = [
        "take", "new", "edit", "enroll", "preview", "sign_in", "sign_up",
        "enrolled", "progress", "admin", "users", "bundle", "checkout",
        "cart", "payment",
    ]
    .into();

    let mut courses = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in slug_re.captures_iter(&html) {
        let slug = &cap[1];
        let lower = slug.to_lowercase();
        if seen.contains(&lower) || skip_set.contains(lower.as_str()) {
            continue;
        }
        seen.insert(lower);

        let mut name = slug.to_string();
        let mut image_url = None;

        if let Ok(detail) = fetch_course_detail(session, slug).await {
            if let Some(co) = detail.get("course").and_then(|v| v.as_object()) {
                let co_val = serde_json::Value::Object(co.clone());
                if let Some(n) = co_val.get("name").and_then(|v| v.as_str()) {
                    if !n.is_empty() {
                        name = n.to_string();
                    }
                }
                image_url = extract_image(&co_val);
            } else {
                if let Some(n) = detail.get("name").and_then(|v| v.as_str()) {
                    if !n.is_empty() {
                        name = n.to_string();
                    }
                }
                image_url = extract_image(&detail);
            }
        }

        courses.push(ThinkificCourse {
            id: slug.to_string(),
            name,
            slug: slug.to_string(),
            image_url,
        });
    }

    Ok(courses)
}

pub async fn get_course_content(
    session: &ThinkificSession,
    slug: &str,
) -> anyhow::Result<Vec<ThinkificModule>> {
    let data = fetch_course_detail(session, slug).await?;

    let chapters = data
        .get("chapters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let contents = data
        .get("contents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut ch_map: std::collections::HashMap<i64, (String, i64)> = std::collections::HashMap::new();
    for (i, ch) in chapters.iter().enumerate() {
        let id = ch.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let name = ch
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Chapter")
            .to_string();
        let pos = ch
            .get("position")
            .and_then(|v| v.as_i64())
            .unwrap_or(i as i64 + 1);
        ch_map.insert(id, (name, pos));
    }

    let mut modules_map: std::collections::HashMap<i64, ThinkificModule> =
        std::collections::HashMap::new();

    for (i, content) in contents.iter().enumerate() {
        let ch_id = content.get("chapter_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let (ch_name, ch_order) = ch_map
            .get(&ch_id)
            .cloned()
            .unwrap_or(("Extras".to_string(), 999));

        let contentable_id = content
            .get("contentable_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let contentable_type = content
            .get("contentable_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let lesson_name = content
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let lesson = ThinkificLesson {
            id: format!("{}", contentable_id),
            name: if lesson_name.is_empty() {
                format!("Lesson {}", i + 1)
            } else {
                lesson_name
            },
            order: i as i64,
            contentable_type,
            contentable_id,
        };

        modules_map
            .entry(ch_id)
            .or_insert_with(|| ThinkificModule {
                id: format!("{}", ch_id),
                name: ch_name,
                order: ch_order,
                lessons: Vec::new(),
            })
            .lessons
            .push(lesson);
    }

    let mut modules: Vec<ThinkificModule> = modules_map.into_values().collect();
    modules.sort_by_key(|m| m.order);
    for m in &mut modules {
        m.lessons.sort_by_key(|l| l.order);
    }

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &ThinkificSession,
    lesson: &ThinkificLesson,
) -> anyhow::Result<ThinkificLessonDetail> {
    let lesson_id = lesson.contentable_id;

    match lesson.contentable_type.as_str() {
        "Download" => {
            let files = fetch_download_files(session, lesson_id).await?;
            Ok(ThinkificLessonDetail {
                id: lesson.id.clone(),
                name: lesson.name.clone(),
                video_url: None,
                html_text: None,
                files,
            })
        }
        "HtmlItem" => {
            let html = fetch_html_item(session, lesson_id).await?;
            Ok(ThinkificLessonDetail {
                id: lesson.id.clone(),
                name: lesson.name.clone(),
                video_url: None,
                html_text: Some(html),
                files: Vec::new(),
            })
        }
        _ => {
            let url = format!(
                "{}/api/course_player/v2/lessons/{}",
                session.site_url, lesson_id
            );
            let resp = session.client.get(&url).send().await?;
            if !resp.status().is_success() {
                return Ok(ThinkificLessonDetail {
                    id: lesson.id.clone(),
                    name: lesson.name.clone(),
                    video_url: None,
                    html_text: None,
                    files: Vec::new(),
                });
            }

            let body: serde_json::Value = resp.json().await?;
            let lesson_obj = body.get("lesson").unwrap_or(&body);

            let video_url = lesson_obj
                .get("video_url")
                .or_else(|| lesson_obj.get("video_embed_url"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let html_text = lesson_obj
                .get("html_text")
                .and_then(|v| v.as_str())
                .map(String::from);

            let files = body
                .get("download_files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            let dl_url = f.get("download_url").and_then(|v| v.as_str())?;
                            let fname = f
                                .get("file_name")
                                .or_else(|| f.get("label"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("attachment")
                                .to_string();
                            Some(ThinkificFile {
                                name: fname,
                                url: dl_url.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(ThinkificLessonDetail {
                id: lesson.id.clone(),
                name: lesson.name.clone(),
                video_url,
                html_text,
                files,
            })
        }
    }
}

async fn fetch_download_files(
    session: &ThinkificSession,
    lesson_id: i64,
) -> anyhow::Result<Vec<ThinkificFile>> {
    let url = format!(
        "{}/api/course_player/v2/downloads/{}",
        session.site_url, lesson_id
    );

    let resp = session.client.get(&url).send().await;
    let body: serde_json::Value = match resp {
        Ok(r) if r.status().is_success() => r.json().await?,
        _ => {
            let fallback = format!(
                "{}/api/course_player/v2/lessons/{}",
                session.site_url, lesson_id
            );
            let r = session.client.get(&fallback).send().await?;
            if !r.status().is_success() {
                return Ok(Vec::new());
            }
            r.json().await?
        }
    };

    let files = body
        .get("download_files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    let dl_url = f.get("download_url").and_then(|v| v.as_str())?;
                    let fname = f
                        .get("file_name")
                        .or_else(|| f.get("label"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("file")
                        .to_string();
                    Some(ThinkificFile {
                        name: fname,
                        url: dl_url.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(files)
}

async fn fetch_html_item(
    session: &ThinkificSession,
    lesson_id: i64,
) -> anyhow::Result<String> {
    let url = format!(
        "{}/api/course_player/v2/html_items/{}",
        session.site_url, lesson_id
    );

    let resp = session.client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(String::new());
    }

    let body: serde_json::Value = resp.json().await?;
    let item = body.get("html_item").unwrap_or(&body);

    let content = item
        .get("html_text")
        .or_else(|| item.get("body"))
        .or_else(|| item.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(content)
}

pub fn resolve_video_url(page_html: &str, fallback_url: &str) -> String {
    let player_re = Regex::new(r"player\.thinkific\.com/embed/([a-f0-9-]+)").ok();
    let mp4_re = Regex::new(r#"https?://[^\s"']+\.mp4"#).ok();
    let wistia_re = Regex::new(r#"(?i)(?:wistia_async_|fast\.wistia\.com/embed/(?:medias|iframe)/|hashedId["']?\s*[:=]\s*["'])([a-z0-9]+)"#).ok();
    let cdn_re = Regex::new(r#"(https://(?:d2p6ecj15pyavq|d1fto35gcfffzn)\.cloudfront\.net/[^\s"']+\.(?:mp4|m3u8)|https://player-api\.thinkific\.com/(?:hls|api/video)/[^\s"']+)"#).ok();

    if let Some(re) = wistia_re {
        if let Some(cap) = re.captures(page_html) {
            return format!("https://fast.wistia.com/embed/iframe/{}", &cap[1]);
        }
    }

    if let Some(re) = cdn_re {
        if let Some(cap) = re.captures(page_html) {
            return cap[1].to_string();
        }
    }

    if let Some(re) = player_re {
        if let Some(cap) = re.captures(page_html) {
            return format!("https://player.thinkific.com/embed/{}", &cap[1]);
        }
    }

    if let Some(re) = mp4_re {
        if let Some(m) = re.find(page_html) {
            return m.as_str().to_string();
        }
    }

    fallback_url.to_string()
}

pub async fn save_session(session: &ThinkificSession) -> anyhow::Result<()> {
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
    tracing::info!("[thinkific] session saved for {}", session.site_url);
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<ThinkificSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookies, &saved.site_url)?;

    tracing::info!("[thinkific] session loaded for {}", saved.site_url);

    Ok(Some(ThinkificSession {
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
