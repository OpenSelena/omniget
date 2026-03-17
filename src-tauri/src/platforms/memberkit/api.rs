use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

#[derive(Clone)]
pub struct MemberkitSession {
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
pub struct MemberkitCourse {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberkitModule {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub lessons: Vec<MemberkitLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberkitLesson {
    pub id: String,
    pub name: String,
    pub url: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberkitLessonDetail {
    pub id: String,
    pub name: String,
    pub video_url: Option<String>,
    pub attachments: Vec<MemberkitAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberkitAttachment {
    pub name: String,
    pub url: String,
}

fn build_client(cookies: &str, site_url: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!("_memberkit_session={}", cookies))?,
    );
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("{}/", site_url))?,
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
    Ok(data_dir.join("omniget").join("memberkit_session.json"))
}

pub async fn authenticate(
    email: &str,
    password: &str,
    site_url: &str,
) -> anyhow::Result<MemberkitSession> {
    let temp_client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?;

    let login_page_url = format!("{}/users/sign_in", site_url.trim_end_matches('/'));
    let resp = temp_client.get(&login_page_url).send().await?;
    let login_html = resp.text().await?;

    let csrf_token = extract_meta_csrf(&login_html)
        .or_else(|| extract_input_value(&login_html, "authenticity_token"))
        .ok_or_else(|| anyhow!("Could not find CSRF token on login page"))?;

    let form_url = format!("{}/users/sign_in", site_url.trim_end_matches('/'));
    let resp = temp_client
        .post(&form_url)
        .form(&[
            ("authenticity_token", csrf_token.as_str()),
            ("user[email]", email),
            ("user[password]", password),
        ])
        .send()
        .await?;

    let status = resp.status();
    let resp_url = resp.url().to_string();
    let _body = resp.text().await?;

    if resp_url.contains("/sign_in") {
        if !status.is_success() && !status.is_redirection() {
            return Err(anyhow!("Authentication failed (status {})", status));
        }
        return Err(anyhow!("Authentication failed - invalid credentials"));
    }

    let root_resp = temp_client
        .get(site_url.trim_end_matches('/'))
        .send()
        .await?;

    if !root_resp.status().is_success() {
        return Err(anyhow!("Authentication failed - cannot access member area"));
    }

    let cookies_header = root_resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| {
            if s.contains("_memberkit_session=") {
                let start = s.find("_memberkit_session=")? + 19;
                let end = s[start..].find(';').map(|e| e + start).unwrap_or(s.len());
                Some(s[start..end].to_string())
            } else {
                None
            }
        })
        .last();

    let session_cookie = cookies_header
        .ok_or_else(|| anyhow!("Could not extract session cookie after login"))?;

    let client = build_client(&session_cookie, site_url)?;

    Ok(MemberkitSession {
        cookies: session_cookie,
        site_url: site_url.trim_end_matches('/').to_string(),
        client,
    })
}

pub async fn validate_token(session: &MemberkitSession) -> anyhow::Result<bool> {
    let resp = session.client.get(&session.site_url).send().await?;

    let url = resp.url().to_string();
    if url.contains("/sign_in") || url.contains("/login") {
        return Ok(false);
    }

    Ok(resp.status().is_success())
}

pub async fn list_courses(session: &MemberkitSession) -> anyhow::Result<Vec<MemberkitCourse>> {
    let resp = session.client.get(&session.site_url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_courses returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let mut courses = Vec::new();

    let card_re = regex::Regex::new(
        r#"(?s)data-id="(\d+)"[^>]*>.*?<a[^>]+class="[^"]*text-base[^"]*"[^>]+href="([^"]+)"[^>]*>([^<]+)"#
    )?;

    for cap in card_re.captures_iter(&body_text) {
        let id = cap[1].to_string();
        let href = cap[2].to_string();
        let name = cap[3].trim().to_string();

        let slug = href
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();

        if courses.iter().any(|c: &MemberkitCourse| c.id == id) {
            continue;
        }

        courses.push(MemberkitCourse { id, name, slug });
    }

    if courses.is_empty() {
        let link_re = regex::Regex::new(
            r#"<a[^>]+href="([^"]*(/courses?/[^"]+))"[^>]*>([^<]+)"#
        )?;

        for cap in link_re.captures_iter(&body_text) {
            let href = cap[1].to_string();
            let name = cap[3].trim().to_string();

            let slug = href
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();

            let id = slug.clone();

            if courses.iter().any(|c: &MemberkitCourse| c.slug == slug) {
                continue;
            }

            courses.push(MemberkitCourse { id, name, slug });
        }
    }

    Ok(courses)
}

pub async fn get_modules(
    session: &MemberkitSession,
    course_slug: &str,
) -> anyhow::Result<Vec<MemberkitModule>> {
    let url = format!("{}/courses/{}", session.site_url, course_slug);
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

    let mut modules = Vec::new();

    let section_re = regex::Regex::new(
        r#"(?s)<div[^>]+id="([^"]+)"[^>]*class="[^"]*section[^"]*"[^>]*>.*?<span[^>]*>([^<]+)</span>"#
    )?;

    let lesson_re = regex::Regex::new(
        r#"(?s)<li[^>]+data-id="(\d+)"[^>]*>.*?<a[^>]+class="[^"]*lesson__title[^"]*"[^>]+href="([^"]+)"[^>]*>([^<]+)"#
    )?;

    let sections: Vec<(String, String, usize)> = section_re
        .captures_iter(&body_text)
        .enumerate()
        .map(|(i, cap)| (cap[1].to_string(), cap[2].trim().to_string(), i))
        .collect();

    if sections.is_empty() {
        let mut lessons = Vec::new();
        for (i, cap) in lesson_re.captures_iter(&body_text).enumerate() {
            let lesson_id = cap[1].to_string();
            let lesson_url = cap[2].to_string();
            let lesson_name = cap[3].trim().to_string();

            if lessons.iter().any(|l: &MemberkitLesson| l.id == lesson_id) {
                continue;
            }

            let full_url = if lesson_url.starts_with("http") {
                lesson_url
            } else {
                format!("{}{}", session.site_url, lesson_url)
            };

            lessons.push(MemberkitLesson {
                id: lesson_id,
                name: lesson_name,
                url: full_url,
                order: i as i64,
            });
        }

        if !lessons.is_empty() {
            modules.push(MemberkitModule {
                id: "0".to_string(),
                name: "Content".to_string(),
                order: 0,
                lessons,
            });
        }
    } else {
        for (section_id, section_name, idx) in &sections {
            let section_start = body_text
                .find(&format!("id=\"{}\"", section_id))
                .unwrap_or(0);

            let section_end = if idx + 1 < sections.len() {
                body_text
                    .find(&format!("id=\"{}\"", sections[idx + 1].0))
                    .unwrap_or(body_text.len())
            } else {
                body_text.len()
            };

            let section_html = &body_text[section_start..section_end];

            let mut lessons = Vec::new();
            for (i, cap) in lesson_re.captures_iter(section_html).enumerate() {
                let lesson_id = cap[1].to_string();
                let lesson_url = cap[2].to_string();
                let lesson_name = cap[3].trim().to_string();

                if lessons.iter().any(|l: &MemberkitLesson| l.id == lesson_id) {
                    continue;
                }

                let full_url = if lesson_url.starts_with("http") {
                    lesson_url
                } else {
                    format!("{}{}", session.site_url, lesson_url)
                };

                lessons.push(MemberkitLesson {
                    id: lesson_id,
                    name: lesson_name,
                    url: full_url,
                    order: i as i64,
                });
            }

            modules.push(MemberkitModule {
                id: section_id.clone(),
                name: section_name.clone(),
                order: *idx as i64,
                lessons,
            });
        }
    }

    Ok(modules)
}

pub async fn get_lesson_detail(
    session: &MemberkitSession,
    lesson_url: &str,
) -> anyhow::Result<MemberkitLessonDetail> {
    let resp = session.client.get(lesson_url).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_lesson_detail returned status {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        ));
    }

    let title_re = regex::Regex::new(r#"<title>([^<]+)</title>"#)?;
    let name = title_re
        .captures(&body_text)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let id = lesson_url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();

    let mut video_url: Option<String> = None;

    let yt_re = regex::Regex::new(r#"data-youtube-uid-value="([^"]+)""#)?;
    if let Some(cap) = yt_re.captures(&body_text) {
        video_url = Some(format!("https://www.youtube.com/watch?v={}", &cap[1]));
    }

    if video_url.is_none() {
        let vimeo_re = regex::Regex::new(r#"data-vimeo-uid-value="([^"]+)""#)?;
        if let Some(cap) = vimeo_re.captures(&body_text) {
            video_url = Some(format!("https://vimeo.com/{}", &cap[1]));
        }
    }

    if video_url.is_none() {
        let panda_re = regex::Regex::new(r#"data-panda-url-value="([^"]+)""#)?;
        if let Some(cap) = panda_re.captures(&body_text) {
            video_url = Some(cap[1].to_string());
        }
    }

    if video_url.is_none() {
        let iframe_re = regex::Regex::new(r#"<iframe[^>]+src="([^"]+)"[^>]*>"#)?;
        for cap in iframe_re.captures_iter(&body_text) {
            let src = &cap[1];
            if src.contains("scaleup")
                || src.contains("youtube")
                || src.contains("vimeo")
                || src.contains("pandavideo")
                || src.contains("player")
            {
                video_url = Some(src.to_string());
                break;
            }
        }
    }

    if video_url.is_none() {
        let video_re = regex::Regex::new(r#"<video[^>]+src="([^"]+)""#)?;
        if let Some(cap) = video_re.captures(&body_text) {
            video_url = Some(cap[1].to_string());
        }
    }

    let mut attachments = Vec::new();

    let dl_re = regex::Regex::new(
        r#"<a[^>]+href="([^"]*downloads/[^"]+)"[^>]*>([^<]+)"#
    )?;

    for cap in dl_re.captures_iter(&body_text) {
        let att_url = cap[1].to_string();
        let att_name = cap[2].trim().to_string();

        let full_url = if att_url.starts_with("http") {
            att_url
        } else {
            format!("{}{}", session.site_url, att_url)
        };

        attachments.push(MemberkitAttachment {
            name: att_name,
            url: full_url,
        });
    }

    Ok(MemberkitLessonDetail {
        id,
        name,
        video_url,
        attachments,
    })
}

fn extract_meta_csrf(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<meta\s+name="csrf-token"\s+content="([^"]+)""#).ok()?;
    re.captures(html).map(|c| c[1].to_string())
}

fn extract_input_value(html: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"name="{}"\s+value="([^"]*)""#, regex::escape(name));
    let re = regex::Regex::new(&pattern).ok()?;
    if let Some(cap) = re.captures(html) {
        return Some(cap[1].to_string());
    }

    let pattern2 = format!(r#"value="([^"]*)"\s+name="{}""#, regex::escape(name));
    let re2 = regex::Regex::new(&pattern2).ok()?;
    if let Some(cap) = re2.captures(html) {
        return Some(cap[1].to_string());
    }

    None
}

pub async fn save_session(session: &MemberkitSession) -> anyhow::Result<()> {
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
    tracing::info!("[memberkit] session saved for {}", session.site_url);
    Ok(())
}

pub async fn load_session() -> anyhow::Result<Option<MemberkitSession>> {
    let path = session_file_path()?;
    let json = match tokio::fs::read_to_string(&path).await {
        Ok(j) => j,
        Err(_) => return Ok(None),
    };

    let saved: SavedSession = serde_json::from_str(&json)?;
    let client = build_client(&saved.cookies, &saved.site_url)?;

    tracing::info!("[memberkit] session loaded for {}", saved.site_url);

    Ok(Some(MemberkitSession {
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
