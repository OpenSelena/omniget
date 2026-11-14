pub use libreget_core::platforms::traits;

pub mod afyainternato;
pub mod alpaclass;
pub mod areademembros;
pub mod astronmembers;
pub mod cademi;
pub mod cakto;
pub mod caktomembers;
pub mod curseduca;
pub mod datascienceacademy;
pub mod eduzznutror;
pub mod entregadigital;
pub mod estrategia_concursos;
pub mod estrategia_ldi;
pub mod estrategia_militares;
pub mod fluencyacademy;
pub mod grancursos;
pub mod greatcourses;
pub mod greennclub;
pub mod gumroad;
pub mod hotmart;
pub mod kajabi;
pub mod kirvano;
pub mod kiwify;
pub mod masterclass;
pub mod matematicaprapassar;
pub mod medcel;
pub mod medcof;
pub mod medway;
pub mod memberkit;
pub mod pluralsight;
pub mod rocketseat;
pub mod skool;
pub mod teachable;
pub mod themembers;
pub mod thinkific;
pub mod udemy;
pub mod voompplay;

pub fn session_file(filename: &str) -> anyhow::Result<std::path::PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Could not find app data directory"))?;
    let target = data_dir.join("libreget").join(filename);
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Ok(target)
}
