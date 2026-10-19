/**
 * OmniGet v1.0.0 Workspace, Catalog, Sessions & Multi-Agent Threads API
 *
 * Provides strongly-typed models, fallback mock stores, format helpers,
 * and RPC dispatchers for Category G routes.
 */

import { invokeV100 } from "./ipc-bridge";
import { t as i18nT } from "$lib/i18n";
import { get } from "svelte/store";

export function t(
  key: string,
  fallbackOrPayload?: string | Record<string, any>,
  payload?: Record<string, any>
): string {
  try {
    const fn = get(i18nT);
    if (typeof fn === "function") {
      const p = typeof fallbackOrPayload === "object" ? fallbackOrPayload : payload;
      const val = fn(key, p);
      if (val && val !== key) return val;
    }
  } catch {
    // fallback
  }
  if (typeof fallbackOrPayload === "string") return fallbackOrPayload;
  return key;
}

export const tr = t;

export {
  formatUsd,
  formatMicroUsd,
  formatDuration,
  formatRelativeTime,
  statusColor,
} from "./company-api";

/* --- 1. Catalog & Packages --- */

export interface CatalogItem {
  id: string;
  name: string;
  description: string;
  category: "tools" | "skills" | "agents" | "models" | "mcp" | "extensions";
  author: string;
  version: string;
  stars: number;
  downloads: number;
  license: string;
  installed: boolean;
  enabled: boolean;
  tags: string[];
  readme: string;
  permissions: string[];
  repository?: string;
  updated_at: number;
}

/* --- 2. Missions & Autonomous Goals --- */

export interface MissionTask {
  id: string;
  title: string;
  done: boolean;
  agent?: string;
  result?: string;
}

export interface MissionLog {
  ts: number;
  level: "info" | "warn" | "error" | "step";
  message: string;
}

export interface MissionItem {
  id: string;
  title: string;
  objective: string;
  status: "active" | "queued" | "completed" | "failed" | "paused";
  priority: "low" | "medium" | "high" | "critical";
  progress_pct: number;
  assigned_agents: string[];
  tasks: MissionTask[];
  logs: MissionLog[];
  created_at: number;
  updated_at: number;
}

/* --- 3. Reading & Research Digest --- */

export interface ReadingItem {
  id: string;
  title: string;
  source_url?: string;
  source_type: "web" | "pdf" | "markdown" | "paper" | "transcript";
  status: "unread" | "reading" | "analyzed" | "archived";
  key_takeaways: string[];
  summary: string;
  tags: string[];
  word_count: number;
  added_at: number;
  extracted_content?: string;
}

/* --- 4. Agent Roster --- */

export interface RosterAgent {
  id: string;
  name: string;
  role: string;
  avatar_color: string;
  primary_model: string;
  fallback_chain: string[];
  temperature: number;
  system_prompt: string;
  active_mcp_servers: string[];
  allowed_tools: string[];
  policy: {
    require_approval_on_write: boolean;
    max_tokens_per_turn: number;
    budget_limit_usd: number;
  };
  status: "idle" | "busy" | "standby";
}

/* --- 5. Interactive Sessions --- */

export interface SessionTurn {
  id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  tool_calls?: Array<{
    name: string;
    args: any;
    output?: any;
    status: "running" | "success" | "failed";
  }>;
  created_at: number;
  tokens?: { input: number; output: number };
  latency_ms?: number;
}

export interface SessionItem {
  id: string;
  tool: string;
  title: string;
  model: string;
  turns_count: number;
  token_count: number;
  status: "active" | "idle" | "finished";
  updated_at: number;
  created_at: number;
  turns?: SessionTurn[];
}

/* --- 6. Summaries --- */

export interface SummaryItem {
  id: string;
  title: string;
  source_type: "text" | "url" | "audio" | "session";
  format: "bullet_points" | "executive" | "detailed" | "action_items";
  input_length_words: number;
  output_summary: string;
  tags: string[];
  created_at: number;
}

/* --- 7. Threads & Turns --- */

export interface ThreadPatch {
  path: string;
  patch: string;
  additions: number;
  deletions: number;
}

export interface ThreadTurn {
  ordinal: number;
  role: string;
  text: string;
  state: "running" | "completed" | "error";
  patches?: ThreadPatch[];
  timestamp: string;
}

export interface ThreadPendingApproval {
  id: string;
  action: string;
  details: any;
  createdAt: string;
}

export interface ThreadPendingUserInput {
  requestId: string;
  questions: Array<{
    id: string;
    prompt: string;
    allowCustomAnswer: boolean;
  }>;
}

export interface ThreadItem {
  threadId: string;
  projectId: string;
  title: string;
  model: string;
  driver: string;
  runtimeMode: "autonomous" | "interactive" | "step_by_step";
  activeTurnId?: string | null;
  pendingApprovalCount: number;
  pendingUserInputCount: number;
  turnCount: number;
  worktreePath?: string;
  created_at: string;
  turns: ThreadTurn[];
  pendingApprovals: ThreadPendingApproval[];
  pendingUserInputs: ThreadPendingUserInput[];
}

/* --- 8. Work Hub --- */

export interface WorkItem {
  id: string;
  title: string;
  category: "approval_needed" | "user_input" | "failed_job" | "active_mission" | "routine";
  severity: "info" | "warning" | "error" | "action_required";
  target_route: string;
  source_id: string;
  description: string;
  created_at: number;
}

/* --- Mock Storage --- */

const mockCatalog: CatalogItem[] = [
  {
    id: "tools/web-browser",
    name: "Playwright Web Agent",
    description: "Headless web browsing, DOM interaction, page screenshot capture and form automation.",
    category: "tools",
    author: "OmniGet Core Team",
    version: "1.2.0",
    stars: 342,
    downloads: 12840,
    license: "MIT",
    installed: true,
    enabled: true,
    tags: ["browser", "web", "scraping", "automation"],
    readme: "# Playwright Web Agent\n\nEnables autonomous browser exploration, PDF rendering, and interactive page actions.\n\n### Usage\nConnect via MCP server `mcp://browser` or call directly from agents.",
    permissions: ["network:http", "network:https", "process:spawn"],
    repository: "https://github.com/OpenSelena/omniget",
    updated_at: Date.now() - 86400000 * 2,
  },
  {
    id: "tools/git-flow",
    name: "Git Worktree Manager",
    description: "Automated Git branch isolation, commit verification, and worktree cleaning for safe agent execution.",
    category: "tools",
    author: "OmniGet Core Team",
    version: "2.0.4",
    stars: 218,
    downloads: 8740,
    license: "MIT",
    installed: true,
    enabled: true,
    tags: ["git", "vcs", "worktree", "safe"],
    readme: "# Git Worktree Manager\n\nIsolates experimental patches in separate disk branches before merging.",
    permissions: ["fs:read", "fs:write", "process:git"],
    repository: "https://github.com/OpenSelena/omniget",
    updated_at: Date.now() - 86400000 * 5,
  },
  {
    id: "skills/deep-research",
    name: "Multi-Source Deep Researcher",
    description: "Recursive web search and citation synthesizer across arXiv, GitHub, and technical documentation.",
    category: "skills",
    author: "OpenSelena Community",
    version: "0.9.5",
    stars: 580,
    downloads: 15400,
    license: "Apache-2.0",
    installed: false,
    enabled: false,
    tags: ["research", "arxiv", "synthesis", "llm"],
    readme: "# Multi-Source Deep Researcher\n\nAutonomous topic exploration, fact checking and citation generation.",
    permissions: ["network:http", "network:https"],
    updated_at: Date.now() - 86400000 * 1,
  },
  {
    id: "agents/code-reviewer",
    name: "Architectural Code Auditor",
    description: "Reviews code changes against coding guidelines, antipatterns, and memory leaks.",
    category: "agents",
    author: "Antigravity Team",
    version: "1.1.0",
    stars: 412,
    downloads: 9230,
    license: "MIT",
    installed: true,
    enabled: true,
    tags: ["review", "audit", "security", "rust", "typescript"],
    readme: "# Architectural Code Auditor\n\nRuns static analysis and diff-by-diff reviews before PR submission.",
    permissions: ["fs:read"],
    updated_at: Date.now() - 86400000 * 7,
  },
  {
    id: "mcp/postgres-inspector",
    name: "PostgreSQL Schema Inspector",
    description: "Read-only database introspection, query plan optimizer and schema diff engine.",
    category: "mcp",
    author: "Database Guild",
    version: "1.0.1",
    stars: 175,
    downloads: 4320,
    license: "MIT",
    installed: false,
    enabled: false,
    tags: ["database", "postgres", "sql", "mcp"],
    readme: "# PostgreSQL Schema Inspector\n\nExposes database structure as typed tools for LLMs.",
    permissions: ["network:tcp"],
    updated_at: Date.now() - 86400000 * 12,
  },
];

const mockMissions: MissionItem[] = [
  {
    id: "mis-v100-audit",
    title: "OmniGet v1.0.0 Native Integration Audit",
    objective: "Decompile all legacy AST bridge routes into pure Svelte 5 runes and achieve 100% native Rust ownership.",
    status: "active",
    priority: "critical",
    progress_pct: 85,
    assigned_agents: ["Lead Engineer", "MosCat Sentinel"],
    tasks: [
      { id: "t1", title: "Decompile Category A Media Routes", done: true, agent: "Lead Engineer", result: "Commit 67df5eba" },
      { id: "t2", title: "Decompile Category B Companion Routes", done: true, agent: "Lead Engineer", result: "Commit dc52fb61" },
      { id: "t3", title: "Decompile Category C Overlays Routes", done: true, agent: "Lead Engineer", result: "Commit e17b6d0b" },
      { id: "t4", title: "Decompile Category D Spatial HUD Routes", done: true, agent: "Lead Engineer", result: "Commit 9eb0618d" },
      { id: "t5", title: "Decompile Category E Company Suite (12 routes)", done: true, agent: "Lead Engineer", result: "Commit 9686e3b4" },
      { id: "t6", title: "Decompile Category F LLM Engines & Arena (14 routes)", done: true, agent: "Lead Engineer", result: "Commit 62000100" },
      { id: "t7", title: "Decompile Category G Catalog & Workspaces (14 routes)", done: false, agent: "Lead Engineer" },
    ],
    logs: [
      { ts: Date.now() - 3600000 * 4, level: "step", message: "Category E decompiled cleanly with 100% test pass rate." },
      { ts: Date.now() - 3600000 * 2, level: "step", message: "Category F completed. Model observatory and arena ready." },
      { ts: Date.now() - 1800000, level: "info", message: "Starting final Category G navigation and workspace decompilation." },
    ],
    created_at: Date.now() - 86400000,
    updated_at: Date.now() - 60000,
  },
  {
    id: "mis-media-sniff",
    title: "Universal Media Sniffer Heuristics",
    objective: "Support DRM-free extraction for 50+ video portals with multi-part stream segment stitching.",
    status: "active",
    priority: "high",
    progress_pct: 60,
    assigned_agents: ["MosCat Sentinel"],
    tasks: [
      { id: "m1", title: "HLS and DASH manifest parser", done: true },
      { id: "m2", title: "Browser cookie extraction profiles", done: true },
      { id: "m3", title: "Automated chunk integrity verification", done: false },
    ],
    logs: [
      { ts: Date.now() - 7200000, level: "info", message: "Sniffer active across 12 target hostnames." },
    ],
    created_at: Date.now() - 86400000 * 3,
    updated_at: Date.now() - 7200000,
  },
];

const mockReadingList: ReadingItem[] = [
  {
    id: "read-1",
    title: "Large Language Model Reasoning Chains & Tree-of-Thought",
    source_url: "https://arxiv.org/abs/2305.10601",
    source_type: "paper",
    status: "analyzed",
    key_takeaways: [
      "Branching reasoning improves complex problem solving by 40%",
      "Backtracking allows pruning dead exploration paths early",
      "Memory state checkpoints reduce redundant token generation",
    ],
    summary: "Comprehensive study exploring multi-step deliberate problem solving in transformer models.",
    tags: ["ai", "reasoning", "agents", "research"],
    word_count: 8400,
    added_at: Date.now() - 86400000 * 4,
  },
  {
    id: "read-2",
    title: "Svelte 5 Runes & Fine-Grained Reactivity Architecture",
    source_url: "https://svelte.dev/docs/svelte/runes",
    source_type: "web",
    status: "reading",
    key_takeaways: [
      "Signals replace compile-time top-level assignment tracking",
      "$state, $derived, and $effect provide universal fine-grained updates",
      "Zero overhead reactive state outside component closures",
    ],
    summary: "Technical guide on Svelte 5 reactive primitives and performance characteristics.",
    tags: ["svelte", "frontend", "reactivity"],
    word_count: 5200,
    added_at: Date.now() - 86400000 * 2,
  },
];

const mockRoster: RosterAgent[] = [
  {
    id: "agent-lead",
    name: "Architect & Lead Engineer",
    role: "System Architecture & Core Implementation",
    avatar_color: "var(--blue)",
    primary_model: "claude-3-7-sonnet",
    fallback_chain: ["gpt-4o", "gemini-2.0-flash"],
    temperature: 0.2,
    system_prompt: "You are the Lead Systems Architect. Write robust, idiomatic, and highly performant code.",
    active_mcp_servers: ["mcp-filesystem", "mcp-git", "mcp-browser"],
    allowed_tools: ["fs:read", "fs:write", "process:run", "git:commit"],
    policy: {
      require_approval_on_write: false,
      max_tokens_per_turn: 32000,
      budget_limit_usd: 50.0,
    },
    status: "busy",
  },
  {
    id: "agent-sentinel",
    name: "MosCat Companion Sentinel",
    role: "Repository Guardian & Media Downloader",
    avatar_color: "var(--orange)",
    primary_model: "gpt-4o",
    fallback_chain: ["claude-3-7-sonnet"],
    temperature: 0.4,
    system_prompt: "You are MosCat, the loyal OmniGet robotic companion. Watch downloads, sniffer pipelines, and health.",
    active_mcp_servers: ["mcp-media", "mcp-network"],
    allowed_tools: ["net:sniff", "media:download", "notify:system"],
    policy: {
      require_approval_on_write: true,
      max_tokens_per_turn: 16000,
      budget_limit_usd: 25.0,
    },
    status: "idle",
  },
];

const mockSessions: SessionItem[] = [
  {
    id: "sess-v100",
    tool: "chat",
    title: "V100 Native Decompilation Strategy",
    model: "claude-3-7-sonnet",
    turns_count: 8,
    token_count: 14200,
    status: "active",
    updated_at: Date.now() - 120000,
    created_at: Date.now() - 3600000,
    turns: [
      {
        id: "t1",
        role: "user",
        content: "Let's decompile Category G routes into pure Svelte 5 runes.",
        created_at: Date.now() - 3600000,
      },
      {
        id: "t2",
        role: "assistant",
        content: "Understood. Starting with shared workspace API, layout navigation shell, catalog, and session runners.",
        created_at: Date.now() - 3590000,
      },
    ],
  },
  {
    id: "sess-media",
    tool: "terminal",
    title: "FFmpeg Segment Pipeline Diagnostic",
    model: "gpt-4o",
    turns_count: 4,
    token_count: 6500,
    status: "finished",
    updated_at: Date.now() - 86400000,
    created_at: Date.now() - 86400000 - 1800000,
  },
];

const mockSummaries: SummaryItem[] = [
  {
    id: "sum-1",
    title: "OmniGet v1.0.0 Architecture Transition",
    source_type: "text",
    format: "executive",
    input_length_words: 12400,
    output_summary: "Complete migration from legacy AST iframe wrappers to native Tauri v2 Rust plugins and Svelte 5 runes. Total of 53 routes decompiled across 7 categories.",
    tags: ["architecture", "v100", "milestone"],
    created_at: Date.now() - 86400000 * 2,
  },
];

const mockThreads: ThreadItem[] = [
  {
    threadId: "th-v100-integration",
    projectId: "omniget-core",
    title: "OmniGet v1.0.0 Integration Pipeline",
    model: "claude-3-7-sonnet",
    driver: "anthropic",
    runtimeMode: "autonomous",
    activeTurnId: null,
    pendingApprovalCount: 0,
    pendingUserInputCount: 0,
    turnCount: 16,
    worktreePath: "x:\\omniget",
    created_at: new Date(Date.now() - 86400000 * 2).toISOString(),
    turns: [
      {
        ordinal: 1,
        role: "user",
        text: "Decompile Category F and Category G routes into pure Svelte 5 runes.",
        state: "completed",
        timestamp: new Date(Date.now() - 7200000).toISOString(),
      },
      {
        ordinal: 2,
        role: "assistant",
        text: "Category F completed. Now executing Category G decompilation.",
        state: "completed",
        patches: [
          { path: "src/lib/v100/workspace-api.ts", patch: "+ export interface CatalogItem ...", additions: 180, deletions: 0 }
        ],
        timestamp: new Date(Date.now() - 3600000).toISOString(),
      },
    ],
    pendingApprovals: [],
    pendingUserInputs: [],
  },
];

const mockWorkItems: WorkItem[] = [
  {
    id: "work-1",
    title: "Complete Category G Decompilation",
    category: "active_mission",
    severity: "action_required",
    target_route: "/llm/catalog",
    source_id: "mis-v100-audit",
    description: "Replace all remaining AST bridge nodes with clean Svelte 5 runes.",
    created_at: Date.now() - 1800000,
  },
  {
    id: "work-2",
    title: "Run Quality Gates Verification",
    category: "routine",
    severity: "info",
    target_route: "/llm/observatory",
    source_id: "qa-pipeline",
    description: "Verify vitest 100%, E2E 60/60, and svelte-check 0 errors.",
    created_at: Date.now() - 900000,
  },
];

/* --- RPC Dispatcher --- */

export async function workspaceRpc<T = any>(method: string, args: Record<string, any> = {}): Promise<T> {
  try {
    const res = await invokeV100("agentkit_command", {
      command: "workspace_rpc",
      method,
      args,
    });
    if (res && typeof res === "object" && (res as any).status !== "error" && Object.keys(res).length > 0) {
      return res as T;
    }
  } catch {
    // fallback
  }

  // Fallback memory state
  switch (method) {
    case "catalog.list":
      return [...mockCatalog] as any;
    case "catalog.get": {
      const it = mockCatalog.find((x) => x.id === args.id);
      return (it || mockCatalog[0]) as any;
    }
    case "catalog.install": {
      const it = mockCatalog.find((x) => x.id === args.id);
      if (it) {
        it.installed = true;
        it.enabled = true;
      }
      return { ok: true } as any;
    }
    case "catalog.uninstall": {
      const it = mockCatalog.find((x) => x.id === args.id);
      if (it) {
        it.installed = false;
        it.enabled = false;
      }
      return { ok: true } as any;
    }
    case "catalog.toggle": {
      const it = mockCatalog.find((x) => x.id === args.id);
      if (it) it.enabled = !it.enabled;
      return it as any;
    }
    case "missions.list":
      return [...mockMissions] as any;
    case "missions.create": {
      const m: MissionItem = {
        id: `mis-${Date.now()}`,
        title: args.title || "New Mission",
        objective: args.objective || "",
        status: "active",
        priority: args.priority || "medium",
        progress_pct: 0,
        assigned_agents: args.agents || ["Architect"],
        tasks: [],
        logs: [{ ts: Date.now(), level: "info", message: "Mission initialized." }],
        created_at: Date.now(),
        updated_at: Date.now(),
      };
      mockMissions.unshift(m);
      return m as any;
    }
    case "reading.list":
      return [...mockReadingList] as any;
    case "reading.add": {
      const r: ReadingItem = {
        id: `read-${Date.now()}`,
        title: args.title || "Untitled Document",
        source_url: args.url,
        source_type: args.source_type || "web",
        status: "unread",
        key_takeaways: [],
        summary: args.summary || "",
        tags: args.tags || [],
        word_count: args.word_count || 1200,
        added_at: Date.now(),
      };
      mockReadingList.unshift(r);
      return r as any;
    }
    case "roster.list":
      return [...mockRoster] as any;
    case "sessions.list":
      return [...mockSessions] as any;
    case "sessions.get": {
      const s = mockSessions.find((x) => x.id === args.id);
      return (s || mockSessions[0]) as any;
    }
    case "summaries.list":
      return [...mockSummaries] as any;
    case "summaries.create": {
      const sum: SummaryItem = {
        id: `sum-${Date.now()}`,
        title: args.title || "Quick Summary",
        source_type: args.source_type || "text",
        format: args.format || "bullet_points",
        input_length_words: args.text ? args.text.split(/\s+/).length : 500,
        output_summary: args.output || "Generated summary points:\n- Key point 1\n- Key point 2",
        tags: args.tags || ["digest"],
        created_at: Date.now(),
      };
      mockSummaries.unshift(sum);
      return sum as any;
    }
    case "threads.list":
      return [...mockThreads] as any;
    case "work.list":
      return [...mockWorkItems] as any;
    default:
      return {} as any;
  }
}
