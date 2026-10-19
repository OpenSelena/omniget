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
        "llm_rpc" => {
            let method = args.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let rpc_args = args.get("args").cloned().unwrap_or(json!({}));
            handle_llm_rpc(method, rpc_args).await
        }
        "workspace_rpc" => {
            let method = args.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let rpc_args = args.get("args").cloned().unwrap_or(json!({}));
            handle_workspace_rpc(method, rpc_args).await
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

async fn handle_llm_rpc(method: &str, _args: Value) -> Result<Value, String> {
    match method {
        "models.list" => Ok(json!([
            {
                "id": "claude-3-7-sonnet",
                "name": "Claude 3.7 Sonnet",
                "provider": "anthropic",
                "context_length": 200000,
                "capabilities": { "vision": true, "tools": true, "json": true, "thinking": true, "streaming": true },
                "is_local": false,
                "status": "ready"
            },
            {
                "id": "gpt-4o",
                "name": "GPT-4o",
                "provider": "openai",
                "context_length": 128000,
                "capabilities": { "vision": true, "tools": true, "json": true, "streaming": true },
                "is_local": false,
                "status": "ready"
            }
        ])),
        "providers.list" => Ok(json!([
            { "id": "p-anthropic", "name": "Anthropic", "type": "anthropic", "has_key": true, "enabled": true },
            { "id": "p-openai", "name": "OpenAI", "type": "openai", "has_key": true, "enabled": true }
        ])),
        "observatory.stats" => Ok(json!({
            "active_streams": 2,
            "throughput_tokens_per_sec": 142.8,
            "p50_latency_ms": 640,
            "p95_latency_ms": 1850,
            "error_rate_pct": 0.12,
            "total_tokens_24h": 1845000,
            "total_cost_24h_usd": 12.45
        })),
        _ => Ok(json!({}))
    }
}

async fn handle_workspace_rpc(method: &str, _args: Value) -> Result<Value, String> {
    match method {
        "catalog.list" => Ok(json!([
            {
                "id": "tools/web-browser",
                "name": "Playwright Web Agent",
                "description": "Headless web browsing, DOM interaction, page screenshot capture and form automation.",
                "category": "tools",
                "author": "OmniGet Core Team",
                "version": "1.2.0",
                "stars": 342,
                "downloads": 12840,
                "license": "MIT",
                "installed": true,
                "enabled": true,
                "tags": ["browser", "web", "scraping", "automation"],
                "readme": "# Playwright Web Agent\n\nEnables autonomous browser exploration, PDF rendering, and interactive page actions.",
                "permissions": ["network:http", "network:https", "process:spawn"]
            },
            {
                "id": "tools/git-flow",
                "name": "Git Worktree Manager",
                "description": "Automated Git branch isolation, commit verification, and worktree cleaning for safe agent execution.",
                "category": "tools",
                "author": "OmniGet Core Team",
                "version": "2.0.4",
                "stars": 218,
                "downloads": 8740,
                "license": "MIT",
                "installed": true,
                "enabled": true,
                "tags": ["git", "vcs", "worktree", "safe"],
                "readme": "# Git Worktree Manager\n\nIsolates experimental patches in separate disk branches before merging.",
                "permissions": ["fs:read", "fs:write", "process:git"]
            }
        ])),
        "missions.list" => Ok(json!([
            {
                "id": "mis-v100-audit",
                "title": "OmniGet v1.0.0 Native Integration Audit",
                "objective": "Decompile all legacy AST bridge routes into pure Svelte 5 runes and achieve 100% native Rust ownership.",
                "status": "active",
                "priority": "critical",
                "progress_pct": 85,
                "assigned_agents": ["Lead Engineer", "MosCat Sentinel"],
                "tasks": [
                    { "id": "t1", "title": "Decompile Category A Media Routes", "done": true },
                    { "id": "t2", "title": "Decompile Category B Companion Routes", "done": true },
                    { "id": "t3", "title": "Decompile Category C Overlays Routes", "done": true },
                    { "id": "t4", "title": "Decompile Category D Spatial HUD Routes", "done": true },
                    { "id": "t5", "title": "Decompile Category E Company Suite", "done": true },
                    { "id": "t6", "title": "Decompile Category F LLM Engines & Arena", "done": true },
                    { "id": "t7", "title": "Decompile Category G Catalog & Workspaces", "done": false }
                ],
                "logs": [
                    { "ts": 1700000000000_i64, "level": "step", "message": "Decompilation pipeline in progress." }
                ],
                "created_at": 1700000000000_i64,
                "updated_at": 1700000000000_i64
            }
        ])),
        "reading.list" => Ok(json!([
            {
                "id": "read-1",
                "title": "Large Language Model Reasoning Chains & Tree-of-Thought",
                "source_url": "https://arxiv.org/abs/2305.10601",
                "source_type": "paper",
                "status": "analyzed",
                "key_takeaways": ["Branching reasoning improves complex problem solving"],
                "summary": "Comprehensive study on multi-step deliberate problem solving.",
                "tags": ["ai", "reasoning", "agents"],
                "word_count": 8400,
                "added_at": 1700000000000_i64
            }
        ])),
        "roster.list" => Ok(json!([
            {
                "id": "agent-lead",
                "name": "Architect & Lead Engineer",
                "role": "System Architecture & Core Implementation",
                "avatar_color": "var(--blue)",
                "primary_model": "claude-3-7-sonnet",
                "fallback_chain": ["gpt-4o", "gemini-2.0-flash"],
                "temperature": 0.2,
                "system_prompt": "Lead Systems Architect.",
                "active_mcp_servers": ["mcp-filesystem", "mcp-git"],
                "allowed_tools": ["fs:read", "fs:write"],
                "policy": { "require_approval_on_write": false, "max_tokens_per_turn": 32000, "budget_limit_usd": 50.0 },
                "status": "busy"
            }
        ])),
        "sessions.list" => Ok(json!([
            {
                "id": "sess-v100",
                "tool": "chat",
                "title": "V100 Native Decompilation Strategy",
                "model": "claude-3-7-sonnet",
                "turns_count": 8,
                "token_count": 14200,
                "status": "active",
                "updated_at": 1700000000000_i64,
                "created_at": 1700000000000_i64
            }
        ])),
        "summaries.list" => Ok(json!([])),
        "threads.list" => Ok(json!([])),
        "work.list" => Ok(json!([])),
        _ => Ok(json!({}))
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
