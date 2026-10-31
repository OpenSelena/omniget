use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

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
    #[serde(default)]
    pub detail: SkoolLessonDetail,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkoolLessonDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub video_url: Option<String>,
    #[serde(default)]
    pub files: Vec<SkoolFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolFile {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

pub fn build_client(cookie_token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    let trimmed = cookie_token.trim();
    let cookie_header = if trimmed.contains('=') {
        trimmed.to_string()
    } else {
        format!("auth_token={}; skooltok={}", trimmed, trimmed)
    };

    headers.insert("Cookie", HeaderValue::from_str(&cookie_header)?);
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
    let data_dir = dirs::data_dir().ok_or_else(|| anyhow!("Could not find app data directory"))?;
    Ok(data_dir.join("omniget").join("skool_session.json"))
}

pub fn extract_build_id(html: &str) -> anyhow::Result<String> {
    let re = regex::Regex::new(r#""buildId"\s*:\s*"([^"]+)""#)?;
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("Could not extract buildId from Skool HTML"))
}

pub async fn fetch_build_id(client: &reqwest::Client) -> anyhow::Result<String> {
    let resp = client.get("https://www.skool.com").send().await?;
    let html = resp.text().await?;
    extract_build_id(&html)
}

pub async fn authenticate(email: &str, password: &str) -> anyhow::Result<SkoolSession> {
    let temp_client =
        omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
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
        if cookie.name() == "auth_token" || cookie.name() == "skooltok" {
            cookie_token = cookie.value().to_string();
            break;
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
            .or_else(|| body.get("auth_token"))
            .or_else(|| body.get("skooltok"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    if cookie_token.is_empty() {
        return Err(anyhow!(
            "No auth_token or skooltok cookie in login response"
        ));
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

        let lessons = get_course_lessons(session, group_slug, &course_slug)
            .await
            .unwrap_or_default();

        modules.push(SkoolModule {
            id: if course_id.is_empty() {
                course_slug
            } else {
                course_id
            },
            name: course_name,
            order: i as i64,
            lessons,
        });
    }

    Ok(modules)
}

pub fn parse_next_redirect_md(body: &serde_json::Value) -> Option<String> {
    let redirect = body.get("__N_REDIRECT").and_then(|v| v.as_str())?;
    let target = if redirect.starts_with("http") {
        redirect.to_string()
    } else {
        format!("https://www.skool.com{}", redirect)
    };
    let parsed = url::Url::parse(&target).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == "md")
        .map(|(_, v)| v.to_string())
}

pub fn parse_lesson_detail(child: &serde_json::Value) -> SkoolLessonDetail {
    let lesson_id = child
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let metadata = child.get("metadata").unwrap_or(child);

    let name = metadata
        .get("title")
        .or_else(|| metadata.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let description = metadata
        .get("description")
        .or_else(|| metadata.get("content"))
        .or_else(|| metadata.get("body"))
        .or_else(|| metadata.get("text"))
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
        .or_else(|| metadata.get("video_url"))
        .or_else(|| metadata.get("video"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut files = Vec::new();

    let raw_attachments = metadata
        .get("attachments")
        .or_else(|| metadata.get("files"))
        .and_then(|v| v.as_array());

    if let Some(attachments) = raw_attachments {
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

    SkoolLessonDetail {
        id: lesson_id,
        name,
        description,
        video_url,
        files,
    }
}

fn parse_lesson_node(child: &serde_json::Value, order: i64) -> SkoolLesson {
    let metadata = child.get("metadata").unwrap_or(child);

    let lesson_id = child
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let lesson_name = metadata
        .get("title")
        .or_else(|| metadata.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let detail = parse_lesson_detail(child);

    SkoolLesson {
        id: if lesson_id.is_empty() {
            format!("{}", order)
        } else {
            lesson_id
        },
        name: if lesson_name.is_empty() {
            format!("Lesson {}", order)
        } else {
            lesson_name
        },
        order,
        detail,
    }
}

pub fn parse_course_lessons(body: &serde_json::Value) -> Vec<SkoolLesson> {
    let children = body
        .pointer("/pageProps/course/children")
        .or_else(|| body.pointer("/pageProps/children"))
        .or_else(|| body.pointer("/pageProps/selectedModule/children"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lessons = Vec::new();
    let mut order = 0;

    for child in &children {
        if let Some(sub_children) = child
            .get("children")
            .and_then(|v| v.as_array())
            .filter(|a| !a.is_empty())
        {
            for sub in sub_children {
                order += 1;
                lessons.push(parse_lesson_node(sub, order));
            }
        } else {
            order += 1;
            lessons.push(parse_lesson_node(child, order));
        }
    }

    lessons
}

async fn get_course_lessons(
    session: &SkoolSession,
    group_slug: &str,
    course_slug: &str,
) -> anyhow::Result<Vec<SkoolLesson>> {
    let mut url = format!(
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

    let mut body: serde_json::Value = serde_json::from_str(&body_text)?;

    if let Some(md_val) = parse_next_redirect_md(&body) {
        url = format!(
            "https://www.skool.com/_next/data/{}/{}/classroom/{}.json?group={}&course={}&md={}",
            session.build_id, group_slug, course_slug, group_slug, course_slug, md_val
        );

        let redirect_resp = session.client.get(&url).send().await?;
        if redirect_resp.status().is_success() {
            let redirect_text = redirect_resp.text().await?;
            if let Ok(parsed) = serde_json::from_str(&redirect_text) {
                body = parsed;
            }
        }
    }

    Ok(parse_course_lessons(&body))
}

pub async fn get_lesson_detail(
    _session: &SkoolSession,
    _group_slug: &str,
    _course_slug: &str,
    lesson_id: &str,
    children: &[serde_json::Value],
) -> anyhow::Result<SkoolLessonDetail> {
    if let Some(child) = children.iter().find(|c| {
        c.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == lesson_id)
            .unwrap_or(false)
    }) {
        Ok(parse_lesson_detail(child))
    } else {
        Ok(SkoolLessonDetail {
            id: lesson_id.to_string(),
            name: String::new(),
            description: None,
            video_url: None,
            files: Vec::new(),
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_build_id() {
        let html = r#"<!DOCTYPE html><html><head><script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{}},"page":"/","query":{},"buildId":"b1d_xyz_12345","isFallback":false,"gip":true}</script></head><body></body></html>"#;
        let id = extract_build_id(html).unwrap();
        assert_eq!(id, "b1d_xyz_12345");

        let bad_html = "<html><body>No id here</body></html>";
        assert!(extract_build_id(bad_html).is_err());
    }

    #[test]
    fn test_parse_next_redirect_md() {
        let redirect_json = serde_json::json!({
            "__N_REDIRECT": "/my-group/classroom/intro-course?md=module_abc999",
            "__N_REDIRECT_STATUS": 307
        });
        let md = parse_next_redirect_md(&redirect_json);
        assert_eq!(md.as_deref(), Some("module_abc999"));

        let normal_json = serde_json::json!({
            "pageProps": {
                "course": { "children": [] }
            }
        });
        assert_eq!(parse_next_redirect_md(&normal_json), None);
    }

    #[test]
    fn test_parse_course_lessons_from_page_props() {
        let payload = serde_json::json!({
            "pageProps": {
                "course": {
                    "children": [
                        {
                            "id": "les_1",
                            "metadata": {
                                "title": "Welcome Lesson",
                                "description": "<p>Intro text</p>",
                                "videoLink": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                                "attachments": [
                                    { "id": "att_1", "name": "cheatsheet.pdf" }
                                ]
                            }
                        },
                        {
                            "id": "les_2",
                            "metadata": {
                                "title": "Lesson 2 Without Video"
                            }
                        }
                    ]
                }
            }
        });

        let lessons = parse_course_lessons(&payload);
        assert_eq!(lessons.len(), 2);
        assert_eq!(lessons[0].id, "les_1");
        assert_eq!(lessons[0].name, "Welcome Lesson");
        assert_eq!(
            lessons[0].detail.video_url.as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
        assert_eq!(
            lessons[0].detail.description.as_deref(),
            Some("<p>Intro text</p>")
        );
        assert_eq!(lessons[0].detail.files.len(), 1);
        assert_eq!(lessons[0].detail.files[0].name, "cheatsheet.pdf");

        assert_eq!(lessons[1].id, "les_2");
        assert_eq!(lessons[1].name, "Lesson 2 Without Video");
        assert_eq!(lessons[1].detail.video_url, None);
    }

    #[test]
    fn test_parse_course_lessons_nested_module_structure() {
        let payload = serde_json::json!({
            "pageProps": {
                "course": {
                    "children": [
                        {
                            "id": "mod_1",
                            "name": "Module 1",
                            "children": [
                                {
                                    "id": "les_nested_1",
                                    "metadata": {
                                        "title": "Nested Lesson 1"
                                    }
                                }
                            ]
                        }
                    ]
                }
            }
        });

        let lessons = parse_course_lessons(&payload);
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].id, "les_nested_1");
        assert_eq!(lessons[0].name, "Nested Lesson 1");
    }
}
