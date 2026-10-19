use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod after_downloads;
pub mod agentkit;
pub mod canvas;
pub mod league;
pub mod limits;
pub mod llm;
pub mod routines;
pub mod spicetify;
pub mod superpowers;
pub mod tools;

pub struct V100Plugin {
    host: Option<Arc<dyn PluginHost>>,
    workspace_dir: Arc<RwLock<PathBuf>>,
    active_sandbox: Arc<RwLock<bool>>,
}

impl V100Plugin {
    pub fn new() -> Self {
        let default_workspace = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            host: None,
            workspace_dir: Arc::new(RwLock::new(default_workspace)),
            active_sandbox: Arc::new(RwLock::new(true)),
        }
    }
}

pub fn manifest() -> omniget_plugin_sdk::PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid v100 manifest")
}

impl OmnigetPlugin for V100Plugin {
    fn id(&self) -> &str {
        "v100"
    }

    fn name(&self) -> &str {
        "OmniGet v1.0.0 Native Engine"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        tracing::info!("[v100-native] Initialized native OmniGet v1.0.0 engine in Rust");
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Value, String>> + Send + 'static>,
    > {
        let ws = Arc::clone(&self.workspace_dir);
        let sb = Arc::clone(&self.active_sandbox);

        Box::pin(async move {
            tracing::debug!("[v100-native] handle_command: {}", command);

            // 1. LLM & Workspace Commands
            if command.starts_with("llm_") || command.starts_with("assist_") {
                return llm::handle_llm_command(&command, args, ws, sb).await;
            }

            // 2. Canvas & Graph Commands
            if command.starts_with("canvas_") {
                return canvas::handle_canvas_command(&command, args, ws).await;
            }

            // 3. AgentKit Commands
            if command.starts_with("agentkit_") || command.starts_with("arena_") || command.starts_with("company_") {
                return agentkit::handle_agentkit_command(&command, args).await;
            }

            // 4. Superpowers & Media Pipeline
            if command.starts_with("superpowers_") || command.starts_with("media_") {
                return superpowers::handle_superpowers_command(&command, args).await;
            }

            // 5. Tools & Utility Subsystems
            if command.starts_with("tool_") || command.starts_with("tools_") {
                return tools::handle_tools_command(&command, args).await;
            }

            // 6. Routines & Automation
            if command.starts_with("routine_") || command.starts_with("routines_") {
                return routines::handle_routines_command(&command, args).await;
            }

            // 7. Limits & Quotas
            if command.starts_with("limits_") {
                return limits::handle_limits_command(&command, args).await;
            }

            // 8. After-Downloads Actions
            if command.starts_with("after_downloads_") {
                return after_downloads::handle_after_downloads_command(&command, args).await;
            }

            // 9. Spicetify Subsystem
            if command.starts_with("spicetify_") {
                return spicetify::handle_spicetify_command(&command, args).await;
            }

            // 10. League Subsystem
            if command.starts_with("league_") {
                return league::handle_league_command(&command, args).await;
            }

            // 11. Fallback for unhandled command
            Ok(json!({
                "status": "success",
                "handled_by": "omniget-v100-native",
                "command": command,
                "data": {}
            }))
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "llm_workspace_get".into(),
            "llm_workspace_set".into(),
            "llm_permission_rules_get".into(),
            "llm_permission_rules_set".into(),
            "llm_chat_send".into(),
            "llm_turn_undo".into(),
            "llm_conversation_list".into(),
            "llm_accounts_list".into(),
            "assist_conversation_context".into(),
            "canvas_state_get".into(),
            "canvas_state_set".into(),
            "canvas_artifacts_list".into(),
            "agentkit_installed".into(),
            "agentkit_detect".into(),
            "superpowers_list".into(),
            "superpowers_toggle".into(),
            "tool_keys_balance".into(),
            "tool_whisper_install".into(),
            "routine_list".into(),
            "limits_get".into(),
            "after_downloads_get".into(),
            "spicetify_install".into(),
            "league_status".into(),
        ]
    }
}
