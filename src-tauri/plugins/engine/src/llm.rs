use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle_llm_command(
    command: &str,
    args: Value,
    workspace_dir: Arc<RwLock<PathBuf>>,
    active_sandbox: Arc<RwLock<bool>>,
) -> Result<Value, String> {
    match command {
        "llm_workspace_get" => {
            let path = workspace_dir.read().await;
            let sandbox = *active_sandbox.read().await;
            Ok(json!({
                "path": path.to_string_lossy(),
                "sandbox": sandbox
            }))
        }
        "llm_workspace_set" => {
            if let Some(new_path) = args.get("path").and_then(|v| v.as_str()) {
                let mut path = workspace_dir.write().await;
                *path = PathBuf::from(new_path);
            }
            if let Some(sb) = args.get("sandbox").and_then(|v| v.as_bool()) {
                let mut sandbox = active_sandbox.write().await;
                *sandbox = sb;
            }
            Ok(json!({ "success": true }))
        }
        "llm_permission_rules_get" => Ok(json!({
            "rules": [
                { "tool": "fs:read", "pattern": "*", "action": "allow" },
                { "tool": "fs:write", "pattern": "build/**", "action": "allow" },
                { "tool": "net:http", "pattern": "http://127.0.0.1:*", "action": "allow" }
            ]
        })),
        "llm_permission_rules_set" => Ok(json!({ "success": true })),
        "llm_chat_send" => {
            let user_text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let agent_id = args
                .get("agentId")
                .and_then(|v| v.as_str())
                .unwrap_or("assistant");

            // Native response synthesis
            let response = format!(
                "OmniGet Native Engine [{}]: Received '{}'. Workspace synced.",
                agent_id, user_text
            );

            Ok(json!({
                "text": response,
                "model": "native-rust",
                "done": true
            }))
        }
        "llm_turn_undo" => Ok(json!({ "success": true })),
        "llm_conversation_list" => Ok(json!({
            "conversations": []
        })),
        "llm_accounts_list" => Ok(json!({
            "accounts": [
                { "id": "local", "name": "Local Runner", "provider": "native", "active": true }
            ]
        })),
        "assist_conversation_context" => Ok(json!({
            "total_tokens": 512,
            "context_window": 128000,
            "remaining_tokens": 127488
        })),
        _ => Ok(json!({
            "status": "ok",
            "command": command,
            "data": {}
        })),
    }
}
