use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::platforms::medcel::api::{self, MedcelCourse, MedcelSession};
use crate::platforms::medcel::downloader;
use crate::state::CoursesState;

const SESSION_COOLDOWN: Duration = Duration::from_secs(5 * 60);
const COURSES_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Serialize)]
struct MedcelDownloadCompleteEvent {
    course_name: String,
    success: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn medcel_login(
    state: tauri::State<'_, CoursesState>,
    email: String,
    password: String,
    api_key: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.medcel_session.lock().await.take();
    *state.medcel_session_validated_at.lock().await = None;
    *state.medcel_courses_cache.lock().await = None;

    match api::authenticate(&email, &password, &api_key).await {
        Ok(session) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.medcel_session.lock().await;
            *guard = Some(session);
            *state.medcel_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Err(e) => {
            tracing::error!("[medcel] login failed: {}", e);
            Err(format!("Login failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn medcel_login_token(
    state: tauri::State<'_, CoursesState>,
    token: String,
    api_key: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.medcel_session.lock().await.take();
    *state.medcel_session_validated_at.lock().await = None;
    *state.medcel_courses_cache.lock().await = None;

    let parsed_token = omniget_core::core::cookie_parser::parse_bearer_input(&token);

    let client = api::build_client_with_origin(&parsed_token, &api_key, "https://areaaluno.medcel.com.br")
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let mut session = MedcelSession {
        token: parsed_token.clone(),
        api_key: api_key.clone(),
        student_id: String::new(),
        client,
    };

    match api::validate_token(&session).await {
        Ok(true) => {
            match api::fetch_student_id(&session).await {
                Ok(id) => session.student_id = id,
                Err(e) => tracing::warn!("[medcel] could not fetch student id: {}", e),
            }
            let _ = api::save_session(&session).await;
            let mut guard = state.medcel_session.lock().await;
            *guard = Some(session);
            *state.medcel_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => Err("Invalid token".to_string()),
        Err(e) => Err(format!("Token validation failed: {}", e)),
    }
}

#[tauri::command]
pub async fn medcel_check_session(
    state: tauri::State<'_, CoursesState>,
) -> Result<String, String> {
    let has_memory_session = state.medcel_session.lock().await.is_some();

    if !has_memory_session {
        match api::load_session().await {
            Ok(Some(session)) => {
                let mut guard = state.medcel_session.lock().await;
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

    let guard = state.medcel_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "not_authenticated".to_string())?;

    {
        let validated_at = state.medcel_session_validated_at.lock().await;
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
            *state.medcel_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => {
            state.medcel_session.lock().await.take();
            *state.medcel_session_validated_at.lock().await = None;
            *state.medcel_courses_cache.lock().await = None;
            let _ = api::delete_saved_session().await;
            Err("session_expired".to_string())
        }
        Err(e) => Err(format!("session_check_failed: {}", e)),
    }
}

#[tauri::command]
pub async fn medcel_logout(
    state: tauri::State<'_, CoursesState>,
) -> Result<(), String> {
    let _ = api::delete_saved_session().await;
    state.medcel_session.lock().await.take();
    *state.medcel_session_validated_at.lock().await = None;
    *state.medcel_courses_cache.lock().await = None;
    Ok(())
}

async fn fetch_medcel_courses(
    state: &tauri::State<'_, CoursesState>,
) -> Result<Vec<MedcelCourse>, String> {
    let guard = state.medcel_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    let courses = api::list_courses(session)
        .await
        .map_err(|e| e.to_string())?;

    let mut cache = state.medcel_courses_cache.lock().await;
    *cache = Some(crate::state::MedcelCoursesCache {
        courses: courses.clone(),
        fetched_at: Instant::now(),
    });

    Ok(courses)
}

#[tauri::command]
pub async fn medcel_list_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<MedcelCourse>, String> {
    {
        let cache = state.medcel_courses_cache.lock().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < COURSES_CACHE_TTL {
                return Ok(cached.courses.clone());
            }
        }
    }

    fetch_medcel_courses(&state).await
}

#[tauri::command]
pub async fn medcel_refresh_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<MedcelCourse>, String> {
    {
        let mut cache = state.medcel_courses_cache.lock().await;
        *cache = None;
    }
    fetch_medcel_courses(&state).await
}

#[tauri::command]
pub async fn start_medcel_course_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, CoursesState>,
    course_json: String,
    output_dir: String,
) -> Result<String, String> {
    let course: MedcelCourse =
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
        let guard = state.medcel_session.lock().await;
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
                    &MedcelDownloadCompleteEvent {
                        course_name: course.name,
                        success: true,
                        error: None,
                    },
                );
            }
            Err(e) => {
                tracing::error!("[medcel] download error for '{}': {}", course.name, e);
                let _ = app.emit(
                    "download-complete",
                    &MedcelDownloadCompleteEvent {
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
