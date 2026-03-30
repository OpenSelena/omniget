use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use crate::platforms::hotmart::auth::HotmartSession;
use crate::platforms::hotmart::api::Course;
use crate::platforms::udemy::auth::UdemySession;
use crate::platforms::udemy::api::UdemyCourse;
use crate::platforms::gumroad::api::GumroadProduct;
use crate::platforms::skool::api::SkoolGroup;
use crate::platforms::kiwify::api::KiwifyCourse;
use crate::platforms::greennclub::api::GreennCourse;
use crate::platforms::voompplay::api::VoompCourse;
use crate::platforms::entregadigital::api::EntregaDigitalCourse;
use crate::platforms::alpaclass::api::AlpaclassCourse;
use crate::platforms::themembers::api::TheMembersCourse;
use crate::platforms::kirvano::api::KirvanoCourse;
use crate::platforms::datascienceacademy::api::DsaCourse;
use crate::platforms::medcel::api::MedcelCourse;
use crate::platforms::afyainternato::api::AfyaCourse;
use crate::platforms::medway::api::MedwayCourse;
use crate::platforms::estrategia_concursos::api::EstrategiaConcursosCourse;
use crate::platforms::estrategia_ldi::api::EstrategiaLdiCourse;
use crate::platforms::estrategia_militares::api::EstrategiaMilitaresCourse;
use crate::platforms::grancursos::api::GranCursosCourse;
use crate::platforms::teachable::api::TeachableCourse;
use crate::platforms::kajabi::api::KajabiCourse;
use crate::platforms::fluencyacademy::api::FluencyCourse;
use crate::platforms::eduzznutror::api::NutrorCourse;
use crate::platforms::cademi::api::CademiCourse;
use crate::platforms::memberkit::api::MemberkitCourse;
use crate::platforms::areademembros::api::AreaDeMembrosApiCourse;
use crate::platforms::astronmembers::api::AstronCourse;
use crate::platforms::cakto::api::CaktoCourse;
use crate::platforms::caktomembers::api::CaktoMembersCourse;
use crate::platforms::curseduca::api::CurseducaCourse;
use crate::platforms::medcof::api::MedcofCourse;
use crate::platforms::pluralsight::api::PluralsightCourse;
use crate::platforms::greatcourses::api::WondriumCourse;
use crate::platforms::masterclass::api::MasterClassCourse;
use crate::platforms::thinkific::api::ThinkificCourse;
use crate::platforms::rocketseat::api::RocketseatCourse;


pub struct CoursesCache {
    pub courses: Vec<Course>,
    pub fetched_at: std::time::Instant,
}

pub struct UdemyCoursesCache {
    pub courses: Vec<UdemyCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct KiwifyCoursesCache {
    pub courses: Vec<KiwifyCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct GreennCoursesCache {
    pub courses: Vec<GreennCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct VoompCoursesCache {
    pub courses: Vec<VoompCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct EntregaDigitalCoursesCache {
    pub courses: Vec<EntregaDigitalCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct AlpaclassCoursesCache {
    pub courses: Vec<AlpaclassCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct TheMembersCoursesCache {
    pub courses: Vec<TheMembersCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct GumroadCoursesCache {
    pub products: Vec<GumroadProduct>,
    pub fetched_at: std::time::Instant,
}

pub struct KirvanoCoursesCache {
    pub courses: Vec<KirvanoCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct DsaCoursesCache {
    pub courses: Vec<DsaCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct MedcelCoursesCache {
    pub courses: Vec<MedcelCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct AfyaCoursesCache {
    pub courses: Vec<AfyaCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct MedwayCoursesCache {
    pub courses: Vec<MedwayCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct EstrategiaConcursosCoursesCache {
    pub courses: Vec<EstrategiaConcursosCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct EstrategiaLdiCoursesCache {
    pub courses: Vec<EstrategiaLdiCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct EstrategiaMilitaresCoursesCache {
    pub courses: Vec<EstrategiaMilitaresCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct GranCursosCoursesCache {
    pub courses: Vec<GranCursosCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct SkoolCoursesCache {
    pub groups: Vec<SkoolGroup>,
    pub fetched_at: std::time::Instant,
}

pub struct TeachableCoursesCache {
    pub courses: Vec<TeachableCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct KajabiCoursesCache {
    pub courses: Vec<KajabiCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct FluencyCoursesCache {
    pub courses: Vec<FluencyCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct NutrorCoursesCache {
    pub courses: Vec<NutrorCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct CademiCoursesCache {
    pub courses: Vec<CademiCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct MemberkitCoursesCache {
    pub courses: Vec<MemberkitCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct AreaDeMembrosCoursesCache {
    pub courses: Vec<AreaDeMembrosApiCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct AstronCoursesCache {
    pub courses: Vec<AstronCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct CaktoCoursesCache {
    pub courses: Vec<CaktoCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct CaktoMembersCoursesCache {
    pub courses: Vec<CaktoMembersCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct CurseducaCoursesCache {
    pub courses: Vec<CurseducaCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct MedcofCoursesCache {
    pub courses: Vec<MedcofCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct PluralsightCoursesCache {
    pub courses: Vec<PluralsightCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct WondriumCoursesCache {
    pub courses: Vec<WondriumCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct MasterClassCoursesCache {
    pub courses: Vec<MasterClassCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct ThinkificCoursesCache {
    pub courses: Vec<ThinkificCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct RocketseatCoursesCache {
    pub courses: Vec<RocketseatCourse>,
    pub fetched_at: std::time::Instant,
}

pub struct CoursesState {
    pub hotmart_session: Arc<tokio::sync::Mutex<Option<HotmartSession>>>,
    pub active_downloads: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
    pub courses_cache: Arc<tokio::sync::Mutex<Option<CoursesCache>>>,
    pub session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub udemy_session: Arc<tokio::sync::Mutex<Option<UdemySession>>>,
    pub udemy_courses_cache: Arc<tokio::sync::Mutex<Option<UdemyCoursesCache>>>,
    pub udemy_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub udemy_api_webview: Arc<tokio::sync::Mutex<Option<tauri::WebviewWindow>>>,
    pub udemy_api_result: Arc<std::sync::Mutex<Option<String>>>,
    pub kiwify_session: Arc<tokio::sync::Mutex<Option<crate::platforms::kiwify::api::KiwifySession>>>,
    pub kiwify_courses_cache: Arc<tokio::sync::Mutex<Option<KiwifyCoursesCache>>>,
    pub kiwify_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub greenn_session: Arc<tokio::sync::Mutex<Option<crate::platforms::greennclub::api::GreennSession>>>,
    pub greenn_courses_cache: Arc<tokio::sync::Mutex<Option<GreennCoursesCache>>>,
    pub greenn_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub voomp_session: Arc<tokio::sync::Mutex<Option<crate::platforms::voompplay::api::VoompSession>>>,
    pub voomp_courses_cache: Arc<tokio::sync::Mutex<Option<VoompCoursesCache>>>,
    pub voomp_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub entregadigital_session: Arc<tokio::sync::Mutex<Option<crate::platforms::entregadigital::api::EntregaDigitalSession>>>,
    pub entregadigital_courses_cache: Arc<tokio::sync::Mutex<Option<EntregaDigitalCoursesCache>>>,
    pub entregadigital_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub alpaclass_session: Arc<tokio::sync::Mutex<Option<crate::platforms::alpaclass::api::AlpaclassSession>>>,
    pub alpaclass_courses_cache: Arc<tokio::sync::Mutex<Option<AlpaclassCoursesCache>>>,
    pub alpaclass_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub themembers_session: Arc<tokio::sync::Mutex<Option<crate::platforms::themembers::api::TheMembersSession>>>,
    pub themembers_courses_cache: Arc<tokio::sync::Mutex<Option<TheMembersCoursesCache>>>,
    pub themembers_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub gumroad_session: Arc<tokio::sync::Mutex<Option<crate::platforms::gumroad::api::GumroadSession>>>,
    pub gumroad_courses_cache: Arc<tokio::sync::Mutex<Option<GumroadCoursesCache>>>,
    pub gumroad_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub kirvano_session: Arc<tokio::sync::Mutex<Option<crate::platforms::kirvano::api::KirvanoSession>>>,
    pub kirvano_courses_cache: Arc<tokio::sync::Mutex<Option<KirvanoCoursesCache>>>,
    pub kirvano_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub dsa_session: Arc<tokio::sync::Mutex<Option<crate::platforms::datascienceacademy::api::DsaSession>>>,
    pub dsa_courses_cache: Arc<tokio::sync::Mutex<Option<DsaCoursesCache>>>,
    pub dsa_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub medcel_session: Arc<tokio::sync::Mutex<Option<crate::platforms::medcel::api::MedcelSession>>>,
    pub medcel_courses_cache: Arc<tokio::sync::Mutex<Option<MedcelCoursesCache>>>,
    pub medcel_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub afya_session: Arc<tokio::sync::Mutex<Option<crate::platforms::afyainternato::api::AfyaSession>>>,
    pub afya_courses_cache: Arc<tokio::sync::Mutex<Option<AfyaCoursesCache>>>,
    pub afya_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub medway_session: Arc<tokio::sync::Mutex<Option<crate::platforms::medway::api::MedwaySession>>>,
    pub medway_courses_cache: Arc<tokio::sync::Mutex<Option<MedwayCoursesCache>>>,
    pub medway_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub estrategia_concursos_session: Arc<tokio::sync::Mutex<Option<crate::platforms::estrategia_concursos::api::EstrategiaConcursosSession>>>,
    pub estrategia_concursos_courses_cache: Arc<tokio::sync::Mutex<Option<EstrategiaConcursosCoursesCache>>>,
    pub estrategia_concursos_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub estrategia_ldi_session: Arc<tokio::sync::Mutex<Option<crate::platforms::estrategia_ldi::api::EstrategiaLdiSession>>>,
    pub estrategia_ldi_courses_cache: Arc<tokio::sync::Mutex<Option<EstrategiaLdiCoursesCache>>>,
    pub estrategia_ldi_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub estrategia_militares_session: Arc<tokio::sync::Mutex<Option<crate::platforms::estrategia_militares::api::EstrategiaMilitaresSession>>>,
    pub estrategia_militares_courses_cache: Arc<tokio::sync::Mutex<Option<EstrategiaMilitaresCoursesCache>>>,
    pub estrategia_militares_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub grancursos_session: Arc<tokio::sync::Mutex<Option<crate::platforms::grancursos::api::GranCursosSession>>>,
    pub grancursos_courses_cache: Arc<tokio::sync::Mutex<Option<GranCursosCoursesCache>>>,
    pub grancursos_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub skool_session: Arc<tokio::sync::Mutex<Option<crate::platforms::skool::api::SkoolSession>>>,
    pub skool_courses_cache: Arc<tokio::sync::Mutex<Option<SkoolCoursesCache>>>,
    pub skool_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub teachable_session: Arc<tokio::sync::Mutex<Option<crate::platforms::teachable::api::TeachableSession>>>,
    pub teachable_courses_cache: Arc<tokio::sync::Mutex<Option<TeachableCoursesCache>>>,
    pub teachable_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub kajabi_session: Arc<tokio::sync::Mutex<Option<crate::platforms::kajabi::api::KajabiSession>>>,
    pub kajabi_courses_cache: Arc<tokio::sync::Mutex<Option<KajabiCoursesCache>>>,
    pub kajabi_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub fluency_session: Arc<tokio::sync::Mutex<Option<crate::platforms::fluencyacademy::api::FluencySession>>>,
    pub fluency_courses_cache: Arc<tokio::sync::Mutex<Option<FluencyCoursesCache>>>,
    pub fluency_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub nutror_session: Arc<tokio::sync::Mutex<Option<crate::platforms::eduzznutror::api::NutrorSession>>>,
    pub nutror_courses_cache: Arc<tokio::sync::Mutex<Option<NutrorCoursesCache>>>,
    pub nutror_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub cademi_session: Arc<tokio::sync::Mutex<Option<crate::platforms::cademi::api::CademiSession>>>,
    pub cademi_courses_cache: Arc<tokio::sync::Mutex<Option<CademiCoursesCache>>>,
    pub cademi_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub memberkit_session: Arc<tokio::sync::Mutex<Option<crate::platforms::memberkit::api::MemberkitSession>>>,
    pub memberkit_courses_cache: Arc<tokio::sync::Mutex<Option<MemberkitCoursesCache>>>,
    pub memberkit_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub areademembros_session: Arc<tokio::sync::Mutex<Option<crate::platforms::areademembros::api::AreaDeMembrosSession>>>,
    pub areademembros_courses_cache: Arc<tokio::sync::Mutex<Option<AreaDeMembrosCoursesCache>>>,
    pub areademembros_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub astron_session: Arc<tokio::sync::Mutex<Option<crate::platforms::astronmembers::api::AstronSession>>>,
    pub astron_courses_cache: Arc<tokio::sync::Mutex<Option<AstronCoursesCache>>>,
    pub astron_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub cakto_session: Arc<tokio::sync::Mutex<Option<crate::platforms::cakto::api::CaktoSession>>>,
    pub cakto_courses_cache: Arc<tokio::sync::Mutex<Option<CaktoCoursesCache>>>,
    pub cakto_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub caktomembers_session: Arc<tokio::sync::Mutex<Option<crate::platforms::caktomembers::api::CaktoMembersSession>>>,
    pub caktomembers_courses_cache: Arc<tokio::sync::Mutex<Option<CaktoMembersCoursesCache>>>,
    pub caktomembers_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub curseduca_session: Arc<tokio::sync::Mutex<Option<crate::platforms::curseduca::api::CurseducaSession>>>,
    pub curseduca_courses_cache: Arc<tokio::sync::Mutex<Option<CurseducaCoursesCache>>>,
    pub curseduca_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub medcof_session: Arc<tokio::sync::Mutex<Option<crate::platforms::medcof::api::MedcofSession>>>,
    pub medcof_courses_cache: Arc<tokio::sync::Mutex<Option<MedcofCoursesCache>>>,
    pub medcof_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub thinkific_session: Arc<tokio::sync::Mutex<Option<crate::platforms::thinkific::api::ThinkificSession>>>,
    pub thinkific_courses_cache: Arc<tokio::sync::Mutex<Option<ThinkificCoursesCache>>>,
    pub thinkific_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub rocketseat_session: Arc<tokio::sync::Mutex<Option<crate::platforms::rocketseat::api::RocketseatSession>>>,
    pub rocketseat_courses_cache: Arc<tokio::sync::Mutex<Option<RocketseatCoursesCache>>>,
    pub rocketseat_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub pluralsight_session: Arc<tokio::sync::Mutex<Option<crate::platforms::pluralsight::api::PluralsightSession>>>,
    pub pluralsight_courses_cache: Arc<tokio::sync::Mutex<Option<PluralsightCoursesCache>>>,
    pub pluralsight_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub wondrium_session: Arc<tokio::sync::Mutex<Option<crate::platforms::greatcourses::api::WondriumSession>>>,
    pub wondrium_courses_cache: Arc<tokio::sync::Mutex<Option<WondriumCoursesCache>>>,
    pub wondrium_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    pub masterclass_session: Arc<tokio::sync::Mutex<Option<crate::platforms::masterclass::api::MasterClassSession>>>,
    pub masterclass_courses_cache: Arc<tokio::sync::Mutex<Option<MasterClassCoursesCache>>>,
    pub masterclass_session_validated_at: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
}

impl Default for CoursesState {
    fn default() -> Self {
        Self {
            hotmart_session: Arc::new(tokio::sync::Mutex::new(None)),
            active_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            udemy_session: Arc::new(tokio::sync::Mutex::new(None)),
            udemy_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            udemy_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            udemy_api_webview: Arc::new(tokio::sync::Mutex::new(None)),
            udemy_api_result: Arc::new(std::sync::Mutex::new(None)),
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
            estrategia_concursos_session: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_concursos_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_concursos_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_ldi_session: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_ldi_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_ldi_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_militares_session: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_militares_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            estrategia_militares_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            grancursos_session: Arc::new(tokio::sync::Mutex::new(None)),
            grancursos_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            grancursos_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            skool_session: Arc::new(tokio::sync::Mutex::new(None)),
            skool_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            skool_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            teachable_session: Arc::new(tokio::sync::Mutex::new(None)),
            teachable_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            teachable_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            kajabi_session: Arc::new(tokio::sync::Mutex::new(None)),
            kajabi_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            kajabi_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            fluency_session: Arc::new(tokio::sync::Mutex::new(None)),
            fluency_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            fluency_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            nutror_session: Arc::new(tokio::sync::Mutex::new(None)),
            nutror_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            nutror_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            cademi_session: Arc::new(tokio::sync::Mutex::new(None)),
            cademi_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            cademi_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            memberkit_session: Arc::new(tokio::sync::Mutex::new(None)),
            memberkit_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            memberkit_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            areademembros_session: Arc::new(tokio::sync::Mutex::new(None)),
            areademembros_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            areademembros_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            astron_session: Arc::new(tokio::sync::Mutex::new(None)),
            astron_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            astron_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            cakto_session: Arc::new(tokio::sync::Mutex::new(None)),
            cakto_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            cakto_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            caktomembers_session: Arc::new(tokio::sync::Mutex::new(None)),
            caktomembers_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            caktomembers_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            curseduca_session: Arc::new(tokio::sync::Mutex::new(None)),
            curseduca_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            curseduca_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            medcof_session: Arc::new(tokio::sync::Mutex::new(None)),
            medcof_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            medcof_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            thinkific_session: Arc::new(tokio::sync::Mutex::new(None)),
            thinkific_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            thinkific_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            rocketseat_session: Arc::new(tokio::sync::Mutex::new(None)),
            rocketseat_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            rocketseat_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            pluralsight_session: Arc::new(tokio::sync::Mutex::new(None)),
            pluralsight_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            pluralsight_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            wondrium_session: Arc::new(tokio::sync::Mutex::new(None)),
            wondrium_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            wondrium_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
            masterclass_session: Arc::new(tokio::sync::Mutex::new(None)),
            masterclass_courses_cache: Arc::new(tokio::sync::Mutex::new(None)),
            masterclass_session_validated_at: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}
