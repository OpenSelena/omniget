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
    /// A club can contain several internal courses. Lesson/file endpoints use this ID.
    #[serde(default)]
    pub course_id: Option<String>,
    #[serde(default = "default_available")]
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyLessonDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub video_url: Option<String>,
    pub files: Vec<KiwifyFile>,
    #[serde(default = "default_available")]
    pub available: bool,
}

fn default_available() -> bool {
    true
}

fn content_is_available(value: &serde_json::Value) -> bool {
    let user = value.get("user_data").unwrap_or(value);
    !["locked", "expired"].iter().any(|key| {
        value.get(*key).and_then(|v| v.as_bool()) == Some(true)
            || user.get(*key).and_then(|v| v.as_bool()) == Some(true)
    }) && ["published", "active", "has_access"]
        .iter()
        .all(|key| value.get(*key).and_then(|v| v.as_bool()) != Some(false))
        && user.get("visible").and_then(|v| v.as_bool()) != Some(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiwifyFile {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

pub(crate) fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    let mut authorization = HeaderValue::from_str(&format!("Bearer {}", token))
        .map_err(|_| anyhow!("Invalid Kiwify token header"))?;
    authorization.set_sensitive(true);
    headers.insert("Authorization", authorization);
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://admin.kiwify.com.br"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://admin.kiwify.com.br/"),
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

/// CDN requests must never inherit the API bearer token.
pub(crate) fn build_media_client() -> anyhow::Result<reqwest::Client> {
    Ok(
        omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
            .user_agent(USER_AGENT)
            .referer(true)
            .default_headers({
                let mut headers = HeaderMap::new();
                headers.insert(
                    "Referer",
                    HeaderValue::from_static("https://admin.kiwify.com.br/"),
                );
                headers
            })
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()?,
    )
}

async fn checked_json(
    resp: reqwest::Response,
    operation: &str,
) -> anyhow::Result<serde_json::Value> {
    let status = resp.status();
    if !status.is_success() {
        // Response bodies and request URLs may contain private data or signed links.
        return Err(anyhow!(
            "Kiwify {} failed (HTTP {})",
            operation,
            status.as_u16()
        ));
    }
    resp.json()
        .await
        .map_err(|_| anyhow!("Invalid JSON in Kiwify {} response", operation))
}

fn session_file_path() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_dir().ok_or_else(|| anyhow!("Could not find app data directory"))?;
    Ok(data_dir.join("omniget").join("kiwify_session.json"))
}

pub async fn authenticate(email: &str, password: &str) -> anyhow::Result<KiwifySession> {
    let temp_client =
        omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
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
        return Err(anyhow!("Authentication failed (HTTP {})", status.as_u16()));
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

    match resp.status().as_u16() {
        200..=299 => Ok(true),
        401 | 403 => Ok(false),
        status => Err(anyhow!("Kiwify session check failed (HTTP {})", status)),
    }
}

pub async fn list_courses(session: &KiwifySession) -> anyhow::Result<Vec<KiwifyCourse>> {
    list_courses_at(session, "https://admin-api.kiwify.com.br/v1/viewer").await
}

async fn list_courses_at(session: &KiwifySession, base: &str) -> anyhow::Result<Vec<KiwifyCourse>> {
    let mut all_courses = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut page = 1u32;

    loop {
        let url = format!("{}/schools/courses?page={}&archived=false", base, page);

        tracing::info!("[kiwify] fetching courses page {}", page);

        let resp = session
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.without_url())?;
        let body = checked_json(resp, "course list").await?;
        let body = body.get("data").unwrap_or(&body);
        if !body.get("courses").is_some_and(|v| v.is_array()) {
            return Err(anyhow!("Invalid Kiwify course list: missing courses array"));
        }

        let total_count = body.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        let page_size = body.get("page_size").and_then(|v| v.as_i64()).unwrap_or(10);

        let page_courses = parse_courses_page(body);
        tracing::info!(
            "[kiwify] page {} returned {} courses (total={}, page_size={})",
            page,
            page_courses.len(),
            total_count,
            page_size
        );

        if page_courses.is_empty() {
            if total_count > all_courses.len() as i64 {
                return Err(anyhow!(
                    "Kiwify course list ended before the reported total"
                ));
            }
            break;
        }
        let before = all_courses.len();
        all_courses.extend(
            page_courses
                .into_iter()
                .filter(|c| !c.id.is_empty() && seen.insert(c.id.clone())),
        );
        if all_courses.len() == before {
            return Err(anyhow!(
                "Kiwify course list repeated a page without new course IDs"
            ));
        }

        if total_count > 0 && all_courses.len() as i64 >= total_count {
            break;
        }

        if page >= 50 {
            return Err(anyhow!(
                "Kiwify course list exceeded the 50-page safety limit"
            ));
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
    let body = body.get("data").unwrap_or(body);
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
    get_course_content_at(
        session,
        course_id,
        "https://admin-api.kiwify.com.br/v1/viewer",
    )
    .await
}

async fn get_course_content_at(
    session: &KiwifySession,
    course_id: &str,
    base: &str,
) -> anyhow::Result<Vec<KiwifyModule>> {
    let course_id = urlencoding::encode(course_id);
    let club_url = format!("{}/clubs/{}/content?caipirinha=true", base, course_id);

    let resp = session
        .client
        .get(&club_url)
        .send()
        .await
        .map_err(|e| e.without_url())?;
    let body = match resp.status().as_u16() {
        404 | 405 => {
            let fallback_url = format!("{}/courses/{}", base, course_id);
            let resp = session
                .client
                .get(&fallback_url)
                .send()
                .await
                .map_err(|e| e.without_url())?;
            checked_json(resp, "course content").await?
        }
        _ => checked_json(resp, "club content").await?,
    };

    parse_course_modules(&body)
}

/// Caipirinha/v3 is a graph, not a nested list. The reference arrays define
/// membership and display order; map iteration would include orphaned records
/// and reorder UUIDs alphabetically.
fn parse_normalized_modules(body: &serde_json::Value) -> anyhow::Result<Vec<KiwifyModule>> {
    use std::collections::HashSet;

    fn map<'a>(
        body: &'a serde_json::Value,
        field: &str,
    ) -> anyhow::Result<&'a serde_json::Map<String, serde_json::Value>> {
        body.get(field).and_then(|v| v.as_object()).ok_or_else(|| {
            anyhow!(
                "Invalid Kiwify normalized content: {} must be an object",
                field
            )
        })
    }

    fn refs<'a>(record: &'a serde_json::Value, field: &str) -> anyhow::Result<Vec<&'a str>> {
        record
            .get(field)
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                anyhow!(
                    "Invalid Kiwify normalized content: missing {} references",
                    field
                )
            })?
            .iter()
            .map(|v| {
                v.as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| {
                    anyhow!(
                        "Invalid Kiwify normalized content: invalid {} reference",
                        field
                    )
                })
            })
            .collect()
    }

    fn record<'a>(
        map: &'a serde_json::Map<String, serde_json::Value>,
        id: &str,
        kind: &str,
    ) -> anyhow::Result<&'a serde_json::Value> {
        let value = map.get(id).filter(|v| v.is_object()).ok_or_else(|| {
            anyhow!(
                "Invalid Kiwify normalized content: unresolved {} reference",
                kind
            )
        })?;
        if value
            .get("id")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v != id)
        {
            return Err(anyhow!(
                "Invalid Kiwify normalized content: mismatched {} ID",
                kind
            ));
        }
        Ok(value)
    }

    fn check_parent(value: &serde_json::Value, field: &str, parent: &str) -> anyhow::Result<()> {
        if value
            .get(field)
            .and_then(|v| v.as_str())
            .is_some_and(|v| v != parent)
        {
            return Err(anyhow!(
                "Invalid Kiwify normalized content: mismatched {}",
                field
            ));
        }
        Ok(())
    }

    let courses = map(body, "all_courses")?;
    let module_map = map(body, "all_modules")?;
    let lesson_map = map(body, "all_lessons")?;
    let mut course_entries: Vec<_> = courses.iter().collect();
    course_entries.sort_by_key(|(_, c)| c.get("order").and_then(|v| v.as_i64()).unwrap_or(0));
    let mut modules = Vec::new();
    let mut seen_modules = HashSet::new();

    for (course_id, _) in course_entries {
        let course = record(courses, course_id, "course")?;
        for module_id in refs(course, "modules")? {
            let module = record(module_map, module_id, "module")?;
            check_parent(module, "course_id", course_id)?;
            if !seen_modules.insert(module_id) {
                continue;
            }
            let mut lessons = Vec::new();
            let mut seen_lessons = HashSet::new();
            for lesson_id in refs(module, "lessons")? {
                if !seen_lessons.insert(lesson_id) {
                    continue;
                }
                let lesson = record(lesson_map, lesson_id, "lesson")?;
                check_parent(lesson, "module_id", module_id)?;
                check_parent(lesson, "course_id", course_id)?;
                lessons.push(KiwifyLesson {
                    id: lesson_id.to_string(),
                    name: first_non_empty_str(lesson, &["name", "title", "ref"])
                        .unwrap_or(lesson_id)
                        .to_string(),
                    order: lessons.len() as i64,
                    course_id: Some(course_id.clone()),
                    available: content_is_available(course)
                        && content_is_available(module)
                        && content_is_available(lesson),
                });
            }
            modules.push(KiwifyModule {
                id: module_id.to_string(),
                name: first_non_empty_str(module, &["name", "title"])
                    .unwrap_or(module_id)
                    .to_string(),
                order: modules.len() as i64,
                lessons,
            });
        }
    }
    Ok(modules)
}

pub fn parse_course_modules(body: &serde_json::Value) -> anyhow::Result<Vec<KiwifyModule>> {
    let body = body.get("data").unwrap_or(body);
    if body.get("all_modules").is_some() || body.get("all_courses").is_some() {
        return parse_normalized_modules(body);
    }
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
                            course_id: extract_id_string(body.get("course").unwrap_or(body)),
                            available: content_is_available(module_val)
                                && content_is_available(lesson_val),
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
                            course_id: extract_id_string(body.get("course").unwrap_or(body)),
                            available: content_is_available(module_val)
                                && content_is_available(lesson_val),
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
                            course_id: extract_id_string(body.get("course").unwrap_or(body)),
                            available: content_is_available(module_val)
                                && content_is_available(lesson_val),
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

    let resp = session
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.without_url())?;
    let body = checked_json(resp, "lesson detail").await?;
    let lesson_body = body.get("data").unwrap_or(&body);
    if !lesson_body.get("lesson").unwrap_or(lesson_body).is_object()
        || !lesson_body
            .get("lesson")
            .unwrap_or(lesson_body)
            .get("id")
            .is_some_and(|v| v.is_string())
    {
        return Err(anyhow!(
            "Invalid Kiwify lesson detail: missing lesson object or ID"
        ));
    }
    Ok(parse_lesson_detail(&body, lesson_id))
}

pub fn parse_lesson_detail(body: &serde_json::Value, fallback_id: &str) -> KiwifyLessonDetail {
    let body = body.get("data").unwrap_or(body);
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
        available: content_is_available(lesson),
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

    let resp = session
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.without_url())?;
    let body = checked_json(resp, "file URL").await?;
    let body = body.get("data").unwrap_or(&body);

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
    tracing::info!("[kiwify] session saved");
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

    tracing::info!("[kiwify] session loaded");

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
#[path = "api_tests.rs"]
mod regression_tests;

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
