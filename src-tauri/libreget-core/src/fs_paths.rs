use std::path::PathBuf;

pub trait AppPaths: Send + Sync {
    fn downloads_dir(&self) -> PathBuf;
    fn data_dir(&self) -> PathBuf;
    fn cache_dir(&self) -> PathBuf;
    fn bin_dir(&self) -> Option<PathBuf>;
}

pub struct DesktopPaths;

impl AppPaths for DesktopPaths {
    fn downloads_dir(&self) -> PathBuf {
        dirs::download_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    fn data_dir(&self) -> PathBuf {
        dirs::data_dir()
            .map(|d| {
                let p = d.join("libreget");
                if p.exists() {
                    return p;
                }
                let old = d.join("omniget");
                if old.exists() {
                    return old;
                }
                p
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn cache_dir(&self) -> PathBuf {
        dirs::cache_dir()
            .map(|d| {
                let p = d.join("libreget");
                if p.exists() {
                    return p;
                }
                let old = d.join("omniget");
                if old.exists() {
                    return old;
                }
                p
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn bin_dir(&self) -> Option<PathBuf> {
        dirs::data_dir().map(|d| {
            let p = d.join("libreget").join("bin");
            if p.exists() {
                return p;
            }
            let old = d.join("omniget").join("bin");
            if old.exists() {
                return old;
            }
            p
        })
    }
}
