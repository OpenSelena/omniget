use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::platforms::themembers::api::{self, TheMembersCourse};
use crate::platforms::themembers::downloader;
use crate::state::CoursesState;

const SESSION_COOLDOWN: Duration = Duration::from_secs(5 * 60);
const COURSES_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Serialize)]
struct TheMembersDownloadCompleteEvent {
    course_name: String,
    success: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn themembers_login(
    state: tauri::State<'_, CoursesState>,
    email: String,
    password: String,
    domain: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.themembers_session.lock().await.take();
    *state.themembers_session_validated_at.lock().await = None;
    *state.themembers_courses_cache.lock().await = None;

    match api::authenticate(&email, &password, &domain).await {
        Ok(session) => {
            let response_domain = session.domain.clone();
            let _ = api::save_session(&session).await;
            let mut guard = state.themembers_session.lock().await;
            *guard = Some(session);
            *state.themembers_session_validated_at.lock().await = Some(Instant::now());
            Ok(response_domain)
        }
        Err(e) => {
            tracing::error!("[themembers] login failed: {}", e);
            Err(format!("Login failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn themembers_login_token(
    state: tauri::State<'_, CoursesState>,
    token: String,
    domain: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.themembers_session.lock().await.take();
    *state.themembers_session_validated_at.lock().await = None;
    *state.themembers_courses_cache.lock().await = None;

    let parsed_token = omniget_core::core::cookie_parser::parse_bearer_input(&token);

    match api::authenticate_token(&parsed_token, &domain).await {
        Ok(session) => {
            let response_domain = session.domain.clone();
            let _ = api::save_session(&session).await;
            let mut guard = state.themembers_session.lock().await;
            *guard = Some(session);
            *state.themembers_session_validated_at.lock().await = Some(Instant::now());
            Ok(response_domain)
        }
        Err(e) => {
            tracing::error!("[themembers] token login failed: {}", e);
            Err(format!("Login failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn themembers_check_session(
    state: tauri::State<'_, CoursesState>,
) -> Result<String, String> {
    let has_memory_session = state.themembers_session.lock().await.is_some();

    if !has_memory_session {
        match api::load_session().await {
            Ok(Some(session)) => {
                let mut guard = state.themembers_session.lock().await;
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

    let guard = state.themembers_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "not_authenticated".to_string())?;
    let domain = session.domain.clone();

    {
        let validated_at = state.themembers_session_validated_at.lock().await;
        if let Some(at) = *validated_at {
            if at.elapsed() < SESSION_COOLDOWN {
                return Ok(domain);
            }
        }
    }

    let session_clone = session.clone();
    drop(guard);

    match api::validate_token(&session_clone).await {
        Ok(true) => {
            *state.themembers_session_validated_at.lock().await = Some(Instant::now());
            Ok(domain)
        }
        Ok(false) => {
            state.themembers_session.lock().await.take();
            *state.themembers_session_validated_at.lock().await = None;
            *state.themembers_courses_cache.lock().await = None;
            let _ = api::delete_saved_session().await;
            Err("session_expired".to_string())
        }
        Err(e) => Err(format!("session_check_failed: {}", e)),
    }
}

#[tauri::command]
pub async fn themembers_logout(
    state: tauri::State<'_, CoursesState>,
) -> Result<(), String> {
    let _ = api::delete_saved_session().await;
    state.themembers_session.lock().await.take();
    *state.themembers_session_validated_at.lock().await = None;
    *state.themembers_courses_cache.lock().await = None;
    Ok(())
}

async fn fetch_themembers_courses(
    state: &tauri::State<'_, CoursesState>,
) -> Result<Vec<TheMembersCourse>, String> {
    let guard = state.themembers_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    let courses = api::list_courses(session)
        .await
        .map_err(|e| e.to_string())?;

    let mut cache = state.themembers_courses_cache.lock().await;
    *cache = Some(crate::state::TheMembersCoursesCache {
        courses: courses.clone(),
        fetched_at: Instant::now(),
    });

    Ok(courses)
}

#[tauri::command]
pub async fn themembers_list_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<TheMembersCourse>, String> {
    {
        let cache = state.themembers_courses_cache.lock().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < COURSES_CACHE_TTL {
                return Ok(cached.courses.clone());
            }
        }
    }

    fetch_themembers_courses(&state).await
}

#[tauri::command]
pub async fn themembers_refresh_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<TheMembersCourse>, String> {
    {
        let mut cache = state.themembers_courses_cache.lock().await;
        *cache = None;
    }
    fetch_themembers_courses(&state).await
}

#[tauri::command]
pub async fn start_themembers_course_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, CoursesState>,
    course_json: String,
    output_dir: String,
) -> Result<String, String> {
    let course: TheMembersCourse =
        serde_json::from_str(&course_json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let course_name = course.name.clone();
    let course_id = course.id as u64;
    let active = state.active_downloads.clone();

    let cancel_token = CancellationToken::new();

    {
        let mut map = active.lock().await;
        if map.contains_key(&course_id) {
            return Err("Download already in progress for this course".to_string());
        }
        map.insert(course_id, cancel_token.clone());
    }

    let session = {
        let guard = state.themembers_session.lock().await;
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
            map.remove(&course_id);
        }

        match result {
            Ok(()) => {
                let _ = app.emit(
                    "download-complete",
                    &TheMembersDownloadCompleteEvent {
                        course_name: course.name,
                        success: true,
                        error: None,
                    },
                );
            }
            Err(e) => {
                tracing::error!("[themembers] download error for '{}': {}", course.name, e);
                let _ = app.emit(
                    "download-complete",
                    &TheMembersDownloadCompleteEvent {
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
