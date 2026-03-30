use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::platforms::teachable::api::{self, TeachableCourse, TeachableSession};
use crate::platforms::teachable::downloader;
use crate::state::CoursesState;

const SESSION_COOLDOWN: Duration = Duration::from_secs(5 * 60);
const COURSES_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Serialize)]
struct TeachableDownloadCompleteEvent {
    course_name: String,
    success: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn teachable_request_otp(
    email: String,
) -> Result<(), String> {
    api::request_otp(&email)
        .await
        .map_err(|e| format!("OTP request failed: {}", e))
}

#[tauri::command]
pub async fn teachable_verify_otp(
    state: tauri::State<'_, CoursesState>,
    email: String,
    otp_code: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.teachable_session.lock().await.take();
    *state.teachable_session_validated_at.lock().await = None;
    *state.teachable_courses_cache.lock().await = None;

    match api::verify_otp(&email, &otp_code).await {
        Ok(session) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.teachable_session.lock().await;
            *guard = Some(session);
            *state.teachable_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Err(e) => {
            tracing::error!("[teachable] OTP verify failed: {}", e);
            Err(format!("OTP verification failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn teachable_login_token(
    state: tauri::State<'_, CoursesState>,
    token: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.teachable_session.lock().await.take();
    *state.teachable_session_validated_at.lock().await = None;
    *state.teachable_courses_cache.lock().await = None;

    let parsed_token = omniget_core::core::cookie_parser::parse_bearer_input(&token);

    let client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent("ktor-client")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Authorization", format!("Bearer {}", parsed_token).parse().unwrap());
            h.insert("CLIENT-TOKEN", "9e44e885ac601aae4ee7109baec9ee0a503bfbb4fd11cbcb7d1de9d5e84f395b37d1521b08add19c2604dbe3c1d6c986bbd62a2513884e04e5b40704e77944e4".parse().unwrap());
            h.insert("X-APP-VERSION", "2.3.0".parse().unwrap());
            h.insert("X-DEVICE-OS", "Android 35".parse().unwrap());
            h.insert("Accept", "application/json".parse().unwrap());
            h
        })
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let session = TeachableSession {
        token: parsed_token,
        school_id: None,
        client,
    };

    match api::validate_token(&session).await {
        Ok(true) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.teachable_session.lock().await;
            *guard = Some(session);
            *state.teachable_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => Err("Invalid token".to_string()),
        Err(e) => Err(format!("Token validation failed: {}", e)),
    }
}

#[tauri::command]
pub async fn teachable_check_session(
    state: tauri::State<'_, CoursesState>,
) -> Result<String, String> {
    let has_memory_session = state.teachable_session.lock().await.is_some();

    if !has_memory_session {
        match api::load_session().await {
            Ok(Some(session)) => {
                let mut guard = state.teachable_session.lock().await;
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

    let guard = state.teachable_session.lock().await;
    let _session = guard
        .as_ref()
        .ok_or_else(|| "not_authenticated".to_string())?;

    {
        let validated_at = state.teachable_session_validated_at.lock().await;
        if let Some(at) = *validated_at {
            if at.elapsed() < SESSION_COOLDOWN {
                return Ok("authenticated".to_string());
            }
        }
    }

    let session_clone = guard.as_ref().unwrap().clone();
    drop(guard);

    match api::validate_token(&session_clone).await {
        Ok(true) => {
            *state.teachable_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => {
            state.teachable_session.lock().await.take();
            *state.teachable_session_validated_at.lock().await = None;
            *state.teachable_courses_cache.lock().await = None;
            let _ = api::delete_saved_session().await;
            Err("session_expired".to_string())
        }
        Err(e) => Err(format!("session_check_failed: {}", e)),
    }
}

#[tauri::command]
pub async fn teachable_logout(
    state: tauri::State<'_, CoursesState>,
) -> Result<(), String> {
    let _ = api::delete_saved_session().await;
    state.teachable_session.lock().await.take();
    *state.teachable_session_validated_at.lock().await = None;
    *state.teachable_courses_cache.lock().await = None;
    Ok(())
}

#[tauri::command]
pub async fn teachable_set_school(
    state: tauri::State<'_, CoursesState>,
    school_id: String,
) -> Result<(), String> {
    let mut guard = state.teachable_session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| "Not authenticated".to_string())?;
    session.school_id = Some(school_id);
    let _ = api::save_session(session).await;
    Ok(())
}

#[tauri::command]
pub async fn teachable_list_schools(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<api::TeachableSchool>, String> {
    let guard = state.teachable_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    api::list_schools(session)
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_teachable_courses(
    state: &tauri::State<'_, CoursesState>,
) -> Result<Vec<TeachableCourse>, String> {
    let guard = state.teachable_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    let school_id = session
        .school_id
        .as_ref()
        .ok_or_else(|| "No school selected. Please select a school first.".to_string())?;

    let courses = api::list_courses(session, school_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut cache = state.teachable_courses_cache.lock().await;
    *cache = Some(crate::state::TeachableCoursesCache {
        courses: courses.clone(),
        fetched_at: Instant::now(),
    });

    Ok(courses)
}

#[tauri::command]
pub async fn teachable_list_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<TeachableCourse>, String> {
    {
        let cache = state.teachable_courses_cache.lock().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < COURSES_CACHE_TTL {
                return Ok(cached.courses.clone());
            }
        }
    }

    fetch_teachable_courses(&state).await
}

#[tauri::command]
pub async fn teachable_refresh_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<TeachableCourse>, String> {
    {
        let mut cache = state.teachable_courses_cache.lock().await;
        *cache = None;
    }
    fetch_teachable_courses(&state).await
}

#[tauri::command]
pub async fn start_teachable_course_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, CoursesState>,
    course_json: String,
    output_dir: String,
) -> Result<String, String> {
    let course: TeachableCourse =
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
        let guard = state.teachable_session.lock().await;
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
                    &TeachableDownloadCompleteEvent {
                        course_name: course.name,
                        success: true,
                        error: None,
                    },
                );
            }
            Err(e) => {
                tracing::error!("[teachable] download error for '{}': {}", course.name, e);
                let _ = app.emit(
                    "download-complete",
                    &TeachableDownloadCompleteEvent {
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
