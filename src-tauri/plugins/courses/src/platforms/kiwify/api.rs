use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const CDN_BASE: &str = "https://d3pjuhbfoxhm7c.cloudfront.net";

#[derive(Clone)]
pub struct KiwifySession {
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
pub struct KiwifyCourse {
    pub id: String,
    pub name: String,
    pub seller: String,
    pub image_url: Option<String>,
    pub is_school: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<KiwifyLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyLesson {
    pub id: String,
    pub name: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyLessonDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub video_url: Option<String>,
    pub files: Vec<KiwifyFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyFile {
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
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://admin.kiwify.com.br"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://admin.kiwify.com.br/"),
    );

    let client = libreget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()?;

    Ok(client)
}

fn session_file_path() -> anyhow::Result<PathBuf> {
    crate::platforms::session_file("kiwify_session.json")
}

pub async fn authenticate(email: &str, password: &str) -> anyhow::Result<KiwifySession> {
    let temp_client =
        libreget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(60))
            .build()?;

    let payload = serde_json::json!({
        "email": email,
        "password": password,
        "returnSecureToken": true,
    });

    let resp = temp_client
        .post("https://admin-api.kiwify.com.br/v1/handleAuth/login")
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
        .get("idToken")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("idToken not found in login response"))?
        .to_string();

    let client = build_client(&token)?;

    Ok(KiwifySession {
        token,
        email: email.to_string(),
        client,
    })
}

pub async fn validate_token(session: &KiwifySession) -> anyhow::Result<bool> {
    let resp = session
        .client
        .get("https://admin-api.kiwify.com.br/v1/viewer/schools/courses?page=1&archived=false")
        .send()
        .await?;

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &KiwifySession) -> anyhow::Result<Vec<KiwifyCourse>> {
    let mut all_courses = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "https://admin-api.kiwify.com.br/v1/viewer/schools/courses?page={}&archived=false",
            page
        );

        tracing::info!("[kiwify] fetching courses page {}", page);

        let resp = session.client.get(&url).send().await?;

        let status = resp.status();
        let body_text = resp.text().await?;

        tracing::info!(
            "[kiwify] page {} status={} body_len={}",
            page,
            status,
            body_text.len()
        );

        if !status.is_success() {
            if all_courses.is_empty() {
                return Err(anyhow!(
                    "list_courses returned status {}: {}",
                    status,
                    &body_text[..body_text.len().min(300)]
                ));
            }
            break;
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;

        let total_count = body.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        let page_size = body.get("page_size").and_then(|v| v.as_i64()).unwrap_or(10);

        let page_courses = parse_courses_page(&body);
        tracing::info!(
            "[kiwify] page {} returned {} courses (total={}, page_size={})",
            page,
            page_courses.len(),
            total_count,
            page_size
        );

        if page_courses.is_empty() {
            break;
        }
        all_courses.extend(page_courses);

        if total_count > 0 && (page as i64 * page_size) >= total_count {
            break;
        }

        if page >= 50 {
            tracing::warn!("[kiwify] safety limit reached at page 50");
            break;
        }

        page += 1;
    }

    tracing::info!("[kiwify] total courses found: {}", all_courses.len());
    Ok(all_courses)
}

fn first_non_empty_str<'a>(obj: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        obj.get(*key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    })
}

fn extract_id_string(obj: &serde_json::Value) -> Option<String> {
    obj.get("id").and_then(|v| {
        v.as_str()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })
}

pub fn parse_courses_page(body: &serde_json::Value) -> Vec<KiwifyCourse> {
    let Some(courses_arr) = body.get("courses").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut courses = Vec::new();
    for item in courses_arr {
        let course_in_school = item
            .get("course_in_school")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (info_obj, is_school) = if course_in_school {
            (item.get("school_info").unwrap_or(item), true)
        } else {
            (item.get("course_info").unwrap_or(item), false)
        };

        let id = extract_id_string(info_obj)
            .or_else(|| extract_id_string(item))
            .unwrap_or_default();

        let name = first_non_empty_str(info_obj, &["name"])
            .or_else(|| first_non_empty_str(item, &["name"]))
            .unwrap_or("")
            .to_string();

        let seller = info_obj
            .get("producer")
            .and_then(|p| first_non_empty_str(p, &["name"]))
            .or_else(|| {
                item.get("producer")
                    .and_then(|p| first_non_empty_str(p, &["name"]))
            })
            .unwrap_or("")
            .to_string();

        let image_url =
            first_non_empty_str(info_obj, &["course_img", "logo_url", "image", "thumbnail"])
                .map(String::from);

        courses.push(KiwifyCourse {
            id,
            name,
            seller,
            image_url,
            is_school,
        });
    }
    courses
}

pub async fn get_course_content(
    session: &KiwifySession,
    course_id: &str,
) -> anyhow::Result<Vec<KiwifyModule>> {
    let club_url = format!(
        "https://admin-api.kiwify.com.br/v1/viewer/clubs/{}/content?caipirinha=true",
        course_id
    );

    let body = match session.client.get(&club_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await?;
            serde_json::from_str::<serde_json::Value>(&text)?
        }
        _ => {
            let fallback_url = format!(
                "https://admin-api.kiwify.com.br/v1/viewer/courses/{}",
                course_id
            );
            let resp = session.client.get(&fallback_url).send().await?;
            let status = resp.status();
            let text = resp.text().await?;
            if !status.is_success() {
                return Err(anyhow!(
                    "get_course_content returned status {}: {}",
                    status,
                    &text[..text.len().min(300)]
                ));
            }
            serde_json::from_str::<serde_json::Value>(&text)?
        }
    };

    parse_course_modules(&body)
}

pub fn parse_course_modules(body: &serde_json::Value) -> anyhow::Result<Vec<KiwifyModule>> {
    let modules_obj = body
        .get("course")
        .and_then(|c| c.get("modules"))
        .or_else(|| body.get("content").and_then(|c| c.get("modules")))
        .or_else(|| body.get("modules"))
        .or_else(|| body.get("content"))
        .ok_or_else(|| anyhow!("No modules found in course content response"))?;

    let mut modules = Vec::new();

    match modules_obj {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(_, v)| v.get("order").and_then(|o| o.as_i64()).unwrap_or(0));

            for (key, module_val) in entries {
                let module_name = module_val
                    .get("title")
                    .or_else(|| module_val.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(key)
                    .to_string();

                let module_order = module_val
                    .get("order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                let mut lessons = Vec::new();

                let lessons_obj = module_val
                    .get("lessons")
                    .or_else(|| module_val.get("classes"));

                if let Some(serde_json::Value::Object(lessons_map)) = lessons_obj {
                    let mut lesson_entries: Vec<_> = lessons_map.iter().collect();
                    lesson_entries
                        .sort_by_key(|(_, v)| v.get("order").and_then(|o| o.as_i64()).unwrap_or(0));

                    for (lesson_key, lesson_val) in lesson_entries {
                        let lesson_name = lesson_val
                            .get("title")
                            .or_else(|| lesson_val.get("name"))
                            .or_else(|| lesson_val.get("ref"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(lesson_key)
                            .to_string();

                        let lesson_order = lesson_val
                            .get("order")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        lessons.push(KiwifyLesson {
                            id: lesson_key.clone(),
                            name: lesson_name,
                            order: lesson_order,
                        });
                    }
                } else if let Some(serde_json::Value::Array(lessons_arr)) = lessons_obj {
                    for (i, lesson_val) in lessons_arr.iter().enumerate() {
                        let lesson_id = lesson_val
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let lesson_name = lesson_val
                            .get("title")
                            .or_else(|| lesson_val.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let lesson_order = lesson_val
                            .get("order")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(i as i64);

                        lessons.push(KiwifyLesson {
                            id: if lesson_id.is_empty() {
                                format!("{}", i)
                            } else {
                                lesson_id
                            },
                            name: lesson_name,
                            order: lesson_order,
                        });
                    }
                }

                modules.push(KiwifyModule {
                    id: key.clone(),
                    name: module_name,
                    order: module_order,
                    lessons,
                });
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, module_val) in arr.iter().enumerate() {
                let module_id = module_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let module_name = module_val
                    .get("title")
                    .or_else(|| module_val.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let module_order = module_val
                    .get("order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(i as i64);

                let mut lessons = Vec::new();

                if let Some(serde_json::Value::Array(lessons_arr)) = module_val
                    .get("lessons")
                    .or_else(|| module_val.get("classes"))
                {
                    for (j, lesson_val) in lessons_arr.iter().enumerate() {
                        let lesson_id = lesson_val
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let lesson_name = lesson_val
                            .get("title")
                            .or_else(|| lesson_val.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let lesson_order = lesson_val
                            .get("order")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(j as i64);

                        lessons.push(KiwifyLesson {
                            id: if lesson_id.is_empty() {
                                format!("{}", j)
                            } else {
                                lesson_id
                            },
                            name: lesson_name,
                            order: lesson_order,
                        });
                    }
                }

                modules.push(KiwifyModule {
                    id: if module_id.is_empty() {
                        format!("{}", i)
                    } else {
                        module_id
                    },
                    name: module_name,
                    order: module_order,
                    lessons,
                });
            }
        }
        _ => {
            return Err(anyhow!("Unexpected modules format in course content"));
        }
    }

    modules.sort_by_key(|m| m.order);
    for m in &mut modules {
        m.lessons.sort_by_key(|l| l.order);
    }

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &KiwifySession,
    course_id: &str,
    lesson_id: &str,
) -> anyhow::Result<KiwifyLessonDetail> {
    let url = format!(
        "https://admin-api.kiwify.com.br/v1/viewer/courses/{}/lesson/{}",
        course_id, lesson_id
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
    Ok(parse_lesson_detail(&body, lesson_id))
}

pub fn parse_lesson_detail(body: &serde_json::Value, fallback_id: &str) -> KiwifyLessonDetail {
    let lesson = body.get("lesson").unwrap_or(body);

    let id = first_non_empty_str(lesson, &["id"])
        .unwrap_or(fallback_id)
        .to_string();

    let name = first_non_empty_str(lesson, &["title", "name"])
        .unwrap_or("")
        .to_string();

    let description =
        first_non_empty_str(lesson, &["description", "content", "body"]).map(String::from);

    let video_url = lesson
        .get("video")
        .and_then(|v| {
            first_non_empty_str(v, &["stream_link", "download_link", "url"])
                .or_else(|| v.as_str().filter(|s| !s.trim().is_empty()))
        })
        .or_else(|| first_non_empty_str(lesson, &["stream_link", "video_url"]))
        .map(|url| {
            if url.starts_with('/') {
                format!("{}{}", CDN_BASE, url)
            } else {
                url.to_string()
            }
        });

    let files = lesson
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|f| {
                    let file_id = f
                        .get("id")
                        .map(|v| match v {
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::String(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();

                    let file_name = f
                        .get("name")
                        .or_else(|| f.get("filename"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("file")
                        .to_string();

                    let file_url = f.get("url").and_then(|v| v.as_str()).map(String::from);

                    KiwifyFile {
                        id: file_id,
                        name: file_name,
                        url: file_url,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    KiwifyLessonDetail {
        id,
        name,
        description,
        video_url,
        files,
    }
}

pub async fn get_file_download_url(
    session: &KiwifySession,
    course_id: &str,
    file_id: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://admin-api.kiwify.com.br/v1/viewer/courses/{}/files/{}?forceDownload=true",
        course_id, file_id
    );

    let resp = session.client.get(&url).send().await?;

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

pub async fn save_session(session: &KiwifySession) -> anyhow::Result<()> {
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
    tracing::info!("[kiwify] session saved for {}", session.email);
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<KiwifySession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.token)?;

    tracing::info!("[kiwify] session loaded for {}", saved.email);

    Ok(Some(KiwifySession {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_courses_page_extracts_uuid_and_course_img() {
        let payload = json!({
            "count": 2,
            "page_size": 10,
            "courses": [
                {
                    "course_in_school": false,
                    "course_info": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "name": "Curso Kiwify Pro",
                        "course_img": "https://cdn.kiwify.com.br/cover.png",
                        "producer": { "name": "Produtor A" }
                    }
                },
                {
                    "course_in_school": true,
                    "school_info": {
                        "id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
                        "name": "Escola Premium",
                        "course_img": "",
                        "logo_url": "https://cdn.kiwify.com.br/logo.png",
                        "producer": { "name": "Produtor B" }
                    }
                }
            ]
        });

        let courses = parse_courses_page(&payload);
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(courses[0].name, "Curso Kiwify Pro");
        assert_eq!(courses[0].seller, "Produtor A");
        assert_eq!(
            courses[0].image_url.as_deref(),
            Some("https://cdn.kiwify.com.br/cover.png")
        );
        assert!(!courses[0].is_school);

        assert_eq!(courses[1].id, "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d");
        assert_eq!(
            courses[1].image_url.as_deref(),
            Some("https://cdn.kiwify.com.br/logo.png")
        );
        assert!(courses[1].is_school);
    }

    #[test]
    fn parse_course_modules_unwraps_nested_course_and_content_objects() {
        let course_payload = json!({
            "course": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Curso Kiwify Pro",
                "modules": [
                    {
                        "id": "mod-2",
                        "name": "Módulo 2",
                        "order": 2,
                        "lessons": [
                            { "id": "les-2", "title": "Aula 2", "order": 1 }
                        ]
                    },
                    {
                        "id": "mod-1",
                        "name": "Módulo 1",
                        "order": 1,
                        "lessons": [
                            { "id": "les-1b", "title": "Aula 1B", "order": 2 },
                            { "id": "les-1a", "title": "Aula 1A", "order": 1 }
                        ]
                    }
                ]
            },
            "certificate_enabled": true
        });

        let modules =
            parse_course_modules(&course_payload).expect("should parse nested course.modules");
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0].id, "mod-1");
        assert_eq!(modules[0].name, "Módulo 1");
        assert_eq!(modules[0].lessons.len(), 2);
        assert_eq!(modules[0].lessons[0].id, "les-1a");
        assert_eq!(modules[0].lessons[0].name, "Aula 1A");
        assert_eq!(modules[1].id, "mod-2");

        let club_payload = json!({
            "content": {
                "modules": [
                    {
                        "id": "club-mod-1",
                        "title": "Boas-vindas",
                        "order": 1,
                        "lessons": [
                            { "id": "club-les-1", "title": "Comece aqui", "order": 1 }
                        ]
                    }
                ]
            }
        });

        let club_modules =
            parse_course_modules(&club_payload).expect("should parse nested content.modules");
        assert_eq!(club_modules.len(), 1);
        assert_eq!(club_modules[0].id, "club-mod-1");
        assert_eq!(club_modules[0].lessons[0].id, "club-les-1");
    }

    #[test]
    fn parse_lesson_detail_extracts_nested_video_stream_link_and_files() {
        let payload = json!({
            "lesson": {
                "id": "les-1a",
                "title": "Aula 1A - Introdução",
                "content": "<p>Material de apoio</p>",
                "video": {
                    "id": "vid-1",
                    "name": "aula1.mp4",
                    "stream_link": "/courses/550e8400/videos/master.m3u8",
                    "download_link": null
                },
                "files": [
                    {
                        "id": "file-uuid-1",
                        "name": "slides.pdf"
                    }
                ]
            }
        });

        let detail = parse_lesson_detail(&payload, "fallback-id");
        assert_eq!(detail.id, "les-1a");
        assert_eq!(detail.name, "Aula 1A - Introdução");
        assert_eq!(
            detail.description.as_deref(),
            Some("<p>Material de apoio</p>")
        );
        assert_eq!(
            detail.video_url.as_deref(),
            Some("https://d3pjuhbfoxhm7c.cloudfront.net/courses/550e8400/videos/master.m3u8")
        );
        assert_eq!(detail.files.len(), 1);
        assert_eq!(detail.files[0].id, "file-uuid-1");
        assert_eq!(detail.files[0].name, "slides.pdf");

        let fallback_video_payload = json!({
            "lesson": {
                "id": "les-1b",
                "title": "Aula 1B",
                "video": {
                    "id": "vid-2",
                    "stream_link": null,
                    "download_link": "/courses/550e8400/videos/aula1b.mp4"
                }
            }
        });
        let fallback_detail = parse_lesson_detail(&fallback_video_payload, "fallback-id");
        assert_eq!(
            fallback_detail.video_url.as_deref(),
            Some("https://d3pjuhbfoxhm7c.cloudfront.net/courses/550e8400/videos/aula1b.mp4")
        );
    }
}
