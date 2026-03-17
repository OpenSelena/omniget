use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;

use platforms::hotmart::api::Course;
use platforms::hotmart::auth::HotmartSession;
use platforms::udemy::api::UdemyCourse;
use platforms::udemy::auth::UdemySession;
use platforms::telegram::auth::{TelegramSessionHandle, TelegramState};
use tokio_util::sync::CancellationToken;

pub struct P2pSendHandle {
    pub cancel_token: CancellationToken,
    pub paused: Arc<std::sync::atomic::AtomicBool>,
}
pub type ActiveP2pSends = Arc<tokio::sync::Mutex<HashMap<String, P2pSendHandle>>>;

pub mod commands;
pub mod core;
pub mod hotkey;
pub mod models;
pub mod platforms;
pub mod storage;
pub mod tray;

pub struct CoursesCache {
    pub courses: Vec<Course>,
    pub fetched_at: std::time::Instant,
}

pub struct UdemyCoursesCache {
    pub courses: Vec<UdemyCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::kiwify::api::KiwifyCourse;

pub struct KiwifyCoursesCache {
    pub courses: Vec<KiwifyCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::greennclub::api::GreennCourse;

pub struct GreennCoursesCache {
    pub courses: Vec<GreennCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::voompplay::api::VoompCourse;

pub struct VoompCoursesCache {
    pub courses: Vec<VoompCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::entregadigital::api::EntregaDigitalCourse;

pub struct EntregaDigitalCoursesCache {
    pub courses: Vec<EntregaDigitalCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::alpaclass::api::AlpaclassCourse;

pub struct AlpaclassCoursesCache {
    pub courses: Vec<AlpaclassCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::themembers::api::TheMembersCourse;

pub struct TheMembersCoursesCache {
    pub courses: Vec<TheMembersCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::gumroad::api::GumroadProduct;

pub struct GumroadCoursesCache {
    pub products: Vec<GumroadProduct>,
    pub fetched_at: std::time::Instant,
}

use platforms::kirvano::api::KirvanoCourse;

pub struct KirvanoCoursesCache {
    pub courses: Vec<KirvanoCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::datascienceacademy::api::DsaCourse;

pub struct DsaCoursesCache {
    pub courses: Vec<DsaCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::medcel::api::MedcelCourse;

pub struct MedcelCoursesCache {
    pub courses: Vec<MedcelCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::afyainternato::api::AfyaCourse;

pub struct AfyaCoursesCache {
    pub courses: Vec<AfyaCourse>,
    pub fetched_at: std::time::Instant,
}

use platforms::medway::api::MedwayCourse;

pub struct MedwayCoursesCache {
    pub courses: Vec<MedwayCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct AppState {
    pub hotmart_session: Arc<tokio::sync::Mutex<Option<HotmartSession>>>,
    pub active_downloads: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
    pub active_generic_downloads: Arc<tokio::sync::Mutex<HashMap<u64, (String, CancellationToken)>>>,
    pub active_conversions: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
    pub registry: core::registry::PlatformRegistry,
    pub courses_cache: Arc<tokio::sync::Mutex<Option<CoursesCache>>>,
    pub session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub telegram_session: TelegramSessionHandle,
    pub download_queue: Arc<tokio::sync::Mutex<core::queue::DownloadQueue>>,
    pub auth_registry: core::auth::AuthRegistry,
    pub udemy_session: Arc<tokio::sync::Mutex<Option<UdemySession>>>,
    pub udemy_courses_cache: Arc<tokio::sync::Mutex<Option<UdemyCoursesCache>>>,
    pub udemy_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub udemy_api_webview: Arc<tokio::sync::Mutex<Option<tauri::WebviewWindow>>>,
    pub udemy_api_result: Arc<std::sync::Mutex<Option<String>>>,
    pub torrent_session: Arc<tokio::sync::Mutex<Option<Arc<librqbit::Session>>>>,
    pub active_p2p_sends: ActiveP2pSends,
    pub kiwify_session: Arc<tokio::sync::Mutex<Option<platforms::kiwify::api::KiwifySession>>>,
    pub kiwify_courses_cache: Arc<tokio::sync::Mutex<Option<KiwifyCoursesCache>>>,
    pub kiwify_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub greenn_session: Arc<tokio::sync::Mutex<Option<platforms::greennclub::api::GreennSession>>>,
    pub greenn_courses_cache: Arc<tokio::sync::Mutex<Option<GreennCoursesCache>>>,
    pub greenn_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub voomp_session: Arc<tokio::sync::Mutex<Option<platforms::voompplay::api::VoompSession>>>,
    pub voomp_courses_cache: Arc<tokio::sync::Mutex<Option<VoompCoursesCache>>>,
    pub voomp_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub entregadigital_session: Arc<tokio::sync::Mutex<Option<platforms::entregadigital::api::EntregaDigitalSession>>>,
    pub entregadigital_courses_cache: Arc<tokio::sync::Mutex<Option<EntregaDigitalCoursesCache>>>,
    pub entregadigital_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub alpaclass_session: Arc<tokio::sync::Mutex<Option<platforms::alpaclass::api::AlpaclassSession>>>,
    pub alpaclass_courses_cache: Arc<tokio::sync::Mutex<Option<AlpaclassCoursesCache>>>,
    pub alpaclass_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub themembers_session: Arc<tokio::sync::Mutex<Option<platforms::themembers::api::TheMembersSession>>>,
    pub themembers_courses_cache: Arc<tokio::sync::Mutex<Option<TheMembersCoursesCache>>>,
    pub themembers_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub gumroad_session: Arc<tokio::sync::Mutex<Option<platforms::gumroad::api::GumroadSession>>>,
    pub gumroad_courses_cache: Arc<tokio::sync::Mutex<Option<GumroadCoursesCache>>>,
    pub gumroad_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub kirvano_session: Arc<tokio::sync::Mutex<Option<platforms::kirvano::api::KirvanoSession>>>,
    pub kirvano_courses_cache: Arc<tokio::sync::Mutex<Option<KirvanoCoursesCache>>>,
    pub kirvano_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub dsa_session: Arc<tokio::sync::Mutex<Option<platforms::datascienceacademy::api::DsaSession>>>,
    pub dsa_courses_cache: Arc<tokio::sync::Mutex<Option<DsaCoursesCache>>>,
    pub dsa_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub medcel_session: Arc<tokio::sync::Mutex<Option<platforms::medcel::api::MedcelSession>>>,
    pub medcel_courses_cache: Arc<tokio::sync::Mutex<Option<MedcelCoursesCache>>>,
    pub medcel_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub afya_session: Arc<tokio::sync::Mutex<Option<platforms::afyainternato::api::AfyaSession>>>,
    pub afya_courses_cache: Arc<tokio::sync::Mutex<Option<AfyaCoursesCache>>>,
    pub afya_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub medway_session: Arc<tokio::sync::Mutex<Option<platforms::medway::api::MedwaySession>>>,
    pub medway_courses_cache: Arc<tokio::sync::Mutex<Option<MedwayCoursesCache>>>,
    pub medway_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let session = Arc::new(tokio::sync::Mutex::new(None));
    let telegram_session: TelegramSessionHandle =
        Arc::new(tokio::sync::Mutex::new(TelegramState::new()));

    let mut registry = core::registry::PlatformRegistry::new();
    registry.register(Arc::new(
        platforms::hotmart::downloader::HotmartDownloader::new(
            session.clone(),
            models::settings::AppSettings::default().download,
            20,
            3,
            8,
        ),
    ));
    registry.register(Arc::new(
        platforms::instagram::InstagramDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::pinterest::PinterestDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::tiktok::TikTokDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::twitter::TwitterDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::twitch::TwitchClipsDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::bluesky::BlueskyDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::reddit::RedditDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::youtube::YouTubeDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::vimeo::VimeoDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::telegram::downloader::TelegramDownloader::new(
            telegram_session.clone(),
        ),
    ));
    let torrent_session: Arc<tokio::sync::Mutex<Option<Arc<librqbit::Session>>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    registry.register(Arc::new(
        platforms::magnet::MagnetDownloader::new(torrent_session.clone()),
    ));
    registry.register(Arc::new(
        platforms::p2p::P2pDownloader::new(),
    ));
    registry.register(Arc::new(platforms::kiwify::KiwifyDownloader::new()));
    registry.register(Arc::new(platforms::gumroad::GumroadDownloader::new()));
    registry.register(Arc::new(platforms::teachable::TeachableDownloader::new()));
    registry.register(Arc::new(platforms::kajabi::KajabiDownloader::new()));
    registry.register(Arc::new(platforms::skool::SkoolDownloader::new()));
    registry.register(Arc::new(platforms::pluralsight::PluralsightDownloader::new()));
    registry.register(Arc::new(platforms::greatcourses::GreatCoursesDownloader::new()));
    registry.register(Arc::new(platforms::masterclass::MasterClassDownloader::new()));
    registry.register(Arc::new(platforms::thinkific::ThinkificDownloader::new()));
    registry.register(Arc::new(platforms::curseduca::CurseducaDownloader::new()));
    registry.register(Arc::new(platforms::cademi::CademiDownloader::new()));
    registry.register(Arc::new(platforms::cakto::CaktoDownloader::new()));
    registry.register(Arc::new(platforms::kirvano::KirvanoDownloader::new()));
    registry.register(Arc::new(platforms::memberkit::MemberkitDownloader::new()));
    registry.register(Arc::new(platforms::rocketseat::RocketseatDownloader::new()));
    registry.register(Arc::new(platforms::grancursos::GrancursosDownloader::new()));
    registry.register(Arc::new(platforms::fluencyacademy::FluencyAcademyDownloader::new()));
    registry.register(Arc::new(platforms::datascienceacademy::DataScienceAcademyDownloader::new()));
    registry.register(Arc::new(platforms::medcel::MedcelDownloader::new()));
    registry.register(Arc::new(platforms::medcof::MedcofDownloader::new()));
    registry.register(Arc::new(platforms::medway::MedwayDownloader::new()));
    registry.register(Arc::new(platforms::afyainternato::AfyaInternatoDownloader::new()));
    registry.register(Arc::new(platforms::alpaclass::AlpaclassDownloader::new()));
    registry.register(Arc::new(platforms::areademembros::AreaDeMembrosDownloader::new()));
    registry.register(Arc::new(platforms::astronmembers::AstronMembersDownloader::new()));
    registry.register(Arc::new(platforms::eduzznutror::EduzzNutrorDownloader::new()));
    registry.register(Arc::new(platforms::entregadigital::EntregaDigitalDownloader::new()));
    registry.register(Arc::new(platforms::greennclub::GreennClubDownloader::new()));
    registry.register(Arc::new(platforms::themembers::TheMembersDownloader::new()));
    registry.register(Arc::new(platforms::voompplay::VoompPlayDownloader::new()));
    registry.register(Arc::new(
        platforms::generic_ytdlp::GenericYtdlpDownloader::new(),
    ));

    let auth_registry = core::auth::AuthRegistry::new();

    let state = AppState {
        hotmart_session: session,
        active_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        active_generic_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        active_conversions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        registry,
        courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        telegram_session,
        download_queue: Arc::new(tokio::sync::Mutex::new(core::queue::DownloadQueue::new(2))),
        auth_registry,
        udemy_session: Arc::new(tokio::sync::Mutex::new(None)),
        udemy_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        udemy_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        udemy_api_webview: Arc::new(tokio::sync::Mutex::new(None)),
        udemy_api_result: Arc::new(std::sync::Mutex::new(None)),
        torrent_session,
        active_p2p_sends: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        kiwify_session: Arc::new(tokio::sync::Mutex::new(None)),
        kiwify_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        kiwify_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        greenn_session: Arc::new(tokio::sync::Mutex::new(None)),
        greenn_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        greenn_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        voomp_session: Arc::new(tokio::sync::Mutex::new(None)),
        voomp_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        voomp_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        entregadigital_session: Arc::new(tokio::sync::Mutex::new(None)),
        entregadigital_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        entregadigital_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        alpaclass_session: Arc::new(tokio::sync::Mutex::new(None)),
        alpaclass_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        alpaclass_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        themembers_session: Arc::new(tokio::sync::Mutex::new(None)),
        themembers_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        themembers_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        gumroad_session: Arc::new(tokio::sync::Mutex::new(None)),
        gumroad_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        gumroad_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        kirvano_session: Arc::new(tokio::sync::Mutex::new(None)),
        kirvano_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        kirvano_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        dsa_session: Arc::new(tokio::sync::Mutex::new(None)),
        dsa_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        dsa_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        medcel_session: Arc::new(tokio::sync::Mutex::new(None)),
        medcel_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        medcel_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        afya_session: Arc::new(tokio::sync::Mutex::new(None)),
        afya_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        afya_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        medway_session: Arc::new(tokio::sync::Mutex::new(None)),
        medway_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
        medway_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            tray::show_window(app);
            if let Some(url) = argv.get(1) {
                if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("magnet:") || url.starts_with("p2p:") {
                    let _ = app.emit("deep-link", url.clone());
                }
            }
        }))
        .manage(state)
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        hotkey::on_hotkey_pressed(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let settings = storage::config::load_settings(app.handle());
            core::http_client::init_proxy(settings.proxy.clone());
            tray::setup(app.handle())?;
            hotkey::register_from_settings(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::hotmart_login,
            commands::auth::hotmart_check_session,
            commands::auth::hotmart_logout,
            commands::courses::hotmart_list_courses,
            commands::courses::hotmart_refresh_courses,
            commands::courses::hotmart_get_modules,
            commands::diagnostics::get_rate_limit_stats,
            commands::downloads::start_course_download,
            commands::downloads::cancel_course_download,
            commands::downloads::get_active_downloads,
            commands::downloads::detect_platform,
            commands::downloads::get_media_formats,
            commands::downloads::prefetch_media_info,
            commands::downloads::download_from_url,
            commands::downloads::cancel_generic_download,
            commands::downloads::pause_download,
            commands::downloads::resume_download,
            commands::downloads::retry_download,
            commands::downloads::remove_download,
            commands::downloads::get_queue_state,
            commands::downloads::update_max_concurrent,
            commands::downloads::clear_finished_downloads,
            commands::downloads::reveal_file,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::settings::mark_onboarding_complete,
            commands::autostart::set_autostart,
            commands::autostart::get_autostart_status,
            commands::telegram::telegram_check_session,
            commands::telegram::telegram_qr_start,
            commands::telegram::telegram_qr_poll,
            commands::telegram::telegram_send_code,
            commands::telegram::telegram_verify_code,
            commands::telegram::telegram_verify_2fa,
            commands::telegram::telegram_logout,
            commands::telegram::telegram_list_chats,
            commands::telegram::telegram_list_media,
            commands::telegram::telegram_download_media,
            commands::telegram::telegram_download_batch,
            commands::telegram::telegram_cancel_batch,
            commands::telegram::telegram_get_thumbnail,
            commands::telegram::telegram_get_chat_photo,
            commands::telegram::telegram_search_media,
            commands::telegram::telegram_clear_thumbnail_cache,
            commands::convert::probe_file,
            commands::convert::convert_file,
            commands::convert::cancel_conversion,
            commands::convert::get_hwaccel_info,
            commands::dependencies::check_dependencies,
            commands::dependencies::check_ytdlp_available,
            commands::dependencies::install_dependency,
            commands::search::search_videos,
            commands::platform_auth::platform_auth_check,
            commands::platform_auth::platform_auth_login,
            commands::platform_auth::platform_auth_logout,
            commands::platform_auth::platform_auth_list,
            commands::udemy_auth::udemy_login,
            commands::udemy_auth::udemy_login_cookies,
            commands::udemy_auth::udemy_get_portal,
            commands::udemy_auth::udemy_check_session,
            commands::udemy_auth::udemy_logout,
            commands::udemy_courses::udemy_list_courses,
            commands::udemy_courses::udemy_refresh_courses,
            commands::udemy_downloads::start_udemy_course_download,
            commands::udemy_downloads::cancel_udemy_course_download,
            commands::p2p::p2p_send_file,
            commands::p2p::p2p_cancel_send,
            commands::p2p::p2p_pause_send,
            commands::p2p::p2p_resume_send,
            commands::p2p::p2p_get_active_sends,
            commands::p2p::p2p_validate_code,
            commands::kiwify::kiwify_login,
            commands::kiwify::kiwify_login_token,
            commands::kiwify::kiwify_check_session,
            commands::kiwify::kiwify_logout,
            commands::kiwify::kiwify_list_courses,
            commands::kiwify::kiwify_refresh_courses,
            commands::kiwify::start_kiwify_course_download,
            commands::greenn::greenn_login_token,
            commands::greenn::greenn_check_session,
            commands::greenn::greenn_logout,
            commands::greenn::greenn_list_courses,
            commands::greenn::greenn_refresh_courses,
            commands::greenn::start_greenn_course_download,
            commands::voomp::voomp_login_token,
            commands::voomp::voomp_check_session,
            commands::voomp::voomp_logout,
            commands::voomp::voomp_list_courses,
            commands::voomp::voomp_refresh_courses,
            commands::voomp::start_voomp_course_download,
            commands::entregadigital::entregadigital_login_token,
            commands::entregadigital::entregadigital_check_session,
            commands::entregadigital::entregadigital_logout,
            commands::entregadigital::entregadigital_list_courses,
            commands::entregadigital::entregadigital_refresh_courses,
            commands::entregadigital::start_entregadigital_course_download,
            commands::alpaclass::alpaclass_login,
            commands::alpaclass::alpaclass_check_session,
            commands::alpaclass::alpaclass_logout,
            commands::alpaclass::alpaclass_list_courses,
            commands::alpaclass::alpaclass_refresh_courses,
            commands::alpaclass::start_alpaclass_course_download,
            commands::themembers::themembers_login,
            commands::themembers::themembers_login_token,
            commands::themembers::themembers_check_session,
            commands::themembers::themembers_logout,
            commands::themembers::themembers_list_courses,
            commands::themembers::themembers_refresh_courses,
            commands::themembers::start_themembers_course_download,
            commands::medcel::medcel_login,
            commands::medcel::medcel_login_token,
            commands::medcel::medcel_check_session,
            commands::medcel::medcel_logout,
            commands::medcel::medcel_list_courses,
            commands::medcel::medcel_refresh_courses,
            commands::medcel::start_medcel_course_download,
            commands::afya::afya_login,
            commands::afya::afya_login_token,
            commands::afya::afya_check_session,
            commands::afya::afya_logout,
            commands::afya::afya_list_courses,
            commands::afya::afya_refresh_courses,
            commands::afya::start_afya_course_download,
            commands::medway::medway_login_token,
            commands::medway::medway_check_session,
            commands::medway::medway_logout,
            commands::medway::medway_list_courses,
            commands::medway::medway_refresh_courses,
            commands::medway::start_medway_course_download,
            commands::gumroad::gumroad_login,
            commands::gumroad::gumroad_login_token,
            commands::gumroad::gumroad_check_session,
            commands::gumroad::gumroad_logout,
            commands::gumroad::gumroad_list_products,
            commands::gumroad::gumroad_refresh_products,
            commands::gumroad::start_gumroad_download,
            commands::kirvano::kirvano_login,
            commands::kirvano::kirvano_login_token,
            commands::kirvano::kirvano_check_session,
            commands::kirvano::kirvano_logout,
            commands::kirvano::kirvano_list_courses,
            commands::kirvano::kirvano_refresh_courses,
            commands::kirvano::start_kirvano_course_download,
            commands::dsa::dsa_login_token,
            commands::dsa::dsa_check_session,
            commands::dsa::dsa_logout,
            commands::dsa::dsa_list_courses,
            commands::dsa::dsa_refresh_courses,
            commands::dsa::start_dsa_course_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
