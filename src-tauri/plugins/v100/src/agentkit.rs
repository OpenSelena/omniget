use serde_json::{json, Value};

pub async fn handle_agentkit_command(
    command: &str,
    _args: Value,
) -> Result<Value, String> {
    match command {
        "agentkit_installed" => {
            Ok(json!({
                "installed": true,
                "version": "1.0.0",
                "plugins": ["fs", "browser", "shell", "git"]
            }))
        }
        "agentkit_detect" => {
            Ok(json!({
                "detected": true,
                "shims": []
            }))
        }
        "agentkit_plugins_list" => {
            Ok(json!({
                "plugins": []
            }))
        }
        "arena_list" => {
            Ok(json!({
                "battles": [],
                "models": []
            }))
        }
        _ => {
            Ok(json!({
                "status": "ok",
                "command": command,
                "data": {}
            }))
        }
    }
}
