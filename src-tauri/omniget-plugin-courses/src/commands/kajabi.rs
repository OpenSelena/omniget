use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::platforms::kajabi::api::{self, KajabiCourse, KajabiSession};
use crate::platforms::kajabi::downloader;
use crate::state::CoursesState;

const SESSION_COOLDOWN: Duration = Duration::from_secs(5 * 60);
const COURSES_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Serialize)]
struct KajabiDownloadCompleteEvent {
    course_name: String,
    success: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn kajabi_request_login_link(
    email: String,
) -> Result<String, String> {
    api::request_login_link(&email)
        .await
        .map_err(|e| format!("Login link request failed: {}", e))
}

#[tauri::command]
pub async fn kajabi_verify_login(
    state: tauri::State<'_, CoursesState>,
    email: String,
    confirmation_code: String,
    login_token: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.kajabi_session.lock().await.take();
    *state.kajabi_session_validated_at.lock().await = None;
    *state.kajabi_courses_cache.lock().await = None;

    let bearer_token = api::verify_login(&email, &confirmation_code, &login_token)
        .await
        .map_err(|e| format!("Verification failed: {}", e))?;

    let sites = api::list_sites(&bearer_token)
        .await
        .map_err(|e| format!("Failed to list sites: {}", e))?;

    let site_id = sites
        .first()
        .map(|s| s.id.clone())
        .unwrap_or_default();

    let session = KajabiSession {
        token: bearer_token,
        site_id,
        client: reqwest::Client::new(),
    };

    let client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent("KajabiMobileApp")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Authorization", format!("Bearer {}", session.token).parse().unwrap());
            h.insert("Kjb-App-Id", "Kajabi".parse().unwrap());
            h.insert("KJB-DP", "ANDROID".parse().unwrap());
            h.insert("KJB-SITE-ID", session.site_id.parse().unwrap());
            h.insert("Accept", "application/json".parse().unwrap());
            h
        })
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let session = KajabiSession {
        token: session.token,
        site_id: session.site_id,
        client,
    };

    let _ = api::save_session(&session).await;
    let mut guard = state.kajabi_session.lock().await;
    *guard = Some(session);
    *state.kajabi_session_validated_at.lock().await = Some(Instant::now());
    Ok("authenticated".to_string())
}

#[tauri::command]
pub async fn kajabi_login_token(
    state: tauri::State<'_, CoursesState>,
    token: String,
    site_id: String,
) -> Result<String, String> {
    let _ = api::delete_saved_session().await;
    state.kajabi_session.lock().await.take();
    *state.kajabi_session_validated_at.lock().await = None;
    *state.kajabi_courses_cache.lock().await = None;

    let parsed_token = omniget_core::core::cookie_parser::parse_bearer_input(&token);

    let client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent("KajabiMobileApp")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Authorization", format!("Bearer {}", parsed_token).parse().unwrap());
            h.insert("Kjb-App-Id", "Kajabi".parse().unwrap());
            h.insert("KJB-DP", "ANDROID".parse().unwrap());
            h.insert("KJB-SITE-ID", site_id.parse().unwrap());
            h.insert("Accept", "application/json".parse().unwrap());
            h
        })
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let session = KajabiSession {
        token: parsed_token,
        site_id,
        client,
    };

    match api::validate_token(&session).await {
        Ok(true) => {
            let _ = api::save_session(&session).await;
            let mut guard = state.kajabi_session.lock().await;
            *guard = Some(session);
            *state.kajabi_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => Err("Invalid token".to_string()),
        Err(e) => Err(format!("Token validation failed: {}", e)),
    }
}

#[tauri::command]
pub async fn kajabi_check_session(
    state: tauri::State<'_, CoursesState>,
) -> Result<String, String> {
    let has_memory_session = state.kajabi_session.lock().await.is_some();

    if !has_memory_session {
        match api::load_session().await {
            Ok(Some(session)) => {
                let mut guard = state.kajabi_session.lock().await;
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

    let guard = state.kajabi_session.lock().await;
    let _session = guard
        .as_ref()
        .ok_or_else(|| "not_authenticated".to_string())?;

    {
        let validated_at = state.kajabi_session_validated_at.lock().await;
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
            *state.kajabi_session_validated_at.lock().await = Some(Instant::now());
            Ok("authenticated".to_string())
        }
        Ok(false) => {
            state.kajabi_session.lock().await.take();
            *state.kajabi_session_validated_at.lock().await = None;
            *state.kajabi_courses_cache.lock().await = None;
            let _ = api::delete_saved_session().await;
            Err("session_expired".to_string())
        }
        Err(e) => Err(format!("session_check_failed: {}", e)),
    }
}

#[tauri::command]
pub async fn kajabi_logout(
    state: tauri::State<'_, CoursesState>,
) -> Result<(), String> {
    let _ = api::delete_saved_session().await;
    state.kajabi_session.lock().await.take();
    *state.kajabi_session_validated_at.lock().await = None;
    *state.kajabi_courses_cache.lock().await = None;
    Ok(())
}

#[tauri::command]
pub async fn kajabi_list_sites(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<api::KajabiSite>, String> {
    let guard = state.kajabi_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    api::list_sites(&session.token)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kajabi_set_site(
    state: tauri::State<'_, CoursesState>,
    site_id: String,
) -> Result<(), String> {
    let mut guard = state.kajabi_session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| "Not authenticated".to_string())?;

    let client = omniget_core::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .user_agent("KajabiMobileApp")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Authorization", format!("Bearer {}", session.token).parse().unwrap());
            h.insert("Kjb-App-Id", "Kajabi".parse().unwrap());
            h.insert("KJB-DP", "ANDROID".parse().unwrap());
            h.insert("KJB-SITE-ID", site_id.parse().unwrap());
            h.insert("Accept", "application/json".parse().unwrap());
            h
        })
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    session.site_id = site_id;
    session.client = client;
    let _ = api::save_session(session).await;
    Ok(())
}

async fn fetch_kajabi_courses(
    state: &tauri::State<'_, CoursesState>,
) -> Result<Vec<KajabiCourse>, String> {
    let guard = state.kajabi_session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_string())?;

    let courses = api::list_courses(session)
        .await
        .map_err(|e| e.to_string())?;

    let mut cache = state.kajabi_courses_cache.lock().await;
    *cache = Some(crate::state::KajabiCoursesCache {
        courses: courses.clone(),
        fetched_at: Instant::now(),
    });

    Ok(courses)
}

#[tauri::command]
pub async fn kajabi_list_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<KajabiCourse>, String> {
    {
        let cache = state.kajabi_courses_cache.lock().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < COURSES_CACHE_TTL {
                return Ok(cached.courses.clone());
            }
        }
    }

    fetch_kajabi_courses(&state).await
}

#[tauri::command]
pub async fn kajabi_refresh_courses(
    state: tauri::State<'_, CoursesState>,
) -> Result<Vec<KajabiCourse>, String> {
    {
        let mut cache = state.kajabi_courses_cache.lock().await;
        *cache = None;
    }
    fetch_kajabi_courses(&state).await
}

#[tauri::command]
pub async fn start_kajabi_course_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, CoursesState>,
    course_json: String,
    output_dir: String,
) -> Result<String, String> {
    let course: KajabiCourse =
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
        let guard = state.kajabi_session.lock().await;
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
                    &KajabiDownloadCompleteEvent {
                        course_name: course.name,
                        success: true,
                        error: None,
                    },
                );
            }
            Err(e) => {
                tracing::error!("[kajabi] download error for '{}': {}", course.name, e);
                let _ = app.emit(
                    "download-complete",
                    &KajabiDownloadCompleteEvent {
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
