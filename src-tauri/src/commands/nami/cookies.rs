#![allow(clippy::all)]
#![allow(warnings)]

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DetectedBrowser {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub profile_dir: String,
    pub available: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserIdentity {
    pub id: String,
    pub name: String,
    pub engine: String,
}

struct BrowserCandidateDef {
    id: &'static str,
    name: &'static str,
    engine: &'static str,
    win_dirs: &'static [&'static str],
    mac_dirs: &'static [&'static str],
    linux_dirs: &'static [&'static str],
}

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

pub struct ResolvedCookies {
    pub sandbox: Option<SafeCookieSandbox>,
    pub browser_arg: Option<String>,
}

impl ResolvedCookies {
    pub fn empty() -> Self {
        Self {
            sandbox: None,
            browser_arg: None,
        }
    }

    pub fn cookie_file_path(&self) -> Option<&Path> {
        self.sandbox.as_ref().map(|s| s.path())
    }

    pub fn browser_arg(&self) -> Option<&str> {
        self.browser_arg.as_deref()
    }

    pub fn has_cookies(&self) -> bool {
        self.sandbox.is_some() || self.browser_arg.is_some()
    }
}

pub fn parse_windows_default_browser_prog_id(raw_output: &str) -> Option<String> {
    if raw_output.is_empty() {
        return None;
    }
    for line in raw_output.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("ProgId") {
            let after = trimmed[pos + "ProgId".len()..].trim();
            let parts: Vec<&str> = after.split_whitespace().collect();
            if parts.len() == 2 && parts[0].starts_with("REG_") {
                let prog_id = parts[1].trim();
                if !prog_id.is_empty()
                    && prog_id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
                {
                    return Some(prog_id.to_string());
                }
            }
        }
    }
    None
}

pub fn map_prog_id_to_browser_id(prog_id: &str, command_output: Option<&str>) -> BrowserIdentity {
    let p_lower = prog_id.to_ascii_lowercase();
    let cmd = command_output.unwrap_or("").to_ascii_lowercase();

    if p_lower.contains("helium") || cmd.contains("helium") {
        return BrowserIdentity {
            id: "helium".to_string(),
            name: "Helium".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if p_lower.starts_with("chromehtml") || p_lower.contains("chrome") || cmd.contains("chrome.exe")
    {
        return BrowserIdentity {
            id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if p_lower.starts_with("msedge")
        || p_lower.contains("edge")
        || p_lower.starts_with("appxq0fevzme2pys62n3e0fbqsw5cj08ege9")
        || cmd.contains("msedge.exe")
    {
        return BrowserIdentity {
            id: "edge".to_string(),
            name: "Microsoft Edge".to_string(),
            engine: "edge".to_string(),
        };
    }
    if p_lower.starts_with("brave") || p_lower.contains("brave") || cmd.contains("brave.exe") {
        return BrowserIdentity {
            id: "brave".to_string(),
            name: "Brave Browser".to_string(),
            engine: "brave".to_string(),
        };
    }
    if p_lower.starts_with("firefox") || p_lower.contains("firefox") || cmd.contains("firefox.exe")
    {
        return BrowserIdentity {
            id: "firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if p_lower.contains("zen") || cmd.contains("zen") {
        return BrowserIdentity {
            id: "zen".to_string(),
            name: "Zen Browser".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if p_lower.starts_with("opera") || p_lower.contains("opera") || cmd.contains("opera.exe") {
        return BrowserIdentity {
            id: "opera".to_string(),
            name: "Opera".to_string(),
            engine: "opera".to_string(),
        };
    }
    if p_lower.starts_with("vivaldi") || p_lower.contains("vivaldi") || cmd.contains("vivaldi.exe")
    {
        return BrowserIdentity {
            id: "vivaldi".to_string(),
            name: "Vivaldi".to_string(),
            engine: "vivaldi".to_string(),
        };
    }

    BrowserIdentity {
        id: if p_lower.is_empty() {
            "chromium".to_string()
        } else {
            p_lower
        },
        name: if prog_id.is_empty() {
            "Chromium".to_string()
        } else {
            prog_id.to_string()
        },
        engine: "chromium".to_string(),
    }
}

pub fn parse_mac_default_browser_bundle_id(raw_output: &str) -> Option<String> {
    if raw_output.is_empty() {
        return None;
    }
    let blocks: Vec<&str> = raw_output.split(|c| c == '{' || c == '}').collect();
    let mut http_bundle: Option<String> = None;

    for block in blocks {
        let is_https = block.contains("LSHandlerURLScheme = https")
            || block.contains("LSHandlerURLScheme = \"https\"");
        let is_http = block.contains("LSHandlerURLScheme = http")
            || block.contains("LSHandlerURLScheme = \"http\"");
        if !is_https && !is_http {
            continue;
        }

        let role_section = if let Some(pos) = block.find("LSHandlerRoleAll") {
            Some(&block[pos..])
        } else if let Some(pos) = block.find("LSHandlerRoleViewer") {
            Some(&block[pos..])
        } else {
            None
        };

        if let Some(role_str) = role_section {
            if let Some(eq_pos) = role_str.find('=') {
                let rest = &role_str[eq_pos + 1..];
                let semi_pos = rest.find(';').unwrap_or(rest.len());
                let bundle = rest[..semi_pos]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim();
                if !bundle.is_empty() {
                    if is_https {
                        return Some(bundle.to_string());
                    }
                    if http_bundle.is_none() {
                        http_bundle = Some(bundle.to_string());
                    }
                }
            }
        }
    }
    http_bundle
}

pub fn map_bundle_id_to_browser_id(bundle_id: &str) -> BrowserIdentity {
    let lower = bundle_id.to_ascii_lowercase();
    if lower.contains("helium") {
        return BrowserIdentity {
            id: "helium".to_string(),
            name: "Helium".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if lower.contains("chrome") {
        return BrowserIdentity {
            id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if lower.contains("edge") {
        return BrowserIdentity {
            id: "edge".to_string(),
            name: "Microsoft Edge".to_string(),
            engine: "edge".to_string(),
        };
    }
    if lower.contains("brave") {
        return BrowserIdentity {
            id: "brave".to_string(),
            name: "Brave Browser".to_string(),
            engine: "brave".to_string(),
        };
    }
    if lower.contains("firefox") {
        return BrowserIdentity {
            id: "firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if lower.contains("zen") {
        return BrowserIdentity {
            id: "zen".to_string(),
            name: "Zen Browser".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if lower.contains("opera") {
        return BrowserIdentity {
            id: "opera".to_string(),
            name: "Opera".to_string(),
            engine: "opera".to_string(),
        };
    }
    if lower.contains("vivaldi") {
        return BrowserIdentity {
            id: "vivaldi".to_string(),
            name: "Vivaldi".to_string(),
            engine: "vivaldi".to_string(),
        };
    }
    if lower.contains("safari") {
        return BrowserIdentity {
            id: "safari".to_string(),
            name: "Safari".to_string(),
            engine: "safari".to_string(),
        };
    }
    BrowserIdentity {
        id: lower,
        name: bundle_id.to_string(),
        engine: "chromium".to_string(),
    }
}

pub fn parse_linux_default_browser_desktop(raw_output: &str) -> Option<String> {
    if raw_output.is_empty() {
        return None;
    }
    for line in raw_output.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("ERROR:") && !trimmed.contains("not found") {
            if trimmed.ends_with(".desktop")
                || trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
            {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub fn map_linux_desktop_to_browser_id(desktop_file: &str) -> BrowserIdentity {
    let lower = desktop_file.to_ascii_lowercase();
    if lower.contains("helium") {
        return BrowserIdentity {
            id: "helium".to_string(),
            name: "Helium".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if lower.contains("chrome") {
        return BrowserIdentity {
            id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            engine: "chrome".to_string(),
        };
    }
    if lower.contains("edge") {
        return BrowserIdentity {
            id: "edge".to_string(),
            name: "Microsoft Edge".to_string(),
            engine: "edge".to_string(),
        };
    }
    if lower.contains("brave") {
        return BrowserIdentity {
            id: "brave".to_string(),
            name: "Brave Browser".to_string(),
            engine: "brave".to_string(),
        };
    }
    if lower.contains("firefox") {
        return BrowserIdentity {
            id: "firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if lower.contains("zen") {
        return BrowserIdentity {
            id: "zen".to_string(),
            name: "Zen Browser".to_string(),
            engine: "firefox".to_string(),
        };
    }
    if lower.contains("opera") {
        return BrowserIdentity {
            id: "opera".to_string(),
            name: "Opera".to_string(),
            engine: "opera".to_string(),
        };
    }
    if lower.contains("vivaldi") {
        return BrowserIdentity {
            id: "vivaldi".to_string(),
            name: "Vivaldi".to_string(),
            engine: "vivaldi".to_string(),
        };
    }
    BrowserIdentity {
        id: lower.trim_end_matches(".desktop").to_string(),
        name: desktop_file.to_string(),
        engine: "chromium".to_string(),
    }
}

pub fn detect_system_default_browser() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice",
                "/v",
                "ProgId",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if let Some(prog_id) = parse_windows_default_browser_prog_id(&out) {
            let cmd_out = Command::new("reg")
                .args(["query", &format!("HKCR\\{}\\shell\\open\\command", prog_id)])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok());
            return Some(map_prog_id_to_browser_id(&prog_id, cmd_out.as_deref()).id);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let out = Command::new("defaults")
            .args([
                "read",
                "com.apple.LaunchServices/com.apple.launchservices.secure",
                "LSHandlers",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if let Some(bundle_id) = parse_mac_default_browser_bundle_id(&out) {
            return Some(map_bundle_id_to_browser_id(&bundle_id).id);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let out = Command::new("xdg-settings")
            .args(["get", "default-web-browser"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if let Some(desktop) = parse_linux_default_browser_desktop(&out) {
            return Some(map_linux_desktop_to_browser_id(&desktop).id);
        }

        let mime_out = Command::new("xdg-mime")
            .args(["query", "default", "x-scheme-handler/https"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if let Some(desktop) = parse_linux_default_browser_desktop(&mime_out) {
            return Some(map_linux_desktop_to_browser_id(&desktop).id);
        }
    }

    None
}

pub fn list_detected_browsers() -> Vec<DetectedBrowser> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let default_id = detect_system_default_browser();

    let candidates: [BrowserCandidateDef; 9] = [
        BrowserCandidateDef {
            id: "chrome",
            name: "Google Chrome",
            engine: "chrome",
            win_dirs: &["Google/Chrome/User Data"],
            mac_dirs: &["Library/Application Support/Google/Chrome"],
            linux_dirs: &[
                ".config/google-chrome",
                ".var/app/com.google.Chrome/config/google-chrome",
                ".config/chromium",
                ".var/app/org.chromium.Chromium/config/chromium",
                ".config/google-chrome-for-testing",
            ],
        },
        BrowserCandidateDef {
            id: "edge",
            name: "Microsoft Edge",
            engine: "edge",
            win_dirs: &["Microsoft/Edge/User Data"],
            mac_dirs: &["Library/Application Support/Microsoft Edge"],
            linux_dirs: &[
                ".config/microsoft-edge",
                ".var/app/com.microsoft.Edge/config/microsoft-edge",
            ],
        },
        BrowserCandidateDef {
            id: "brave",
            name: "Brave Browser",
            engine: "brave",
            win_dirs: &["BraveSoftware/Brave-Browser/User Data"],
            mac_dirs: &["Library/Application Support/BraveSoftware/Brave-Browser"],
            linux_dirs: &[
                ".config/BraveSoftware/Brave-Browser",
                ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
            ],
        },
        BrowserCandidateDef {
            id: "firefox",
            name: "Mozilla Firefox",
            engine: "firefox",
            win_dirs: &["Mozilla/Firefox/Profiles", "Mozilla/Firefox"],
            mac_dirs: &[
                "Library/Application Support/Firefox/Profiles",
                "Library/Application Support/Firefox",
            ],
            linux_dirs: &[
                ".mozilla/firefox",
                ".config/mozilla/firefox",
                ".var/app/org.mozilla.firefox/.mozilla/firefox",
            ],
        },
        BrowserCandidateDef {
            id: "zen",
            name: "Zen Browser",
            engine: "firefox",
            win_dirs: &["zen/Profiles", "zen"],
            mac_dirs: &[
                "Library/Application Support/zen/Profiles",
                "Library/Application Support/zen",
            ],
            linux_dirs: &[".zen", ".var/app/app.zen_browser.zen/.zen"],
        },
        BrowserCandidateDef {
            id: "helium",
            name: "Helium",
            engine: "chrome",
            win_dirs: &["imput/Helium/User Data"],
            mac_dirs: &["Library/Application Support/net.imput.Helium"],
            linux_dirs: &[".config/helium", ".var/app/net.imput.Helium/config/helium"],
        },
        BrowserCandidateDef {
            id: "opera",
            name: "Opera",
            engine: "opera",
            win_dirs: &["Opera Software/Opera Stable"],
            mac_dirs: &["Library/Application Support/com.operasoftware.Opera"],
            linux_dirs: &[".config/opera", ".var/app/com.opera.Opera/config/opera"],
        },
        BrowserCandidateDef {
            id: "vivaldi",
            name: "Vivaldi",
            engine: "vivaldi",
            win_dirs: &["Vivaldi/User Data"],
            mac_dirs: &["Library/Application Support/Vivaldi"],
            linux_dirs: &[
                ".config/vivaldi",
                ".var/app/com.vivaldi.Vivaldi/config/vivaldi",
            ],
        },
        BrowserCandidateDef {
            id: "librewolf",
            name: "LibreWolf",
            engine: "firefox",
            win_dirs: &["librewolf/Profiles"],
            mac_dirs: &["Library/Application Support/librewolf/Profiles"],
            linux_dirs: &[
                ".librewolf",
                ".var/app/io.gitlab.librewolf-community/.librewolf",
            ],
        },
    ];

    let mut result = Vec::new();

    for c in &candidates {
        let (dirs, prefix): (&[&str], PathBuf) = if cfg!(target_os = "windows") {
            let app_data = std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData").join("Local"));
            (c.win_dirs, app_data)
        } else if cfg!(target_os = "macos") {
            (c.mac_dirs, home.clone())
        } else {
            (c.linux_dirs, home.clone())
        };

        let mut found_dir = PathBuf::new();
        let mut available = false;

        for d in dirs {
            let check = prefix.join(d);
            if check.exists() {
                found_dir = check;
                available = true;
                break;
            }
        }

        if !available && !dirs.is_empty() {
            found_dir = prefix.join(dirs[0]);
        }

        let is_default = default_id.as_deref() == Some(c.id);

        result.push(DetectedBrowser {
            id: c.id.to_string(),
            name: c.name.to_string(),
            engine: c.engine.to_string(),
            profile_dir: found_dir.to_string_lossy().to_string(),
            available,
            is_default,
        });
    }

    result
}

pub fn find_gecko_profile_dir(base_dir: &Path) -> Option<PathBuf> {
    if !base_dir.exists() {
        return None;
    }

    if base_dir.join("cookies.sqlite").exists() {
        return Some(base_dir.to_path_buf());
    }

    let ini_path = base_dir.join("profiles.ini");
    if ini_path.exists() {
        if let Ok(content) = fs::read_to_string(&ini_path) {
            let mut default_path: Option<String> = None;
            let mut first_path: Option<String> = None;
            let mut current_path: Option<String> = None;
            let mut current_is_default = false;

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    if let Some(cp) = current_path.take() {
                        if current_is_default && default_path.is_none() {
                            default_path = Some(cp.clone());
                        }
                        if first_path.is_none() {
                            first_path = Some(cp);
                        }
                    }
                    current_is_default = false;
                } else if let Some(p) = trimmed.strip_prefix("Path=") {
                    current_path = Some(p.trim().to_string());
                } else if trimmed == "Default=1" {
                    current_is_default = true;
                }
            }
            if let Some(cp) = current_path {
                if current_is_default && default_path.is_none() {
                    default_path = Some(cp.clone());
                }
                if first_path.is_none() {
                    first_path = Some(cp);
                }
            }

            let chosen_rel = default_path.or(first_path);
            if let Some(rel) = chosen_rel {
                let full = if Path::new(&rel).is_absolute() {
                    PathBuf::from(rel)
                } else {
                    base_dir.join(rel)
                };
                if full.join("cookies.sqlite").exists() {
                    return Some(full);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(base_dir) {
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    subdirs.push(entry.path());
                }
            }
        }
        subdirs.sort_by(|a, b| {
            let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let a_score = if a_name.contains("default-release") {
                2
            } else if a_name.contains("default") {
                1
            } else {
                0
            };
            let b_score = if b_name.contains("default-release") {
                2
            } else if b_name.contains("default") {
                1
            } else {
                0
            };
            b_score.cmp(&a_score)
        });

        for dir in subdirs {
            if dir.join("cookies.sqlite").exists() {
                return Some(dir);
            }
        }
    }

    None
}

pub fn extract_gecko_cookies_to_netscape(
    profile_dir: &Path,
    platform_domains: &[&str],
) -> Option<String> {
    let sqlite_file = profile_dir.join("cookies.sqlite");
    if !sqlite_file.exists() {
        return None;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let rand_suffix: u32 = rand::random();
    let temp_db = std::env::temp_dir().join(format!(
        "omniget_gecko_{}_{:x}.sqlite",
        timestamp, rand_suffix
    ));
    let temp_wal = std::env::temp_dir().join(format!(
        "omniget_gecko_{}_{:x}.sqlite-wal",
        timestamp, rand_suffix
    ));
    let src_wal = profile_dir.join("cookies.sqlite-wal");

    if fs::copy(&sqlite_file, &temp_db).is_err() {
        return None;
    }
    if src_wal.exists() {
        let _ = fs::copy(&src_wal, &temp_wal);
    }

    let result = (|| -> Result<String, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            &temp_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let mut query =
            "SELECT host, path, isSecure, expiry, name, value, isHttpOnly FROM moz_cookies"
                .to_string();
        if !platform_domains.is_empty() {
            let clauses: Vec<String> = platform_domains
                .iter()
                .map(|_| "host LIKE ?".to_string())
                .collect();
            query.push_str(" WHERE ");
            query.push_str(&clauses.join(" OR "));
        }

        let mut stmt = conn.prepare(&query)?;
        let mut rows = if platform_domains.is_empty() {
            stmt.query([])?
        } else {
            let params: Vec<&dyn rusqlite::ToSql> = platform_domains
                .iter()
                .map(|d| d as &dyn rusqlite::ToSql)
                .collect();
            stmt.query(params.as_slice())?
        };

        let mut lines = vec!["# Netscape HTTP Cookie File".to_string()];
        let mut count = 0;

        while let Some(row) = rows.next()? {
            let host: String = row.get(0).unwrap_or_default();
            let path: String = row.get(1).unwrap_or_else(|_| "/".to_string());
            let is_secure: i64 = row.get(2).unwrap_or(0);
            let mut expiry: i64 = row.get(3).unwrap_or(0);
            let name: String = row.get(4).unwrap_or_default();
            let value: String = row.get(5).unwrap_or_default();
            let is_http_only: i64 = row.get(6).unwrap_or(0);

            if host.is_empty() || name.is_empty() {
                continue;
            }

            if expiry > 100_000_000_000_000 {
                expiry /= 1_000_000;
            } else if expiry > 100_000_000_000 {
                expiry /= 1_000;
            }

            let domain = if is_http_only == 1 {
                format!("#HttpOnly_{}", host)
            } else {
                host.clone()
            };
            let sub = if host.starts_with('.') {
                "TRUE"
            } else {
                "FALSE"
            };
            let sec = if is_secure == 1 { "TRUE" } else { "FALSE" };

            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                domain, sub, path, sec, expiry, name, value
            ));
            count += 1;
        }

        if count == 0 {
            return Ok(String::new());
        }

        lines.push(String::new());
        Ok(lines.join("\n"))
    })();

    let _ = fs::remove_file(&temp_db);
    let _ = fs::remove_file(&temp_wal);

    match result {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

pub fn resolve_browser_cookie_arg(browser_id: &str, profile_dir: Option<&str>) -> Option<String> {
    let trimmed = browser_id.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower == "auto" {
        let def = detect_system_default_browser();
        return def.and_then(|id| resolve_browser_cookie_arg(&id, None));
    }
    if lower == "zen" {
        if let Some(prof) = profile_dir {
            let p = Path::new(prof);
            if let Some(gecko) = find_gecko_profile_dir(p) {
                return Some(format!("firefox:{}", gecko.display()));
            }
            return Some(format!("firefox:{}", prof));
        }
        let detected = list_detected_browsers();
        if let Some(zen) = detected.iter().find(|b| b.id == "zen" && b.available) {
            let p = Path::new(&zen.profile_dir);
            if let Some(gecko) = find_gecko_profile_dir(p) {
                return Some(format!("firefox:{}", gecko.display()));
            }
            return Some(format!("firefox:{}", zen.profile_dir));
        }
        return Some("firefox".to_string());
    }
    if lower == "helium" {
        if let Some(prof) = profile_dir {
            return Some(format!("chrome:{}", prof));
        }
        let detected = list_detected_browsers();
        if let Some(helium) = detected.iter().find(|b| b.id == "helium" && b.available) {
            return Some(format!("chrome:{}", helium.profile_dir));
        }
        return Some("chrome".to_string());
    }
    if lower == "firefox" {
        if let Some(prof) = profile_dir {
            let p = Path::new(prof);
            if let Some(gecko) = find_gecko_profile_dir(p) {
                return Some(format!("firefox:{}", gecko.display()));
            }
            return Some(format!("firefox:{}", prof));
        }
        return Some("firefox".to_string());
    }
    Some(lower)
}

pub fn get_platform_cookie_domains(platform: &str) -> Vec<&'static str> {
    match platform.to_lowercase().as_str() {
        "instagram" => vec!["%instagram.com%"],
        "tiktok" => vec!["%tiktok.com%"],
        "facebook" => vec!["%facebook.com%", "%fb.com%"],
        "x" | "twitter" => vec!["%x.com%", "%twitter.com%"],
        _ => vec![],
    }
}

pub fn resolve_active_cookies(
    platform: &str,
    manual_path: Option<&str>,
    browser_selection: Option<&str>,
    app_data_dir: &Path,
) -> ResolvedCookies {
    let temp_base = app_data_dir.join("nami").join("temp");

    if let Some(candidate) = find_candidate_cookie(platform, manual_path, app_data_dir) {
        if let Ok(sb) = SafeCookieSandbox::create(&candidate, &temp_base) {
            return ResolvedCookies {
                sandbox: Some(sb),
                browser_arg: None,
            };
        }
    }

    let b_choice = browser_selection.unwrap_or("auto").trim();
    if b_choice.is_empty() || b_choice.eq_ignore_ascii_case("none") {
        return ResolvedCookies::empty();
    }

    let target_browser = if b_choice.eq_ignore_ascii_case("auto") {
        detect_system_default_browser().unwrap_or_else(|| "chrome".to_string())
    } else {
        b_choice.to_ascii_lowercase()
    };

    let browsers = list_detected_browsers();
    let matched_browser = browsers.iter().find(|b| b.id == target_browser);

    if target_browser == "firefox" || target_browser == "zen" || target_browser == "librewolf" {
        if let Some(b) = matched_browser {
            if b.available {
                let prof_path = Path::new(&b.profile_dir);
                if let Some(gecko_dir) = find_gecko_profile_dir(prof_path) {
                    let domains = get_platform_cookie_domains(platform);
                    if let Some(netscape_data) =
                        extract_gecko_cookies_to_netscape(&gecko_dir, &domains)
                    {
                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        let rand_suffix: u32 = rand::random();
                        let temp_file = temp_base.join(format!(
                            "browser_cookies_{}_{:x}.txt",
                            timestamp, rand_suffix
                        ));
                        let _ = fs::create_dir_all(&temp_base);
                        if fs::write(&temp_file, netscape_data).is_ok() {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let _ = fs::set_permissions(
                                    &temp_file,
                                    fs::Permissions::from_mode(0o600),
                                );
                            }
                            return ResolvedCookies {
                                sandbox: Some(SafeCookieSandbox {
                                    temp_path: temp_file,
                                }),
                                browser_arg: None,
                            };
                        }
                    }
                }
            }
        }
    }

    let browser_arg = resolve_browser_cookie_arg(
        &target_browser,
        matched_browser.map(|b| b.profile_dir.as_str()),
    );
    ResolvedCookies {
        sandbox: None,
        browser_arg,
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
        app_data_dir.join("nami").join("cookies"),
        app_data_dir.join("cookies"),
    ];

    if let Some(user_home) = dirs::home_dir() {
        candidate_dirs.push(user_home.join(".nami").join("cookies"));
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
        let temp_dir = std::env::temp_dir().join("nami_test_cookie");
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
        assert!(!temp_file_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_windows_default_browser_prog_id() {
        let chrome_raw = "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice\r\n    ProgId    REG_SZ    ChromeHTML\r\n";
        assert_eq!(
            parse_windows_default_browser_prog_id(chrome_raw),
            Some("ChromeHTML".to_string())
        );

        let helium_raw = "    ProgId    REG_SZ    HeliumHTM\r\n";
        assert_eq!(
            parse_windows_default_browser_prog_id(helium_raw),
            Some("HeliumHTM".to_string())
        );

        let firefox_raw = "    ProgId    REG_SZ    FirefoxURL-308046B0AF4A39CB\r\n";
        assert_eq!(
            parse_windows_default_browser_prog_id(firefox_raw),
            Some("FirefoxURL-308046B0AF4A39CB".to_string())
        );

        let injection_raw = "    ProgId    REG_SZ    ChromeHTML & calc.exe\r\n";
        assert_eq!(parse_windows_default_browser_prog_id(injection_raw), None);

        assert_eq!(parse_windows_default_browser_prog_id(""), None);
    }

    #[test]
    fn test_map_prog_id_to_browser_id() {
        let chrome = map_prog_id_to_browser_id("ChromeHTML", None);
        assert_eq!(chrome.id, "chrome");
        assert_eq!(chrome.name, "Google Chrome");

        let edge = map_prog_id_to_browser_id("MSEdgeHTM", None);
        assert_eq!(edge.id, "edge");

        let edge_uwp = map_prog_id_to_browser_id("AppXq0fevzme2pys62n3e0fbqsw5cj08ege9", None);
        assert_eq!(edge_uwp.id, "edge");

        let brave = map_prog_id_to_browser_id("BraveHTML", None);
        assert_eq!(brave.id, "brave");

        let firefox = map_prog_id_to_browser_id("FirefoxURL-308046B0AF4A39CB", None);
        assert_eq!(firefox.id, "firefox");

        let helium =
            map_prog_id_to_browser_id("CustomProgId", Some("\"C:\\imput\\Helium\\chrome.exe\" %1"));
        assert_eq!(helium.id, "helium");

        let zen = map_prog_id_to_browser_id("ZenURL", Some("\"C:\\Zen\\zen.exe\" %1"));
        assert_eq!(zen.id, "zen");
    }

    #[test]
    fn test_parse_mac_and_linux_default_browser() {
        let mac_output = r#"
        (
            {
                LSHandlerRoleAll = "com.google.chrome";
                LSHandlerURLScheme = https;
            }
        )
        "#;
        assert_eq!(
            parse_mac_default_browser_bundle_id(mac_output),
            Some("com.google.chrome".to_string())
        );
        let mac_browser = map_bundle_id_to_browser_id("com.google.chrome");
        assert_eq!(mac_browser.id, "chrome");

        let linux_output = "google-chrome.desktop\n";
        assert_eq!(
            parse_linux_default_browser_desktop(linux_output),
            Some("google-chrome.desktop".to_string())
        );
        let linux_browser = map_linux_desktop_to_browser_id("google-chrome.desktop");
        assert_eq!(linux_browser.id, "chrome");

        let zen_linux = map_linux_desktop_to_browser_id("app.zen_browser.zen.desktop");
        assert_eq!(zen_linux.id, "zen");
        assert_eq!(zen_linux.engine, "firefox");
    }

    #[test]
    fn test_resolve_browser_cookie_arg() {
        assert_eq!(
            resolve_browser_cookie_arg("chrome", None),
            Some("chrome".to_string())
        );
        assert_eq!(
            resolve_browser_cookie_arg("firefox", None),
            Some("firefox".to_string())
        );
        assert_eq!(resolve_browser_cookie_arg("none", None), None);
        assert_eq!(resolve_browser_cookie_arg("", None), None);

        let zen_arg = resolve_browser_cookie_arg("zen", Some("/home/user/.zen/profile.default"));
        assert!(zen_arg.unwrap().starts_with("firefox:"));
    }

    #[test]
    fn test_gecko_profile_detection_and_sqlite_cookie_extraction() {
        let temp_dir =
            std::env::temp_dir().join(format!("nami_gecko_test_{}", rand::random::<u32>()));
        let _ = fs::create_dir_all(&temp_dir);

        let profile_dir = temp_dir.join("profile.default-release");
        let _ = fs::create_dir_all(&profile_dir);

        let db_path = profile_dir.join("cookies.sqlite");
        {
            let conn = Connection::open(&db_path).expect("Create test sqlite db");
            conn.execute(
                "CREATE TABLE moz_cookies (
                    id INTEGER PRIMARY KEY,
                    host TEXT,
                    path TEXT,
                    isSecure INTEGER,
                    expiry INTEGER,
                    name TEXT,
                    value TEXT,
                    isHttpOnly INTEGER
                )",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO moz_cookies (host, path, isSecure, expiry, name, value, isHttpOnly)
                 VALUES ('.instagram.com', '/', 1, 1750000000, 'sessionid', 'mock_ig_session_123', 1)",
                [],
            ).unwrap();

            conn.execute(
                "INSERT INTO moz_cookies (host, path, isSecure, expiry, name, value, isHttpOnly)
                 VALUES ('.facebook.com', '/', 1, 1750000000, 'c_user', '10002345', 0)",
                [],
            )
            .unwrap();
        }

        let resolved_profile = find_gecko_profile_dir(&temp_dir);
        assert_eq!(resolved_profile, Some(profile_dir.clone()));

        let ig_cookies = extract_gecko_cookies_to_netscape(&profile_dir, &["%instagram.com%"]);
        assert!(ig_cookies.is_some());
        let content = ig_cookies.unwrap();
        assert!(content.contains("# Netscape HTTP Cookie File"));
        assert!(content.contains("#HttpOnly_.instagram.com"));
        assert!(content.contains("sessionid\tmock_ig_session_123"));
        assert!(!content.contains("facebook.com"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
