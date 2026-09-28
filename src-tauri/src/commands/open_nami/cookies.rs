#![allow(clippy::all)]
#![allow(warnings)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SafeCookieSandbox {
    pub temp_path: PathBuf,
}

impl SafeCookieSandbox {
    pub fn create(master_path: &Path, temp_base: &Path) -> Result<Self, String> {
        if !master_path.exists() {
            return Err(format!(
                "Master cookie file does not exist: {}",
                master_path.display()
            ));
        }

        fs::create_dir_all(temp_base)
            .map_err(|e| format!("Failed to create temp cookie directory: {}", e))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let rand_suffix: u32 = rand::random();
        let temp_filename = format!("nami_cookie_{}_{:x}.txt", timestamp, rand_suffix);
        let temp_path = temp_base.join(temp_filename);

        fs::copy(master_path, &temp_path).map_err(|e| {
            format!(
                "Failed to copy cookie file to sandbox {} -> {}: {}",
                master_path.display(),
                temp_path.display(),
                e
            )
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&temp_path, perms);
        }

        Ok(Self { temp_path })
    }

    pub fn path(&self) -> &Path {
        &self.temp_path
    }
}

impl Drop for SafeCookieSandbox {
    fn drop(&mut self) {
        if self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

pub fn find_candidate_cookie(
    platform: &str,
    custom_path: Option<&str>,
    app_data_dir: &Path,
) -> Option<PathBuf> {
    if let Some(custom) = custom_path {
        let p = PathBuf::from(custom.trim());
        if p.exists() && fs::metadata(&p).map(|m| m.len() > 10).unwrap_or(false) {
            return Some(p);
        }
    }

    let filenames = match platform.to_lowercase().as_str() {
        "instagram" => vec!["instagram.com_cookies.txt"],
        "tiktok" => vec!["tiktok.com_cookies.txt"],
        "facebook" => vec!["facebook.com_cookies.txt", "fb.com_cookies.txt"],
        "x" | "twitter" => vec!["x.com_cookies.txt", "twitter.com_cookies.txt"],
        _ => vec![],
    };

    let mut candidate_dirs: Vec<PathBuf> = vec![
        app_data_dir.join("open_nami").join("cookies"),
        app_data_dir.join("cookies"),
    ];

    if let Some(user_home) = dirs::home_dir() {
        candidate_dirs.push(user_home.join(".open-nami").join("cookies"));
        candidate_dirs.push(user_home.join("cookies"));
    }

    for dir in candidate_dirs {
        for fname in &filenames {
            let full = dir.join(fname);
            if full.exists() && fs::metadata(&full).map(|m| m.len() > 10).unwrap_or(false) {
                return Some(full);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_safe_cookie_sandbox_lifecycle() {
        let temp_dir = std::env::temp_dir().join("opennami_test_cookie");
        let _ = fs::create_dir_all(&temp_dir);

        let master = temp_dir.join("master_cookies.txt");
        let mut f = fs::File::create(&master).unwrap();
        writeln!(f, "# Netscape HTTP Cookie File").unwrap();
        drop(f);

        let sandbox_dir = temp_dir.join("sandbox");
        let temp_file_path;
        {
            let sandbox = SafeCookieSandbox::create(&master, &sandbox_dir).expect("Create sandbox");
            temp_file_path = sandbox.path().to_path_buf();
            assert!(temp_file_path.exists());
            assert_ne!(temp_file_path, master);
        }
        // After exiting block, sandbox is dropped and temp file is automatically cleaned up
        assert!(!temp_file_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
