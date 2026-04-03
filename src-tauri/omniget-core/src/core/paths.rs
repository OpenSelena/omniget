pub fn app_data_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("OMNIGET_DATA_DIR") {
        return Some(std::path::PathBuf::from(dir));
    }

    let base = dirs::data_dir()?;
    let new_path = base.join("wtf.tonho.omniget");
    let old_path = base.join("omniget");

    if old_path.exists() && !new_path.join("plugins").exists() {
        let _ = std::fs::create_dir_all(&new_path);
        if let Ok(entries) = std::fs::read_dir(&old_path) {
            for entry in entries.flatten() {
                let dest = new_path.join(entry.file_name());
                if !dest.exists() {
                    let _ = copy_dir_recursive(&entry.path(), &dest);
                }
            }
        }
    }

    Some(new_path)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dest = dst.join(entry.file_name());
            copy_dir_recursive(&entry.path(), &dest)?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}
