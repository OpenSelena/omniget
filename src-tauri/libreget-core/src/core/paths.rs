pub fn app_data_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("LIBREGET_DATA_DIR") {
        return Some(std::path::PathBuf::from(dir));
    }

    let base = dirs::data_dir()?;
    let new_path = base.join("com.openselena.libreget");
    let _ = std::fs::create_dir_all(&new_path);
    Some(new_path)
}
