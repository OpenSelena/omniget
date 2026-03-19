use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CHROME_HOST_NAME: &str = "wtf.tonho.omniget";
pub const CHROME_EXTENSION_ID: &str = "dkjelkhaaakffpghdfalobccaaipajip";

const HOST_COPY_NAME: &str = "omniget-native-host.exe";
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
        .map(|stem| stem.eq_ignore_ascii_case("omniget-native-host"))
        .unwrap_or(false)
}

pub fn run_native_host() -> anyhow::Result<()> {
    let request = read_message()?;
    let response = handle_request(request);
    write_message(&response)?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn ensure_registered() -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    let current_exe = std::env::current_exe()?;
    let integration_dir = crate::core::paths::app_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chrome-native-host");
    fs::create_dir_all(&integration_dir)?;

    let host_exe = integration_dir.join(HOST_COPY_NAME);
    if current_exe != host_exe && should_copy_exe(&current_exe, &host_exe) {
        fs::copy(&current_exe, &host_exe)?;
    }

    let config_path = integration_dir.join(HOST_CONFIG_NAME);
    let manifest_path = integration_dir.join(HOST_MANIFEST_NAME);

    let config = NativeHostConfig {
        app_path: current_exe.to_string_lossy().to_string(),
    };
    fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;

    let manifest = serde_json::json!({
        "name": CHROME_HOST_NAME,
        "description": "OmniGet native host for Chrome",
        "path": host_exe.to_string_lossy().to_string(),
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", CHROME_EXTENSION_ID)
        ]
    });
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

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

// TODO: implement native messaging host registration for macOS
// (~/.config/google-chrome/NativeMessagingHosts/ on Linux,
// ~/Library/Application Support/Google/Chrome/NativeMessagingHosts/ on macOS)
#[cfg(not(target_os = "windows"))]
pub fn ensure_registered() -> anyhow::Result<()> {
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
