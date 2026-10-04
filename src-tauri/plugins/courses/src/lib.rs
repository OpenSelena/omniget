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

pub struct CoursesPlugin {
    host: Option<Arc<dyn PluginHost>>,
    state: Arc<CoursesState>,
}

impl CoursesPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            state: Arc::new(CoursesState::default()),
        }
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
        Box::pin(async move {
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
            "voomp_check_session".into(),
            "voomp_list_courses".into(),
            "voomp_logout".into(),
            "voomp_refresh_courses".into(),
            "wondrium_check_session".into(),
            "wondrium_list_courses".into(),
            "wondrium_logout".into(),
            "wondrium_refresh_courses".into(),
        ]
    }
}

omniget_plugin_sdk::export_plugin!(CoursesPlugin::new());
