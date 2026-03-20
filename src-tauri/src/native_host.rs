use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use anyhow::Context;
use serde::{Deserialize, Serialize};

pub const CHROME_HOST_NAME: &str = "wtf.tonho.omniget";
pub const CHROME_EXTENSION_ID: &str = "dkjelkhaaakffpghdfalobccaaipajip";

#[cfg(target_os = "windows")]
const HOST_COPY_NAME: &str = "omniget-native-host.exe";
#[cfg(not(target_os = "windows"))]
const HOST_COPY_NAME: &str = "omniget-native-host";
const HOST_BINARY_STEM: &str = "omniget-native-host";
const HOST_CONFIG_NAME: &str = "native-host-config.json";
const HOST_MANIFEST_NAME: &str = "wtf.tonho.omniget.json";

#[derive(Debug, Deserialize)]
struct NativeHostRequest {
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct NativeHostResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NativeHostConfig {
    app_path: String,
}

pub fn should_run_as_native_host() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().to_string()))
        .map(|stem| stem.eq_ignore_ascii_case(HOST_BINARY_STEM))
        .unwrap_or(false)
}

pub fn run_native_host() -> anyhow::Result<()> {
    let request = read_message()?;
    let response = handle_request(request);
    write_message(&response)?;
    Ok(())
}

pub fn ensure_registered() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    let integration_dir = chrome_integration_dir();
    fs::create_dir_all(&integration_dir)?;

    let host_exe = integration_dir.join(HOST_COPY_NAME);
    copy_host_exe(&current_exe, &host_exe)?;

    let config_path = integration_dir.join(HOST_CONFIG_NAME);
    write_host_config(&config_path, &current_exe)?;

    let manifest_path = chrome_manifest_path(&integration_dir)?;
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_host_manifest(&manifest_path, &host_exe)?;
    register_host_manifest(&manifest_path)?;

    Ok(())
}

fn chrome_integration_dir() -> PathBuf {
    crate::core::paths::app_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chrome-native-host")
}

fn copy_host_exe(source: &Path, dest: &Path) -> anyhow::Result<()> {
    if source != dest && should_copy_exe(source, dest) {
        fs::copy(source, dest)?;
        sync_host_permissions(source, dest)?;
    }
    Ok(())
}

fn write_host_config(config_path: &Path, app_path: &Path) -> anyhow::Result<()> {
    let config = NativeHostConfig {
        app_path: app_path.to_string_lossy().to_string(),
    };
    fs::write(config_path, serde_json::to_vec_pretty(&config)?)?;
    Ok(())
}

fn write_host_manifest(manifest_path: &Path, host_exe: &Path) -> anyhow::Result<()> {
    fs::write(manifest_path, serde_json::to_vec_pretty(&build_host_manifest(host_exe))?)?;
    Ok(())
}

fn build_host_manifest(host_exe: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": CHROME_HOST_NAME,
        "description": "OmniGet native host for Chrome",
        "path": host_exe.to_string_lossy().to_string(),
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", CHROME_EXTENSION_ID)
        ]
    })
}

#[cfg(target_os = "windows")]
fn chrome_manifest_path(integration_dir: &Path) -> anyhow::Result<PathBuf> {
    Ok(chrome_manifest_path_from_base(integration_dir))
}

#[cfg(target_os = "linux")]
fn chrome_manifest_path(_integration_dir: &Path) -> anyhow::Result<PathBuf> {
    let base_dir = dirs::config_dir()
        .context("Could not resolve the Linux config directory for Chrome native host registration")?;
    Ok(chrome_manifest_path_from_base(&base_dir))
}

#[cfg(target_os = "macos")]
fn chrome_manifest_path(_integration_dir: &Path) -> anyhow::Result<PathBuf> {
    let base_dir = dirs::data_dir().context(
        "Could not resolve the macOS Application Support directory for Chrome native host registration",
    )?;
    Ok(chrome_manifest_path_from_base(&base_dir))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn chrome_manifest_path(_integration_dir: &Path) -> anyhow::Result<PathBuf> {
    anyhow::bail!("Chrome native host registration is unsupported on this platform");
}

#[cfg(target_os = "windows")]
fn chrome_manifest_path_from_base(base: &Path) -> PathBuf {
    base.join(HOST_MANIFEST_NAME)
}

#[cfg(target_os = "linux")]
fn chrome_manifest_path_from_base(base: &Path) -> PathBuf {
    base.join("google-chrome")
        .join("NativeMessagingHosts")
        .join(HOST_MANIFEST_NAME)
}

#[cfg(target_os = "macos")]
fn chrome_manifest_path_from_base(base: &Path) -> PathBuf {
    base.join("Google")
        .join("Chrome")
        .join("NativeMessagingHosts")
        .join(HOST_MANIFEST_NAME)
}

#[cfg(target_os = "windows")]
fn register_host_manifest(manifest_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let status = std::process::Command::new("reg")
        .args([
            "add",
            &format!(
                r"HKCU\Software\Google\Chrome\NativeMessagingHosts\{}",
                CHROME_HOST_NAME
            ),
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            &manifest_path.to_string_lossy(),
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to register Chrome native host");
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn register_host_manifest(_manifest_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn sync_host_permissions(source: &Path, dest: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(source)?.permissions().mode();
    fs::set_permissions(dest, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_host_permissions(_source: &Path, _dest: &Path) -> anyhow::Result<()> {
    Ok(())
}

fn should_copy_exe(source: &Path, dest: &Path) -> bool {
    let Ok(src_meta) = fs::metadata(source) else {
        return true;
    };
    let Ok(dst_meta) = fs::metadata(dest) else {
        return true;
    };
    src_meta.len() != dst_meta.len()
}

fn handle_request(request: NativeHostRequest) -> NativeHostResponse {
    if request.kind != "enqueue" {
        return NativeHostResponse {
            ok: false,
            code: Some("INVALID_URL"),
            message: Some("Unsupported native host message".to_string()),
        };
    }

    if !crate::external_url::is_external_url(&request.url) {
        return NativeHostResponse {
            ok: false,
            code: Some("INVALID_URL"),
            message: Some("The requested URL is invalid".to_string()),
        };
    }

    match launch_omniget(&request.url) {
        Ok(()) => NativeHostResponse {
            ok: true,
            code: None,
            message: None,
        },
        Err(error) => NativeHostResponse {
            ok: false,
            code: Some("LAUNCH_FAILED"),
            message: Some(error.to_string()),
        },
    }
}

fn launch_omniget(url: &str) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    let config_path = current_exe
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(HOST_CONFIG_NAME);
    let config: NativeHostConfig = serde_json::from_slice(&fs::read(config_path)?)?;

    let mut command = std::process::Command::new(config.app_path);
    command.arg(url);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn()?;
    Ok(())
}

fn read_message() -> anyhow::Result<NativeHostRequest> {
    const MAX_MESSAGE_LENGTH: usize = 1_048_576; // 1 MB — Chrome's own limit

    let mut length_bytes = [0u8; 4];
    std::io::stdin().read_exact(&mut length_bytes)?;
    let length = u32::from_le_bytes(length_bytes) as usize;

    if length > MAX_MESSAGE_LENGTH {
        anyhow::bail!(
            "Native message too large ({length} bytes, max {MAX_MESSAGE_LENGTH})"
        );
    }

    let mut payload = vec![0u8; length];
    std::io::stdin().read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn write_message(response: &NativeHostResponse) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(response)?;
    let length = (payload.len() as u32).to_le_bytes();

    let mut stdout = std::io::stdout();
    stdout.write_all(&length)?;
    stdout.write_all(&payload)?;
    stdout.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_copy_name_matches_platform() {
        #[cfg(target_os = "windows")]
        assert_eq!(HOST_COPY_NAME, "omniget-native-host.exe");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(HOST_COPY_NAME, "omniget-native-host");
    }

    #[test]
    fn manifest_path_from_base_matches_platform_location() {
        #[cfg(target_os = "windows")]
        let base = Path::new(r"C:\Users\test\AppData\Roaming\omniget\chrome-native-host");

        #[cfg(not(target_os = "windows"))]
        let base = Path::new("/tmp/chrome-base");

        let manifest_path = chrome_manifest_path_from_base(base);

        #[cfg(target_os = "windows")]
        assert_eq!(manifest_path, base.join(HOST_MANIFEST_NAME));

        #[cfg(target_os = "linux")]
        assert_eq!(
            manifest_path,
            base.join("google-chrome")
                .join("NativeMessagingHosts")
                .join(HOST_MANIFEST_NAME)
        );

        #[cfg(target_os = "macos")]
        assert_eq!(
            manifest_path,
            base.join("Google")
                .join("Chrome")
                .join("NativeMessagingHosts")
                .join(HOST_MANIFEST_NAME)
        );
    }

    #[test]
    fn build_host_manifest_contains_expected_fields() {
        #[cfg(target_os = "windows")]
        let host_exe = Path::new(r"C:\tmp\omniget-native-host.exe");

        #[cfg(not(target_os = "windows"))]
        let host_exe = Path::new("/tmp/omniget-native-host");

        let manifest = build_host_manifest(host_exe);

        assert_eq!(manifest["name"].as_str(), Some(CHROME_HOST_NAME));
        assert_eq!(manifest["description"].as_str(), Some("OmniGet native host for Chrome"));
        assert_eq!(manifest["path"].as_str(), Some(host_exe.to_string_lossy().as_ref()));
        assert_eq!(manifest["type"].as_str(), Some("stdio"));
        assert_eq!(
            manifest["allowed_origins"][0].as_str(),
            Some(format!("chrome-extension://{CHROME_EXTENSION_ID}/").as_str())
        );
    }
}
