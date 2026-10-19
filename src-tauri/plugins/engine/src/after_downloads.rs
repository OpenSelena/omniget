use serde_json::{json, Value};
use tracing::info;

pub async fn handle_after_downloads_command(command: &str, args: Value) -> Result<Value, String> {
    info!("[engine-after-downloads] Executing command: {}", command);

    match command {
        "after_downloads_get" => {
            Ok(json!({
                "action": null,
                "command": null,
                "play_sound": true,
                "notify": true
            }))
        }
        "after_downloads_set" => {
            let action = args.get("action").cloned().unwrap_or(Value::Null);
            Ok(json!({
                "success": true,
                "action": action
            }))
        }
        _ => Ok(json!({
            "status": "success",
            "command": command,
            "handled_by": "omniget-engine-after-downloads"
        })),
    }
}
