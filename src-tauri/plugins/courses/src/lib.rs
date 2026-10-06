#![allow(unused_imports)]

pub mod catalog;
pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

pub mod models {
    pub use omniget_core::models::*;
}

use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};
use state::CoursesState;
use std::sync::Arc;

#[inline]
fn wrap_state<'a>(state: &'a CoursesState) -> tauri::State<'a, CoursesState> {
    unsafe { std::mem::transmute(state) }
}

fn extract_payload(args: &serde_json::Value) -> Result<String, String> {
    if let Some(v) = args
        .get("course_json")
        .or(args.get("courseJson"))
        .or(args.get("product_json"))
        .or(args.get("productJson"))
        .or(args.get("course"))
        .or(args.get("product"))
    {
        if let Some(s) = v.as_str() {
            Ok(s.to_string())
        } else {
            serde_json::to_string(v).map_err(|e| e.to_string())
        }
    } else {
        Err("missing 'course_json'".to_string())
    }
}

fn extract_output_dir(args: &serde_json::Value) -> String {
    args.get("output_dir")
        .or(args.get("outputDir"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub struct CoursesPlugin {
    host: Option<Arc<dyn PluginHost>>,
    state: Arc<CoursesState>,
    pub app: Option<tauri::AppHandle>,
}

impl CoursesPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            state: Arc::new(CoursesState::default()),
            app: None,
        }
    }

    pub fn with_app(mut self, app: tauri::AppHandle) -> Self {
        self.app = Some(app);
        self
    }

    pub fn supported_commands(&self) -> Vec<String> {
        self.commands()
    }
}

pub fn manifest() -> omniget_plugin_sdk::PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid courses manifest")
}

impl OmnigetPlugin for CoursesPlugin {
    fn id(&self) -> &str {
        "courses"
    }

    fn name(&self) -> &str {
        "Course Downloader"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    > {
        let _host = self.host.clone();
        let state = self.state.clone();
        let app = self.app.clone();
        Box::pin(async move {
            macro_rules! dispatch_download {
                ($cmd_fn:expr) => {{
                    let course_json = extract_payload(&args)?;
                    let output_dir = extract_output_dir(&args);
                    let app_handle = app
                        .ok_or_else(|| "Tauri AppHandle not available on CoursesPlugin".to_string())?;
                    let res = $cmd_fn(app_handle, wrap_state(&state), course_json, output_dir).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }};
            }

            match command.as_str() {
                "get_platforms" => {
                    let res = catalog::all_platforms();
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "get_platform_config" => {
                    let platform_id: String = serde_json::from_value(
                        args.get("platform")
                            .cloned()
                            .ok_or_else(|| "missing 'platform'".to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    let cfg = catalog::get_platform_config(&platform_id)
                        .ok_or_else(|| format!("Platform '{}' not found", platform_id))?;
                    serde_json::to_value(cfg).map_err(|e| e.to_string())
                }
                "hotmart_check_session" => {
                    let res = commands::auth::hotmart_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "hotmart_logout" => {
                    commands::auth::hotmart_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "hotmart_list_courses" => {
                    let res = commands::courses::hotmart_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "hotmart_refresh_courses" => {
                    let res =
                        commands::courses::hotmart_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "hotmart_get_modules" => {
                    let course_id: u64 = serde_json::from_value(
                        args.get("course_id")
                            .or(args.get("courseId"))
                            .cloned()
                            .ok_or_else(|| "missing 'course_id'".to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    let slug: String = serde_json::from_value(
                        args.get("slug")
                            .cloned()
                            .unwrap_or_else(|| serde_json::Value::String(String::new())),
                    )
                    .unwrap_or_default();
                    let res =
                        commands::courses::hotmart_get_modules(wrap_state(&state), course_id, slug)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cancel_course_download" => {
                    let course_id: u64 = serde_json::from_value(
                        args.get("course_id")
                            .or(args.get("courseId"))
                            .cloned()
                            .ok_or_else(|| "missing 'course_id'".to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::downloads::cancel_course_download(wrap_state(&state), course_id)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "get_active_downloads" => {
                    let res = commands::downloads::get_active_downloads(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "udemy_check_session" => {
                    let res = commands::udemy_auth::udemy_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "udemy_logout" => {
                    commands::udemy_auth::udemy_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "udemy_get_portal" => {
                    let res = commands::udemy_auth::udemy_get_portal(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cancel_udemy_course_download" => {
                    let course_id: u64 = serde_json::from_value(
                        args.get("course_id")
                            .or(args.get("courseId"))
                            .cloned()
                            .ok_or_else(|| "missing 'course_id'".to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    commands::udemy_downloads::cancel_udemy_course_download(
                        wrap_state(&state),
                        course_id,
                    )
                    .await?;
                    Ok(serde_json::Value::Null)
                }
                "udemy_login_cookies" => {
                    let cookie_json: String = if let Some(v) = args
                        .get("cookie_json")
                        .or(args.get("cookieJson"))
                        .or(args.get("cookies"))
                    {
                        if let Some(s) = v.as_str() {
                            s.to_string()
                        } else {
                            v.to_string()
                        }
                    } else {
                        return Err("missing 'cookie_json'".to_string());
                    };
                    let res =
                        commands::udemy_auth::udemy_login_cookies(wrap_state(&state), cookie_json)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "thinkific_login" => {
                    let cookies: String = serde_json::from_value(
                        args.get("cookies").cloned().ok_or("missing 'cookies'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let site_url: String = serde_json::from_value(
                        args.get("site_url")
                            .or(args.get("url"))
                            .cloned()
                            .ok_or("missing 'site_url'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::thinkific::thinkific_login(wrap_state(&state), cookies, site_url)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "pluralsight_login_cookies" => {
                    let cookies: String = serde_json::from_value(
                        args.get("cookies").cloned().ok_or("missing 'cookies'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::pluralsight::pluralsight_login_cookies(
                        wrap_state(&state),
                        cookies,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "masterclass_login_cookies" => {
                    let cookies: String = serde_json::from_value(
                        args.get("cookies").cloned().ok_or("missing 'cookies'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::masterclass::masterclass_login_cookies(
                        wrap_state(&state),
                        cookies,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "grancursos_login_cookies" => {
                    let cookies: String = serde_json::from_value(
                        args.get("cookies").cloned().ok_or("missing 'cookies'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::grancursos::grancursos_login_cookies(wrap_state(&state), cookies)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "rocketseat_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::rocketseat::rocketseat_login_token(wrap_state(&state), token)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::teachable::teachable_login_token(wrap_state(&state), token)
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let site_id: Option<String> = args
                        .get("site_id")
                        .or(args.get("siteId"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let res =
                        commands::kajabi::kajabi_login_token(wrap_state(&state), token, site_id)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "skool_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or("missing 'email'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or("missing 'password'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::skool::skool_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "skool_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::skool::skool_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kiwify_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or("missing 'email'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or("missing 'password'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::kiwify::kiwify_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kiwify_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::kiwify::kiwify_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "gumroad_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or("missing 'email'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or("missing 'password'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::gumroad::gumroad_login(wrap_state(&state), email, password)
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "gumroad_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::gumroad::gumroad_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "greenn_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::greenn::greenn_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "caktomembers_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::caktomembers::caktomembers_login_token(wrap_state(&state), token)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "dsa_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::dsa::dsa_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "entregadigital_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let site_url: String = args
                        .get("site_url")
                        .or_else(|| args.get("siteUrl"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let app_version: String = args
                        .get("app_version")
                        .or_else(|| args.get("appVersion"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let device_id: String = args
                        .get("device_id")
                        .or_else(|| args.get("deviceId"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let os_value: String = args
                        .get("os_value")
                        .or_else(|| args.get("osValue"))
                        .or_else(|| args.get("os"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let res = commands::entregadigital::entregadigital_login_token(
                        wrap_state(&state),
                        token,
                        site_url,
                        app_version,
                        device_id,
                        os_value,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_concursos_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::estrategia_concursos::estrategia_concursos_login_token(
                        wrap_state(&state),
                        token,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_ldi_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::estrategia_ldi::estrategia_ldi_login_token(
                        wrap_state(&state),
                        token,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_militares_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::estrategia_militares::estrategia_militares_login_token(
                        wrap_state(&state),
                        token,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcof_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::medcof::medcof_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medway_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res =
                        commands::medway::medway_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "nutror_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let refresh_token: Option<String> = args
                        .get("refresh_token")
                        .or_else(|| args.get("refreshToken"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let res = commands::nutror::nutror_login_token(
                        wrap_state(&state),
                        token,
                        refresh_token,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "voomp_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::voomp::voomp_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "areademembros_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token")
                            .or_else(|| args.get("cookies"))
                            .cloned()
                            .ok_or("missing 'token'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let site_url: String = args
                        .get("site_url")
                        .or_else(|| args.get("siteUrl"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let res = commands::areademembros::areademembros_login_token(
                        wrap_state(&state),
                        token,
                        site_url,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kiwify_check_session" => {
                    let res = commands::kiwify::kiwify_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kiwify_logout" => {
                    commands::kiwify::kiwify_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "kiwify_list_courses" => {
                    let res = commands::kiwify::kiwify_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kiwify_refresh_courses" => {
                    let res = commands::kiwify::kiwify_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "skool_check_session" => {
                    let res = commands::skool::skool_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "skool_logout" => {
                    commands::skool::skool_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "skool_list_groups" => {
                    let res = commands::skool::skool_list_groups(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "skool_refresh_groups" => {
                    let res = commands::skool::skool_refresh_groups(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "gumroad_check_session" => {
                    let res = commands::gumroad::gumroad_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "gumroad_logout" => {
                    commands::gumroad::gumroad_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "gumroad_list_products" => {
                    let res = commands::gumroad::gumroad_list_products(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "gumroad_refresh_products" => {
                    let res =
                        commands::gumroad::gumroad_refresh_products(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "rocketseat_check_session" => {
                    let res =
                        commands::rocketseat::rocketseat_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "rocketseat_logout" => {
                    commands::rocketseat::rocketseat_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "rocketseat_list_courses" => {
                    let res =
                        commands::rocketseat::rocketseat_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "rocketseat_refresh_courses" => {
                    let res = commands::rocketseat::rocketseat_refresh_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_check_session" => {
                    let res =
                        commands::teachable::teachable_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_logout" => {
                    commands::teachable::teachable_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "teachable_list_courses" => {
                    let res =
                        commands::teachable::teachable_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_refresh_courses" => {
                    let res =
                        commands::teachable::teachable_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_check_session" => {
                    let res = commands::kajabi::kajabi_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_logout" => {
                    commands::kajabi::kajabi_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "kajabi_list_courses" => {
                    let res = commands::kajabi::kajabi_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_refresh_courses" => {
                    let res = commands::kajabi::kajabi_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "thinkific_check_session" => {
                    let res =
                        commands::thinkific::thinkific_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "thinkific_logout" => {
                    commands::thinkific::thinkific_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "thinkific_list_courses" => {
                    let res =
                        commands::thinkific::thinkific_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "thinkific_refresh_courses" => {
                    let res =
                        commands::thinkific::thinkific_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "pluralsight_check_session" => {
                    let res = commands::pluralsight::pluralsight_check_session(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "pluralsight_logout" => {
                    commands::pluralsight::pluralsight_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "pluralsight_list_courses" => {
                    let res =
                        commands::pluralsight::pluralsight_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "pluralsight_refresh_courses" => {
                    let res =
                        commands::pluralsight::pluralsight_refresh_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "wondrium_check_session" => {
                    let res =
                        commands::greatcourses::wondrium_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "wondrium_logout" => {
                    commands::greatcourses::wondrium_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "wondrium_list_courses" => {
                    let res =
                        commands::greatcourses::wondrium_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "wondrium_refresh_courses" => {
                    let res = commands::greatcourses::wondrium_refresh_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "masterclass_check_session" => {
                    let res = commands::masterclass::masterclass_check_session(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "masterclass_logout" => {
                    commands::masterclass::masterclass_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "masterclass_list_courses" => {
                    let res =
                        commands::masterclass::masterclass_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "masterclass_refresh_courses" => {
                    let res =
                        commands::masterclass::masterclass_refresh_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "greenn_check_session" => {
                    let res = commands::greenn::greenn_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "greenn_logout" => {
                    commands::greenn::greenn_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "greenn_list_courses" => {
                    let res = commands::greenn::greenn_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "greenn_refresh_courses" => {
                    let res = commands::greenn::greenn_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kirvano_check_session" => {
                    let res = commands::kirvano::kirvano_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kirvano_logout" => {
                    commands::kirvano::kirvano_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "kirvano_list_courses" => {
                    let res = commands::kirvano::kirvano_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kirvano_refresh_courses" => {
                    let res =
                        commands::kirvano::kirvano_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cademi_check_session" => {
                    let res =
                        commands::cademi_cmd::cademi_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cademi_logout" => {
                    commands::cademi_cmd::cademi_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "cademi_list_courses" => {
                    let res = commands::cademi_cmd::cademi_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cademi_refresh_courses" => {
                    let res =
                        commands::cademi_cmd::cademi_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "memberkit_check_session" => {
                    let res = commands::memberkit_cmd::memberkit_check_session(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "memberkit_logout" => {
                    commands::memberkit_cmd::memberkit_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "memberkit_list_courses" => {
                    let res =
                        commands::memberkit_cmd::memberkit_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "memberkit_refresh_courses" => {
                    let res =
                        commands::memberkit_cmd::memberkit_refresh_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cakto_check_session" => {
                    let res = commands::cakto::cakto_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cakto_logout" => {
                    commands::cakto::cakto_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "cakto_list_courses" => {
                    let res = commands::cakto::cakto_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cakto_refresh_courses" => {
                    let res = commands::cakto::cakto_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "caktomembers_check_session" => {
                    let res =
                        commands::caktomembers::caktomembers_check_session(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "caktomembers_logout" => {
                    commands::caktomembers::caktomembers_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "caktomembers_list_courses" => {
                    let res = commands::caktomembers::caktomembers_list_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "caktomembers_refresh_courses" => {
                    let res =
                        commands::caktomembers::caktomembers_refresh_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "curseduca_check_session" => {
                    let res =
                        commands::curseduca::curseduca_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "curseduca_logout" => {
                    commands::curseduca::curseduca_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "curseduca_list_courses" => {
                    let res =
                        commands::curseduca::curseduca_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "curseduca_refresh_courses" => {
                    let res =
                        commands::curseduca::curseduca_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "dsa_check_session" => {
                    let res = commands::dsa::dsa_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "dsa_logout" => {
                    commands::dsa::dsa_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "dsa_list_courses" => {
                    let res = commands::dsa::dsa_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "dsa_refresh_courses" => {
                    let res = commands::dsa::dsa_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "entregadigital_check_session" => {
                    let res =
                        commands::entregadigital::entregadigital_check_session(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "entregadigital_logout" => {
                    commands::entregadigital::entregadigital_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "entregadigital_list_courses" => {
                    let res =
                        commands::entregadigital::entregadigital_list_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "entregadigital_refresh_courses" => {
                    let res = commands::entregadigital::entregadigital_refresh_courses(wrap_state(
                        &state,
                    ))
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_concursos_check_session" => {
                    let res = commands::estrategia_concursos::estrategia_concursos_check_session(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_concursos_logout" => {
                    commands::estrategia_concursos::estrategia_concursos_logout(wrap_state(&state))
                        .await?;
                    Ok(serde_json::Value::Null)
                }
                "estrategia_concursos_list_courses" => {
                    let res = commands::estrategia_concursos::estrategia_concursos_list_courses(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_concursos_refresh_courses" => {
                    let res = commands::estrategia_concursos::estrategia_concursos_refresh_courses(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_ldi_check_session" => {
                    let res =
                        commands::estrategia_ldi::estrategia_ldi_check_session(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_ldi_logout" => {
                    commands::estrategia_ldi::estrategia_ldi_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "estrategia_ldi_list_courses" => {
                    let res =
                        commands::estrategia_ldi::estrategia_ldi_list_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_ldi_refresh_courses" => {
                    let res = commands::estrategia_ldi::estrategia_ldi_refresh_courses(wrap_state(
                        &state,
                    ))
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_militares_check_session" => {
                    let res = commands::estrategia_militares::estrategia_militares_check_session(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_militares_logout" => {
                    commands::estrategia_militares::estrategia_militares_logout(wrap_state(&state))
                        .await?;
                    Ok(serde_json::Value::Null)
                }
                "estrategia_militares_list_courses" => {
                    let res = commands::estrategia_militares::estrategia_militares_list_courses(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "estrategia_militares_refresh_courses" => {
                    let res = commands::estrategia_militares::estrategia_militares_refresh_courses(
                        wrap_state(&state),
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "fluency_check_session" => {
                    let res =
                        commands::fluencyacademy::fluency_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "fluency_logout" => {
                    commands::fluencyacademy::fluency_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "fluency_list_courses" => {
                    let res =
                        commands::fluencyacademy::fluency_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "fluency_refresh_courses" => {
                    let res = commands::fluencyacademy::fluency_refresh_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "grancursos_check_session" => {
                    let res =
                        commands::grancursos::grancursos_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "grancursos_logout" => {
                    commands::grancursos::grancursos_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "grancursos_list_courses" => {
                    let res =
                        commands::grancursos::grancursos_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "grancursos_refresh_courses" => {
                    let res = commands::grancursos::grancursos_refresh_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcel_check_session" => {
                    let res = commands::medcel::medcel_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcel_logout" => {
                    commands::medcel::medcel_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "medcel_list_courses" => {
                    let res = commands::medcel::medcel_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcel_refresh_courses" => {
                    let res = commands::medcel::medcel_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcof_check_session" => {
                    let res = commands::medcof::medcof_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcof_logout" => {
                    commands::medcof::medcof_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "medcof_list_courses" => {
                    let res = commands::medcof::medcof_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcof_refresh_courses" => {
                    let res = commands::medcof::medcof_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medway_check_session" => {
                    let res = commands::medway::medway_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medway_logout" => {
                    commands::medway::medway_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "medway_list_courses" => {
                    let res = commands::medway::medway_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medway_refresh_courses" => {
                    let res = commands::medway::medway_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "nutror_check_session" => {
                    let res = commands::nutror::nutror_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "nutror_logout" => {
                    commands::nutror::nutror_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "nutror_list_courses" => {
                    let res = commands::nutror::nutror_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "nutror_refresh_courses" => {
                    let res = commands::nutror::nutror_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "themembers_check_session" => {
                    let res =
                        commands::themembers::themembers_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "themembers_logout" => {
                    commands::themembers::themembers_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "themembers_list_courses" => {
                    let res =
                        commands::themembers::themembers_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "themembers_refresh_courses" => {
                    let res = commands::themembers::themembers_refresh_courses(wrap_state(&state))
                        .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "voomp_check_session" => {
                    let res = commands::voomp::voomp_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "voomp_logout" => {
                    commands::voomp::voomp_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "voomp_list_courses" => {
                    let res = commands::voomp::voomp_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "voomp_refresh_courses" => {
                    let res = commands::voomp::voomp_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "afya_check_session" => {
                    let res = commands::afya::afya_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "afya_logout" => {
                    commands::afya::afya_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "afya_list_courses" => {
                    let res = commands::afya::afya_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "afya_refresh_courses" => {
                    let res = commands::afya::afya_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "alpaclass_check_session" => {
                    let res =
                        commands::alpaclass::alpaclass_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "alpaclass_logout" => {
                    commands::alpaclass::alpaclass_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "alpaclass_list_courses" => {
                    let res =
                        commands::alpaclass::alpaclass_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "alpaclass_refresh_courses" => {
                    let res =
                        commands::alpaclass::alpaclass_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "areademembros_check_session" => {
                    let res =
                        commands::areademembros::areademembros_check_session(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "areademembros_logout" => {
                    commands::areademembros::areademembros_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "areademembros_list_courses" => {
                    let res =
                        commands::areademembros::areademembros_list_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "areademembros_refresh_courses" => {
                    let res =
                        commands::areademembros::areademembros_refresh_courses(wrap_state(&state))
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "astron_check_session" => {
                    let res =
                        commands::astronmembers::astron_check_session(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "astron_logout" => {
                    commands::astronmembers::astron_logout(wrap_state(&state)).await?;
                    Ok(serde_json::Value::Null)
                }
                "astron_list_courses" => {
                    let res =
                        commands::astronmembers::astron_list_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "astron_refresh_courses" => {
                    let res =
                        commands::astronmembers::astron_refresh_courses(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }

                // --- Downloads (36 platforms) ---
                "start_course_download" => dispatch_download!(commands::downloads::start_course_download),
                "start_gumroad_download" => dispatch_download!(commands::gumroad::start_gumroad_download),
                "start_udemy_course_download" => dispatch_download!(commands::udemy_downloads::start_udemy_course_download),
                "start_kiwify_course_download" => dispatch_download!(commands::kiwify::start_kiwify_course_download),
                "start_skool_course_download" => dispatch_download!(commands::skool::start_skool_course_download),
                "start_rocketseat_course_download" => dispatch_download!(commands::rocketseat::start_rocketseat_course_download),
                "start_teachable_course_download" => dispatch_download!(commands::teachable::start_teachable_course_download),
                "start_kajabi_course_download" => dispatch_download!(commands::kajabi::start_kajabi_course_download),
                "start_thinkific_course_download" => dispatch_download!(commands::thinkific::start_thinkific_course_download),
                "start_pluralsight_course_download" => dispatch_download!(commands::pluralsight::start_pluralsight_course_download),
                "start_wondrium_course_download" => dispatch_download!(commands::greatcourses::start_wondrium_course_download),
                "start_masterclass_course_download" => dispatch_download!(commands::masterclass::start_masterclass_course_download),
                "start_greenn_course_download" => dispatch_download!(commands::greenn::start_greenn_course_download),
                "start_kirvano_course_download" => dispatch_download!(commands::kirvano::start_kirvano_course_download),
                "start_cademi_course_download" => dispatch_download!(commands::cademi_cmd::start_cademi_course_download),
                "start_memberkit_course_download" => dispatch_download!(commands::memberkit_cmd::start_memberkit_course_download),
                "start_cakto_course_download" => dispatch_download!(commands::cakto::start_cakto_course_download),
                "start_caktomembers_course_download" => dispatch_download!(commands::caktomembers::start_caktomembers_course_download),
                "start_curseduca_course_download" => dispatch_download!(commands::curseduca::start_curseduca_course_download),
                "start_dsa_course_download" => dispatch_download!(commands::dsa::start_dsa_course_download),
                "start_entregadigital_course_download" => dispatch_download!(commands::entregadigital::start_entregadigital_course_download),
                "start_estrategia_concursos_course_download" => dispatch_download!(commands::estrategia_concursos::start_estrategia_concursos_course_download),
                "start_estrategia_ldi_course_download" => dispatch_download!(commands::estrategia_ldi::start_estrategia_ldi_course_download),
                "start_estrategia_militares_course_download" => dispatch_download!(commands::estrategia_militares::start_estrategia_militares_course_download),
                "start_fluency_course_download" => dispatch_download!(commands::fluencyacademy::start_fluency_course_download),
                "start_grancursos_course_download" => dispatch_download!(commands::grancursos::start_grancursos_course_download),
                "start_medcel_course_download" => dispatch_download!(commands::medcel::start_medcel_course_download),
                "start_medcof_course_download" => dispatch_download!(commands::medcof::start_medcof_course_download),
                "start_medway_course_download" => dispatch_download!(commands::medway::start_medway_course_download),
                "start_nutror_course_download" => dispatch_download!(commands::nutror::start_nutror_course_download),
                "start_themembers_course_download" => dispatch_download!(commands::themembers::start_themembers_course_download),
                "start_voomp_course_download" => dispatch_download!(commands::voomp::start_voomp_course_download),
                "start_afya_course_download" => dispatch_download!(commands::afya::start_afya_course_download),
                "start_alpaclass_course_download" => dispatch_download!(commands::alpaclass::start_alpaclass_course_download),
                "start_areademembros_course_download" => dispatch_download!(commands::areademembros::start_areademembros_course_download),
                "start_astron_course_download" => dispatch_download!(commands::astronmembers::start_astron_course_download),

                // --- Udemy Listing & Auth ---
                "udemy_list_courses" => {
                    let res = if let Some(app_handle) = app {
                        commands::udemy_courses::udemy_list_courses(app_handle, wrap_state(&state)).await?
                    } else {
                        commands::udemy_courses::fetch_courses_via_api(&wrap_state(&state)).await?
                    };
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "udemy_refresh_courses" => {
                    let res = if let Some(app_handle) = app {
                        commands::udemy_courses::udemy_refresh_courses(app_handle, wrap_state(&state)).await?
                    } else {
                        commands::udemy_courses::fetch_courses_via_api(&wrap_state(&state)).await?
                    };
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "udemy_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().unwrap_or(serde_json::Value::String(String::new()))
                    ).unwrap_or_default();
                    let app_handle = app.ok_or_else(|| "Tauri AppHandle not available on CoursesPlugin".to_string())?;
                    commands::udemy_auth::udemy_login(app_handle, wrap_state(&state), email).await?;
                    Ok(serde_json::Value::Null)
                }

                // --- Search Commands ---
                "estrategia_militares_search_courses" => {
                    let query: String = serde_json::from_value(
                        args.get("query").cloned().unwrap_or(serde_json::Value::String(String::new()))
                    ).unwrap_or_default();
                    let res = commands::estrategia_militares::estrategia_militares_search_courses(wrap_state(&state), query).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "rocketseat_search_courses" => {
                    let query: String = serde_json::from_value(
                        args.get("query").cloned().unwrap_or(serde_json::Value::String(String::new()))
                    ).unwrap_or_default();
                    let res = commands::rocketseat::rocketseat_search_courses(wrap_state(&state), query).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }

                // --- Login Methods ---
                "hotmart_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let app_handle = app.ok_or_else(|| "Tauri AppHandle not available on CoursesPlugin".to_string())?;
                    let res = commands::auth::hotmart_login(app_handle, wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "hotmart_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::auth::hotmart_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cademi_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::cademi_cmd::cademi_login(wrap_state(&state), email, password, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cademi_login_cookie" => {
                    let cookie: String = serde_json::from_value(
                        args.get("cookie").or(args.get("cookies")).cloned().ok_or_else(|| "missing 'cookie'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::cademi_cmd::cademi_login_cookie(wrap_state(&state), cookie, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "memberkit_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::memberkit_cmd::memberkit_login(wrap_state(&state), email, password, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "memberkit_login_cookie" => {
                    let cookie: String = serde_json::from_value(
                        args.get("cookie").or(args.get("cookies")).cloned().ok_or_else(|| "missing 'cookie'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::memberkit_cmd::memberkit_login_cookie(wrap_state(&state), cookie, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cakto_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::cakto::cakto_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "cakto_login_token" => {
                    let cookie: String = serde_json::from_value(
                        args.get("cookie").or(args.get("cookies")).or(args.get("token")).cloned().ok_or_else(|| "missing 'cookie'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::cakto::cakto_login_token(wrap_state(&state), cookie).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "curseduca_login" => {
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let username: String = serde_json::from_value(
                        args.get("username").or(args.get("email")).cloned().ok_or_else(|| "missing 'username'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::curseduca::curseduca_login(wrap_state(&state), site_url, username, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "curseduca_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let api_key: String = args.get("api_key").or(args.get("apiKey")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::curseduca::curseduca_login_token(wrap_state(&state), token, api_key, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "fluency_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::fluencyacademy::fluency_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "fluency_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::fluencyacademy::fluency_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "wondrium_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::greatcourses::wondrium_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "wondrium_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::greatcourses::wondrium_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kirvano_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::kirvano::kirvano_login(wrap_state(&state), email, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kirvano_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::kirvano::kirvano_login_token(wrap_state(&state), token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcel_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let api_key: String = args.get("api_key").or(args.get("apiKey")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::medcel::medcel_login(wrap_state(&state), email, password, api_key).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "medcel_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let api_key: String = args.get("api_key").or(args.get("apiKey")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::medcel::medcel_login_token(wrap_state(&state), token, api_key).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "themembers_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let domain: String = args.get("domain").or(args.get("subdomain")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::themembers::themembers_login(wrap_state(&state), email, password, domain).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "themembers_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let domain: String = args.get("domain").or(args.get("subdomain")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::themembers::themembers_login_token(wrap_state(&state), token, domain).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "afya_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let api_key: String = args.get("api_key").or(args.get("apiKey")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::afya::afya_login(wrap_state(&state), email, password, api_key).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "afya_login_token" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let api_key: String = args.get("api_key").or(args.get("apiKey")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::afya::afya_login_token(wrap_state(&state), token, api_key).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "alpaclass_login" => {
                    let token: String = serde_json::from_value(
                        args.get("token").cloned().ok_or_else(|| "missing 'token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let platform_url: String = args.get("platform_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::alpaclass::alpaclass_login(wrap_state(&state), token, platform_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "astron_login" => {
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let login: String = serde_json::from_value(
                        args.get("login").or(args.get("email")).or(args.get("username")).cloned().ok_or_else(|| "missing 'login'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or_else(|| "missing 'password'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::astronmembers::astron_login(wrap_state(&state), site_url, login, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "astron_login_token" => {
                    let cookies: String = serde_json::from_value(
                        args.get("cookies").or(args.get("cookie")).or(args.get("token")).cloned().ok_or_else(|| "missing 'cookies'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let site_url: String = args.get("site_url").or(args.get("url")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let res = commands::astronmembers::astron_login_token(wrap_state(&state), cookies, site_url).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }

                // --- Kajabi & Teachable helpers ---
                "kajabi_list_sites" => {
                    let res = commands::kajabi::kajabi_list_sites(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_request_login_link" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::kajabi::kajabi_request_login_link(email).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_set_site" => {
                    let site_id: String = serde_json::from_value(
                        args.get("site_id").or(args.get("siteId")).cloned().ok_or_else(|| "missing 'site_id'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::kajabi::kajabi_set_site(wrap_state(&state), site_id).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "kajabi_verify_login" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let confirmation_code: String = serde_json::from_value(
                        args.get("confirmation_code").or(args.get("confirmationCode")).cloned().ok_or_else(|| "missing 'confirmation_code'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let login_token: String = serde_json::from_value(
                        args.get("login_token").or(args.get("loginToken")).cloned().ok_or_else(|| "missing 'login_token'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::kajabi::kajabi_verify_login(wrap_state(&state), email, confirmation_code, login_token).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_list_schools" => {
                    let res = commands::teachable::teachable_list_schools(wrap_state(&state)).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_request_otp" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::teachable::teachable_request_otp(email).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_set_school" => {
                    let school_id: String = serde_json::from_value(
                        args.get("school_id").or(args.get("schoolId")).cloned().ok_or_else(|| "missing 'school_id'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::teachable::teachable_set_school(wrap_state(&state), school_id).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "teachable_verify_otp" => {
                    let email: String = serde_json::from_value(
                        args.get("email").cloned().ok_or_else(|| "missing 'email'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let otp_code: String = serde_json::from_value(
                        args.get("otp_code").or(args.get("otpCode")).cloned().ok_or_else(|| "missing 'otp_code'".to_string())?
                    ).map_err(|e| e.to_string())?;
                    let res = commands::teachable::teachable_verify_otp(wrap_state(&state), email, otp_code).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }

                _ => Err(format!("Unknown command: {}", command)),
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "afya_check_session".into(),
            "afya_list_courses".into(),
            "afya_logout".into(),
            "afya_refresh_courses".into(),
            "alpaclass_check_session".into(),
            "alpaclass_list_courses".into(),
            "alpaclass_logout".into(),
            "alpaclass_refresh_courses".into(),
            "areademembros_check_session".into(),
            "areademembros_list_courses".into(),
            "areademembros_logout".into(),
            "areademembros_refresh_courses".into(),
            "astron_check_session".into(),
            "astron_list_courses".into(),
            "astron_logout".into(),
            "astron_refresh_courses".into(),
            "cademi_check_session".into(),
            "cademi_list_courses".into(),
            "cademi_logout".into(),
            "cademi_refresh_courses".into(),
            "cakto_check_session".into(),
            "cakto_list_courses".into(),
            "cakto_logout".into(),
            "cakto_refresh_courses".into(),
            "caktomembers_check_session".into(),
            "caktomembers_list_courses".into(),
            "caktomembers_logout".into(),
            "caktomembers_refresh_courses".into(),
            "cancel_course_download".into(),
            "cancel_udemy_course_download".into(),
            "curseduca_check_session".into(),
            "curseduca_list_courses".into(),
            "curseduca_logout".into(),
            "curseduca_refresh_courses".into(),
            "dsa_check_session".into(),
            "dsa_list_courses".into(),
            "dsa_logout".into(),
            "dsa_refresh_courses".into(),
            "entregadigital_check_session".into(),
            "entregadigital_list_courses".into(),
            "entregadigital_logout".into(),
            "entregadigital_refresh_courses".into(),
            "estrategia_concursos_check_session".into(),
            "estrategia_concursos_list_courses".into(),
            "estrategia_concursos_logout".into(),
            "estrategia_concursos_refresh_courses".into(),
            "estrategia_ldi_check_session".into(),
            "estrategia_ldi_list_courses".into(),
            "estrategia_ldi_logout".into(),
            "estrategia_ldi_refresh_courses".into(),
            "estrategia_militares_check_session".into(),
            "estrategia_militares_list_courses".into(),
            "estrategia_militares_logout".into(),
            "estrategia_militares_refresh_courses".into(),
            "fluency_check_session".into(),
            "fluency_list_courses".into(),
            "fluency_logout".into(),
            "fluency_refresh_courses".into(),
            "get_active_downloads".into(),
            "get_platform_config".into(),
            "get_platforms".into(),
            "grancursos_check_session".into(),
            "grancursos_list_courses".into(),
            "grancursos_logout".into(),
            "grancursos_refresh_courses".into(),
            "greenn_check_session".into(),
            "greenn_list_courses".into(),
            "greenn_logout".into(),
            "greenn_refresh_courses".into(),
            "gumroad_check_session".into(),
            "gumroad_list_products".into(),
            "gumroad_logout".into(),
            "gumroad_refresh_products".into(),
            "hotmart_check_session".into(),
            "hotmart_get_modules".into(),
            "hotmart_list_courses".into(),
            "hotmart_logout".into(),
            "hotmart_refresh_courses".into(),
            "kajabi_check_session".into(),
            "kajabi_list_courses".into(),
            "kajabi_logout".into(),
            "kajabi_refresh_courses".into(),
            "kirvano_check_session".into(),
            "kirvano_list_courses".into(),
            "kirvano_logout".into(),
            "kirvano_refresh_courses".into(),
            "kiwify_check_session".into(),
            "kiwify_list_courses".into(),
            "kiwify_logout".into(),
            "kiwify_refresh_courses".into(),
            "masterclass_check_session".into(),
            "masterclass_list_courses".into(),
            "masterclass_logout".into(),
            "masterclass_refresh_courses".into(),
            "medcel_check_session".into(),
            "medcel_list_courses".into(),
            "medcel_logout".into(),
            "medcel_refresh_courses".into(),
            "medcof_check_session".into(),
            "medcof_list_courses".into(),
            "medcof_logout".into(),
            "medcof_refresh_courses".into(),
            "medway_check_session".into(),
            "medway_list_courses".into(),
            "medway_logout".into(),
            "medway_refresh_courses".into(),
            "memberkit_check_session".into(),
            "memberkit_list_courses".into(),
            "memberkit_logout".into(),
            "memberkit_refresh_courses".into(),
            "nutror_check_session".into(),
            "nutror_list_courses".into(),
            "nutror_logout".into(),
            "nutror_refresh_courses".into(),
            "pluralsight_check_session".into(),
            "pluralsight_list_courses".into(),
            "pluralsight_logout".into(),
            "pluralsight_refresh_courses".into(),
            "rocketseat_check_session".into(),
            "rocketseat_list_courses".into(),
            "rocketseat_logout".into(),
            "rocketseat_refresh_courses".into(),
            "skool_check_session".into(),
            "skool_list_groups".into(),
            "skool_logout".into(),
            "skool_refresh_groups".into(),
            "teachable_check_session".into(),
            "teachable_list_courses".into(),
            "teachable_logout".into(),
            "teachable_refresh_courses".into(),
            "themembers_check_session".into(),
            "themembers_list_courses".into(),
            "themembers_logout".into(),
            "themembers_refresh_courses".into(),
            "thinkific_check_session".into(),
            "thinkific_list_courses".into(),
            "thinkific_logout".into(),
            "thinkific_refresh_courses".into(),
            "udemy_check_session".into(),
            "udemy_get_portal".into(),
            "udemy_logout".into(),
            "udemy_login_cookies".into(),
            "thinkific_login".into(),
            "pluralsight_login_cookies".into(),
            "masterclass_login_cookies".into(),
            "grancursos_login_cookies".into(),
            "rocketseat_login_token".into(),
            "teachable_login_token".into(),
            "kajabi_login_token".into(),
            "skool_login".into(),
            "skool_login_token".into(),
            "kiwify_login".into(),
            "kiwify_login_token".into(),
            "gumroad_login".into(),
            "gumroad_login_token".into(),
            "greenn_login_token".into(),
            "caktomembers_login_token".into(),
            "dsa_login_token".into(),
            "entregadigital_login_token".into(),
            "estrategia_concursos_login_token".into(),
            "estrategia_ldi_login_token".into(),
            "estrategia_militares_login_token".into(),
            "medcof_login_token".into(),
            "medway_login_token".into(),
            "nutror_login_token".into(),
            "voomp_login_token".into(),
            "areademembros_login_token".into(),
            "voomp_check_session".into(),
            "voomp_list_courses".into(),
            "voomp_logout".into(),
            "voomp_refresh_courses".into(),
            "wondrium_check_session".into(),
            "wondrium_list_courses".into(),
            "wondrium_logout".into(),
            "wondrium_refresh_courses".into(),
            "udemy_list_courses".into(),
            "udemy_refresh_courses".into(),
            "udemy_login".into(),
            "estrategia_militares_search_courses".into(),
            "rocketseat_search_courses".into(),
            "hotmart_login".into(),
            "hotmart_login_token".into(),
            "cademi_login".into(),
            "cademi_login_cookie".into(),
            "memberkit_login".into(),
            "memberkit_login_cookie".into(),
            "cakto_login".into(),
            "cakto_login_token".into(),
            "curseduca_login".into(),
            "curseduca_login_token".into(),
            "fluency_login".into(),
            "fluency_login_token".into(),
            "wondrium_login".into(),
            "wondrium_login_token".into(),
            "kirvano_login".into(),
            "kirvano_login_token".into(),
            "medcel_login".into(),
            "medcel_login_token".into(),
            "themembers_login".into(),
            "themembers_login_token".into(),
            "afya_login".into(),
            "afya_login_token".into(),
            "alpaclass_login".into(),
            "astron_login".into(),
            "astron_login_token".into(),
            "kajabi_list_sites".into(),
            "kajabi_request_login_link".into(),
            "kajabi_set_site".into(),
            "kajabi_verify_login".into(),
            "teachable_list_schools".into(),
            "teachable_request_otp".into(),
            "teachable_set_school".into(),
            "teachable_verify_otp".into(),
            "start_course_download".into(),
            "start_gumroad_download".into(),
            "start_udemy_course_download".into(),
            "start_kiwify_course_download".into(),
            "start_skool_course_download".into(),
            "start_rocketseat_course_download".into(),
            "start_teachable_course_download".into(),
            "start_kajabi_course_download".into(),
            "start_thinkific_course_download".into(),
            "start_pluralsight_course_download".into(),
            "start_wondrium_course_download".into(),
            "start_masterclass_course_download".into(),
            "start_greenn_course_download".into(),
            "start_kirvano_course_download".into(),
            "start_cademi_course_download".into(),
            "start_memberkit_course_download".into(),
            "start_cakto_course_download".into(),
            "start_caktomembers_course_download".into(),
            "start_curseduca_course_download".into(),
            "start_dsa_course_download".into(),
            "start_entregadigital_course_download".into(),
            "start_estrategia_concursos_course_download".into(),
            "start_estrategia_ldi_course_download".into(),
            "start_estrategia_militares_course_download".into(),
            "start_fluency_course_download".into(),
            "start_grancursos_course_download".into(),
            "start_medcel_course_download".into(),
            "start_medcof_course_download".into(),
            "start_medway_course_download".into(),
            "start_nutror_course_download".into(),
            "start_themembers_course_download".into(),
            "start_voomp_course_download".into(),
            "start_afya_course_download".into(),
            "start_alpaclass_course_download".into(),
            "start_areademembros_course_download".into(),
            "start_astron_course_download".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_catalog_commands_supported() {
        let plugin = CoursesPlugin::new();
        let supported: std::collections::HashSet<String> = plugin.supported_commands().into_iter().collect();
        let platforms = catalog::all_platforms();

        let mut missing = Vec::new();
        for p in &platforms {
            for m in &p.login_methods {
                if !supported.contains(&m.command) {
                    missing.push(format!("[{}] login method '{}'", p.id, m.command));
                }
            }
            let cmds = [
                ("check_session", Some(&p.commands.check_session)),
                ("logout", Some(&p.commands.logout)),
                ("list", Some(&p.commands.list)),
                ("refresh", Some(&p.commands.refresh)),
                ("download", Some(&p.commands.download)),
                ("cancel", p.commands.cancel.as_ref()),
                ("search", p.commands.search.as_ref()),
                ("curriculum", p.commands.curriculum.as_ref()),
            ];
            for (kind, cmd_opt) in cmds {
                if let Some(cmd) = cmd_opt {
                    if !supported.contains(cmd) {
                        missing.push(format!("[{}] {} '{}'", p.id, kind, cmd));
                    }
                }
            }
        }

        assert!(
            missing.is_empty(),
            "Catalog commands missing from supported_commands:\n{}",
            missing.join("\n")
        );
    }

    #[tokio::test]
    async fn test_all_catalog_commands_dispatched_without_unknown_command() {
        let plugin = CoursesPlugin::new();
        let platforms = catalog::all_platforms();

        let mut unknown_cmds = Vec::new();

        for p in &platforms {
            let mut all_cmds = Vec::new();
            for m in &p.login_methods {
                all_cmds.push(m.command.clone());
            }
            all_cmds.push(p.commands.check_session.clone());
            all_cmds.push(p.commands.logout.clone());
            all_cmds.push(p.commands.list.clone());
            all_cmds.push(p.commands.refresh.clone());
            all_cmds.push(p.commands.download.clone());
            if let Some(ref c) = p.commands.cancel { all_cmds.push(c.clone()); }
            if let Some(ref s) = p.commands.search { all_cmds.push(s.clone()); }
            if let Some(ref cu) = p.commands.curriculum { all_cmds.push(cu.clone()); }

            for cmd in all_cmds {
                let res = plugin.handle_command(cmd.clone(), serde_json::json!({})).await;
                if let Err(ref e) = res {
                    if e.contains("Unknown command") {
                        unknown_cmds.push(format!("[{}] {}", p.id, cmd));
                    }
                }
            }
        }

        assert!(
            unknown_cmds.is_empty(),
            "Catalog commands not dispatched in handle_command (returned Unknown command):\n{}",
            unknown_cmds.join("\n")
        );
    }
}
