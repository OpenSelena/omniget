use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::platforms::astronmembers::api::{self, AstronCourse, AstronSession};
use crate::platforms::astronmembers::downloader;
use crate::state::CoursesState;

const SESSION_COOLDOWN: Duration = Duration::from_secs(5 * 60);
const COURSES_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Serialize)]
struct AstronDownloadCompleteEvent {
    course_name: String,
    success: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn astron_login(
    state: tauri::State<'_, CoursesState>,
    site_url: String,
    login: String,
    password: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.astron_session.lock().await.take();
    *state.astron_session_validated_at.lock().await = None;
    *state.astron_courses_cache.lock().await = None;

    let clean_url = site_url.trim_end_matches('/').to_string();

    match api::authenticate(&clean_url, &login, &password).await {
        Ok(session) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.astron_session.lock().await;
            *guard = Some(session);
            *state.astron_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Err(e) => {
            tracing::error!("[astron] login failed: {}", e);
            Err(format!("Login failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn astron_login_token(
    state: tauri::State<'_, CoursesState>,
    cookies: String,
    site_url: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.astron_session.lock().await.take();
    *state.astron_session_validated_at.lock().await = None;
    *state.astron_courses_cache.lock().await = None;

    let clean_url = site_url.trim_end_matches('/').to_string();
    let parsed = omniget_core::core::cookie_parser::parse_cookie_input(&cookies, "session");

    let session = AstronSession {
        cookies: parsed.cookie_string.clone(),
        site_url: clean_url.clone(),
        client: omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
            .user_agent("Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0")
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert("Cookie", parsed.cookie_string.parse().unwrap());
                h.insert("Referer", format!("{}/", clean_url).parse().unwrap());
                h.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".parse().unwrap());
                h
            })
            .redirect(reqwest::redirect::Policy::limited(10))
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?,
    };

    match api::validate_token(&session).await {
        Ok(true) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.astron_session.lock().await;
            *guard = Some(session);
            *state.astron_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => Err("Invalid cookies".to_string()),
        Err(e) => Err(format!("Validation failed: {}", e)),
    }
}

#[tauri::command]
pub async fn astron_check_session(
    state: tauri::State<'_, CoursesState>,
) -> Result<String, String> {
    let has_memory_session = state.astron_session.lock().await.is_some();

    if !has_memory_session {
        match api::load_session().await {
            Ok(Some(session)) => {
                let mut guard = state.astron_session.lock().await;
                *guard = Some(session);
            }
            Ok(None) => {
                return Err("not_authenticated".to_string());
            }
            Err(_) => {
                return Err("not_authenticated".to_string());
            }
        }
    }

    let guard = state.astron_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "not_authenticated".to_string())?;

    {
        let validated_at = state.astron_session_validated_at.lock().await;
        if let Some(at) = *validated_at {
            if at.elapsed() < SESSION_COOLDOWN {
                return Ok("authenticated".to_string());
            }
        }
    }

    let session_clone = session.clone();
    drop(guard);

    match api::validate_token(&session_clone).await {
        Ok(true) => {
            *state.astron_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => {
            state.astron_session.lock().await.take();
            *state.astron_session_validated_at.lock().await = None;
            *state.astron_courses_cache.lock().await = None;
            let _ = api::delete_saved_session().await;
            Err("session_expired".to_string())
        }
        Err(e) => Err(format!("session_check_failed: {}", e)),
    }
}

#[tauri::command]
pub async fn astron_logout(
    state: tauri::State<'_, CoursesState>,
) -> Result<(), String> {
    let _ = api::delete_saved_session().await;
    state.astron_session.lock().await.take();
    *state.astron_session_validated_at.lock().await = None;
    *state.astron_courses_cache.lock().await = None;
    Ok(())
}

async fn fetch_astron_courses(
    state: &tauri::State<'_, CoursesState>,
) -> Result<Vec<AstronCourse>, String> {
    let guard = state.astron_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    let courses = api::list_courses(session)
        .await
        .map_err(|e| e.to_string())?;

    let mut cache = state.astron_courses_cache.lock().await;
    *cache = Some(crate::state::AstronCoursesCache {
        courses: courses.clone(),
        fetched_at: Instant::now(),
    });

    Ok(courses)
}

#[tauri::command]
pub async fn astron_list_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<AstronCourse>, String> {
    {
        let cache = state.astron_courses_cache.lock().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < COURSES_CACHE_TTL {
                return Ok(cached.courses.clone());
            }
        }
    }

    fetch_astron_courses(&state).await
}

#[tauri::command]
pub async fn astron_refresh_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<AstronCourse>, String> {
    {
        let mut cache = state.astron_courses_cache.lock().await;
        *cache = None;
    }
    fetch_astron_courses(&state).await
}

#[tauri::command]
pub async fn start_astron_course_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, CoursesState>,
    course_json: String,
    output_dir: String,
) -> Result<String, String> {
    let course: AstronCourse =
        serde_json::from_str(&course_json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let course_name = course.name.clone();
    let course_id_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        course.id.hash(&mut hasher);
        hasher.finish()
    };
    let active = state.active_downloads.clone();

    let cancel_token = CancellationToken::new();

    {
        let mut map = active.lock().await;
        if map.contains_key(&course_id_hash) {
            return Err("Download already in progress for this course".to_string());
        }
        map.insert(course_id_hash, cancel_token.clone());
    }

    let session = {
        let guard = state.astron_session.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?
            .clone()
    };

    tokio::spawn(async move {
        let result =
            downloader::download_full_course(&app, &session, &course, &output_dir, cancel_token)
                .await;

        {
            let mut map = active.lock().await;
            map.remove(&course_id_hash);
        }

        match result {
            Ok(()) => {
                let _ = app.emit(
                    "download-complete",
                    &AstronDownloadCompleteEvent {
                        course_name: course.name,
                        success: true,
                        error: None,
                    },
                );
            }
            Err(e) => {
                tracing::error!("[astron] download error for '{}': {}", course.name, e);
                let _ = app.emit(
                    "download-complete",
                    &AstronDownloadCompleteEvent {
                        course_name: course.name,
                        success: false,
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(format!("Download started: {}", course_name))
}
