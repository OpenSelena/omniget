use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle_canvas_command(
    command: &str,
    _args: Value,
    workspace_dir: Arc<RwLock<PathBuf>>,
) -> Result<Value, String> {
    match command {
        "canvas_state_get" => {
            Ok(json!({
                "nodes": [],
                "edges": [],
                "viewport": { "x": 0.0, "y": 0.0, "zoom": 1.0 }
            }))
        }
        "canvas_state_set" => {
            Ok(json!({ "success": true }))
        }
        "canvas_artifacts_list" => {
            let path = workspace_dir.read().await;
            Ok(json!({
                "workspace": path.to_string_lossy(),
                "artifacts": []
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
