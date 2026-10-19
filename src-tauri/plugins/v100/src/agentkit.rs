use serde_json::{json, Value};

pub async fn handle_agentkit_command(
    command: &str,
    args: Value,
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
        "company_rpc" => {
            let method = args.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let rpc_args = args.get("args").cloned().unwrap_or(json!({}));
            handle_company_rpc(method, rpc_args).await
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

async fn handle_company_rpc(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "company.list" => Ok(json!([{
            "id": "omniget-core",
            "name": "OmniGet Operations",
            "issue_prefix": "OMNI",
            "mission": "Universal media downloading, spatial intelligence, and multi-agent systems orchestration.",
            "status": "active",
            "workspace": "x:\\omniget",
            "created_at": 1700000000000_i64
        }])),
        "company.get" => {
            let id = args.get("company").and_then(|v| v.as_str()).unwrap_or("omniget-core");
            Ok(json!({
                "id": id,
                "name": "OmniGet Operations",
                "issue_prefix": "OMNI",
                "mission": "Universal media downloading, spatial intelligence, and multi-agent systems orchestration.",
                "status": "active",
                "workspace": "x:\\omniget",
                "created_at": 1700000000000_i64
            }))
        }
        "company.templates" => Ok(json!([
            { "id": "software", "name": "Software Development", "summary": "Autonomous coding & review team", "roles": ["Architect", "Developer", "Reviewer"] },
            { "id": "content", "name": "Content & Research", "summary": "Media, documentation & knowledge synthesis", "roles": ["Researcher", "Copywriter", "Editor"] },
            { "id": "operations", "name": "Operations & DevOps", "summary": "Automated loops, alerts & pipeline telemetry", "roles": ["SRE", "Security Auditor", "Support"] }
        ])),
        "staff.list" => Ok(json!([
            {
                "id": "staff-lead",
                "company_id": "omniget-core",
                "name": "Omni Commander",
                "role": "System Orchestrator",
                "status": "running",
                "reports_to": null,
                "capabilities": ["terminal", "fs", "git", "browser"],
                "created_at": 1700000000000_i64
            },
            {
                "id": "staff-analyst",
                "company_id": "omniget-core",
                "name": "MosCat Companion",
                "role": "Quality Assurance & Telemetry",
                "status": "running",
                "reports_to": "staff-lead",
                "capabilities": ["telemetry", "cleanup", "pet"],
                "created_at": 1700000000000_i64
            }
        ])),
        "issue.list" => Ok(json!([
            {
                "id": "issue-1",
                "company_id": "omniget-core",
                "identifier": "OMNI-1",
                "title": "Complete v1.0.0 Company & Organization Suite Decompilation",
                "description": "Fully decompile all 12 company routes into pure Svelte 5 runes with 100% test coverage.",
                "status": "in_progress",
                "priority": "urgent",
                "assignee_id": "staff-lead",
                "creator": "User",
                "created_at": 1700000000000_i64,
                "tags": ["core", "v100", "svelte5"]
            }
        ])),
        "company.dashboard" => Ok(json!({
            "company": {
                "id": "omniget-core",
                "name": "OmniGet Operations",
                "issue_prefix": "OMNI",
                "mission": "Universal media downloading, spatial intelligence, and multi-agent systems orchestration.",
                "status": "active",
                "workspace": "x:\\omniget"
            },
            "running": 2,
            "open_approvals": 0,
            "staff": [
                {
                    "id": "staff-lead",
                    "company_id": "omniget-core",
                    "name": "Omni Commander",
                    "role": "System Orchestrator",
                    "status": "running",
                    "reports_to": null,
                    "capabilities": ["terminal", "fs", "git", "browser"]
                },
                {
                    "id": "staff-analyst",
                    "company_id": "omniget-core",
                    "name": "MosCat Companion",
                    "role": "Quality Assurance & Telemetry",
                    "status": "running",
                    "reports_to": "staff-lead",
                    "capabilities": ["telemetry", "cleanup", "pet"]
                }
            ],
            "goals": [
                {
                    "id": "goal-1",
                    "company_id": "omniget-core",
                    "title": "Achieve 100% Native OmniGet v1.0.0 Integration",
                    "description": "Eliminate foreign binary sidecars in favor of in-process Rust plugin architecture.",
                    "level": "objective",
                    "status": "active",
                    "owner_staff_id": "staff-lead"
                }
            ],
            "projects": [
                {
                    "id": "proj-1",
                    "company_id": "omniget-core",
                    "name": "OmniGet Desktop",
                    "key": "DESK",
                    "description": "Tauri v2 + Svelte 5 Native Desktop Suite",
                    "color": "var(--blue)",
                    "workspace_folder": "x:\\omniget"
                }
            ],
            "budgets": [
                {
                    "policy": {
                        "id": "b-monthly",
                        "company_id": "omniget-core",
                        "scope_type": "company",
                        "metric": "usd",
                        "window": "monthly",
                        "amount": 100.0,
                        "warn_pct": 80.0,
                        "action": "notify",
                        "enabled": true
                    },
                    "used": 12.45,
                    "pct": 12.45
                }
            ],
            "recent_runs": [
                {
                    "id": "run-101",
                    "company_id": "omniget-core",
                    "staff_id": "staff-lead",
                    "issue_id": "issue-1",
                    "source": "linear_worker",
                    "state": "running",
                    "liveness": "active",
                    "cost_micros": 25400,
                    "started_ms": 1700000000000_i64,
                    "created_ms": 1700000000000_i64
                }
            ],
            "issue_counts": {
                "backlog": 0,
                "todo": 1,
                "in_progress": 1,
                "in_review": 0,
                "blocked": 0,
                "done": 1,
                "cancelled": 0
            },
            "costs": {
                "cost_usd": 12.45,
                "spend_today_usd": 1.20,
                "spend_this_week_usd": 4.80,
                "spend_this_month_usd": 12.45,
                "total_usd": 48.90,
                "models": {
                    "claude-3-7-sonnet": { "calls": 140, "tokens": 420000, "cost_usd": 8.50 },
                    "gpt-4o": { "calls": 60, "tokens": 180000, "cost_usd": 3.95 }
                },
                "agents": {
                    "Omni Commander": { "calls": 120, "tokens": 360000, "cost_usd": 7.20 },
                    "MosCat Companion": { "calls": 80, "tokens": 240000, "cost_usd": 5.25 }
                }
            }
        })),
        "inbox.list" => Ok(json!({
            "approvals": [],
            "incidents": [],
            "issues": []
        })),
        "goal.list" | "goals.list" => Ok(json!([
            {
                "id": "goal-1",
                "company_id": "omniget-core",
                "title": "Achieve 100% Native OmniGet v1.0.0 Integration",
                "description": "Eliminate foreign binary sidecars in favor of in-process Rust plugin architecture.",
                "level": "objective",
                "status": "active",
                "owner_staff_id": "staff-lead"
            }
        ])),
        "routine.list" | "routines.list" => Ok(json!([
            {
                "id": "routine-health",
                "company_id": "omniget-core",
                "name": "System Health & Memory Integrity",
                "description": "Monitors process handles, token spend, and cache cleanliness.",
                "cron": "0 * * * *",
                "enabled": true,
                "agent_id": "staff-analyst",
                "prompt": "Perform periodic sanity check on active download streams and process handles.",
                "last_run_at": 1700000000000_i64,
                "last_run_status": "succeeded"
            }
        ])),
        "project.list" => Ok(json!([
            {
                "id": "proj-1",
                "company_id": "omniget-core",
                "name": "OmniGet Desktop",
                "key": "DESK",
                "description": "Tauri v2 + Svelte 5 Native Desktop Suite",
                "color": "var(--blue)",
                "workspace_folder": "x:\\omniget"
            }
        ])),
        "audit.list" => Ok(json!([
            {
                "seq": 1,
                "company_id": "omniget-core",
                "ts_ms": 1700000000000_i64,
                "action": "company.created",
                "entity_kind": "company",
                "entity_id": "omniget-core",
                "actor_kind": "user",
                "actor_id": "mint",
                "channel": "app",
                "hash": "a1b2c3d4e5f67890",
                "details": { "name": "OmniGet Operations", "issue_prefix": "OMNI" }
            }
        ])),
        "audit.verify" => Ok(json!({
            "intact": true,
            "lines": 1
        })),
        "audit.csv" => Ok(json!({
            "csv": "seq,ts,action,entity,actor\n1,1700000000,company.created,company,mint\n"
        })),
        "wake.list" => Ok(json!([])),
        "costs.summary" => Ok(json!({
            "cost_usd": 12.45,
            "spend_today_usd": 1.20,
            "spend_this_week_usd": 4.80,
            "spend_this_month_usd": 12.45,
            "total_usd": 48.90,
            "models": {
                "claude-3-7-sonnet": { "calls": 140, "tokens": 420000, "cost_usd": 8.50 },
                "gpt-4o": { "calls": 60, "tokens": 180000, "cost_usd": 3.95 }
            },
            "agents": {
                "Omni Commander": { "calls": 120, "tokens": 360000, "cost_usd": 7.20 },
                "MosCat Companion": { "calls": 80, "tokens": 240000, "cost_usd": 5.25 }
            }
        })),
        "company.create" => {
            let name = args.pointer("/company/name").and_then(|v| v.as_str()).unwrap_or("New Company");
            let mission = args.pointer("/company/mission").and_then(|v| v.as_str()).unwrap_or("");
            let workspace = args.pointer("/company/workspace").and_then(|v| v.as_str());
            let prefix = if name.len() >= 4 { name[..4].to_uppercase() } else { "COMP".to_string() };
            Ok(json!({
                "id": format!("c-{}", chrono::Utc::now().timestamp_millis()),
                "name": name,
                "mission": mission,
                "issue_prefix": prefix,
                "status": "active",
                "workspace": workspace,
                "created_at": chrono::Utc::now().timestamp_millis()
            }))
        }
        "import.preview" => {
            let name = args.pointer("/options/name").and_then(|v| v.as_str()).unwrap_or("Imported Organization");
            Ok(json!({
                "plan_hash": "rust-preview-hash-98765",
                "company": name,
                "issue_prefix": "IMP",
                "staff": ["Lead Architect", "Test Engineer"],
                "goals": 3,
                "projects": 1,
                "routines": 2,
                "budgets": 1,
                "issues": 4,
                "warnings": [],
                "secrets_to_fill": []
            }))
        }
        "import.apply" => {
            let name = args.pointer("/options/name").and_then(|v| v.as_str()).unwrap_or("Imported Organization");
            Ok(json!({
                "id": format!("c-imp-{}", chrono::Utc::now().timestamp_millis()),
                "name": name,
                "mission": "Imported enterprise workspace",
                "issue_prefix": "IMP",
                "status": "active"
            }))
        }
        _ => Ok(json!({ "status": "ok", "method": method, "data": {} })),
    }
}
