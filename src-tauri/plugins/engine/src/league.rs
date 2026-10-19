use serde_json::{json, Value};
use tracing::info;

pub async fn handle_league_command(command: &str, _args: Value) -> Result<Value, String> {
    info!("[engine-league] Executing command: {}", command);

    match command {
        "league_status" | "league_detect" => Ok(json!({ "running": false, "phase": "None" })),
        "league_apply_runes"
        | "league_auto_accept_set"
        | "league_declare_champion"
        | "league_dodge"
        | "league_reset_chat_rank"
        | "league_restart_ux"
        | "league_roll_ward"
        | "league_send_chat"
        | "league_set_challenge_crystal"
        | "league_set_challenge_prefs"
        | "league_set_chat_icon"
        | "league_set_chat_rank"
        | "league_set_icon"
        | "league_set_positions"
        | "league_set_profile_background"
        | "league_set_status"
        | "league_spectate" => Ok(json!({ "success": true })),
        _ => Ok(json!({
            "status": "success",
            "command": command,
            "handled_by": "omniget-engine-league"
        })),
    }
}
