/**
 * OmniGet v1.0.0 LLM Engines, Models, Inference & Arena API
 *
 * Provides strongly-typed models, RPC dispatchers, reactive event helpers,
 * and format utilities for Category F routes.
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

/* Models & Providers */
export interface ModelInfo {
  id: string;
  name: string;
  provider: "anthropic" | "openai" | "google" | "deepseek" | "ollama" | "local";
  context_length: number;
  input_price_per_m?: number;
  output_price_per_m?: number;
  capabilities: {
    vision: boolean;
    tools: boolean;
    json: boolean;
    thinking?: boolean;
    streaming: boolean;
  };
  is_local: boolean;
  status: "ready" | "downloading" | "unavailable";
  download_progress?: number;
  size_gb?: number;
  quantization?: string;
  description?: string;
}

export interface ProviderConfig {
  id: string;
  name: string;
  type: "anthropic" | "openai" | "google" | "deepseek" | "ollama" | "custom";
  base_url?: string;
  api_key_masked?: string;
  has_key: boolean;
  enabled: boolean;
  models_count: number;
  rate_limit_rpm?: number;
  latency_ms?: number;
}

/* Arena */
export interface ArenaBattle {
  id: string;
  prompt: string;
  category: "coding" | "reasoning" | "creative" | "analysis";
  model_a: string;
  model_b: string;
  response_a: string;
  response_b: string;
  latency_a_ms: number;
  latency_b_ms: number;
  tokens_a: number;
  tokens_b: number;
  winner: "a" | "b" | "tie" | null;
  judge_commentary?: string;
  created_at: number;
}

export interface ArenaLeaderboardEntry {
  model_id: string;
  name: string;
  provider: string;
  elo: number;
  battles: number;
  win_rate: number;
  avg_latency_ms: number;
}

/* MCP & Tools */
export interface McpServer {
  id: string;
  name: string;
  transport: "stdio" | "sse" | "websocket";
  command?: string;
  args?: string[];
  url?: string;
  status: "connected" | "connecting" | "stopped" | "error";
  error_message?: string;
  tools_count: number;
  resources_count: number;
  prompts_count: number;
  enabled: boolean;
  last_ping_ms?: number;
}

export interface ToolItem {
  id: string;
  server_id?: string;
  server_name?: string;
  name: string;
  description: string;
  parameters: Record<string, any>;
  risk_level: "low" | "medium" | "high";
  requires_approval: boolean;
  enabled: boolean;
  call_count: number;
}

export interface SkillItem {
  id: string;
  name: string;
  description: string;
  trigger_phrase?: string;
  author: string;
  version: string;
  installed: boolean;
  enabled: boolean;
  tools_required: string[];
}

/* Jobs, Loops & Observatory */
export interface InferenceJob {
  id: string;
  model: string;
  prompt_preview: string;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  tokens_in: number;
  tokens_out: number;
  duration_ms: number;
  cost_micros: number;
  created_at: number;
}

export interface AgentLoop {
  id: string;
  name: string;
  agent_name: string;
  state: "running" | "idle" | "paused" | "interrupted";
  iteration: number;
  max_iterations: number;
  current_action?: string;
  last_thought?: string;
  started_at: number;
}

export interface ObservatoryStats {
  active_streams: number;
  throughput_tokens_per_sec: number;
  p50_latency_ms: number;
  p95_latency_ms: number;
  error_rate_pct: number;
  total_tokens_24h: number;
  total_cost_24h_usd: number;
}

export interface UsageSummary {
  period: "day" | "week" | "month" | "all";
  total_tokens: number;
  total_cost_usd: number;
  by_provider: Record<string, { tokens: number; cost_usd: number; requests: number }>;
  by_model: Record<string, { tokens: number; cost_usd: number; requests: number }>;
  history: { date: string; tokens: number; cost_usd: number }[];
}

export interface MemoryEntry {
  id: string;
  collection: string;
  content: string;
  vector_dim: number;
  metadata?: Record<string, any>;
  score?: number;
  created_at: number;
}

export interface RetroItem {
  id: string;
  session_id: string;
  agent: string;
  summary: string;
  verdict: "success" | "partial" | "failed";
  root_cause?: string;
  lesson_learned: string;
  action_item?: string;
  created_at: number;
}

/* In-Memory Mock Store for Resilience */
const mockModels: ModelInfo[] = [
  {
    id: "claude-3-7-sonnet",
    name: "Claude 3.7 Sonnet",
    provider: "anthropic",
    context_length: 200000,
    input_price_per_m: 3.0,
    output_price_per_m: 15.0,
    capabilities: { vision: true, tools: true, json: true, thinking: true, streaming: true },
    is_local: false,
    status: "ready",
    description: "Hybrid fast and extended reasoning flagship model.",
  },
  {
    id: "gpt-4o",
    name: "GPT-4o",
    provider: "openai",
    context_length: 128000,
    input_price_per_m: 2.5,
    output_price_per_m: 10.0,
    capabilities: { vision: true, tools: true, json: true, streaming: true },
    is_local: false,
    status: "ready",
    description: "High-intelligence multimodal omni model.",
  },
  {
    id: "gemini-2.0-flash",
    name: "Gemini 2.0 Flash",
    provider: "google",
    context_length: 1000000,
    input_price_per_m: 0.1,
    output_price_per_m: 0.4,
    capabilities: { vision: true, tools: true, json: true, streaming: true },
    is_local: false,
    status: "ready",
    description: "Sub-second speed with 1M native context window.",
  },
  {
    id: "deepseek-r1",
    name: "DeepSeek-R1",
    provider: "deepseek",
    context_length: 64000,
    input_price_per_m: 0.55,
    output_price_per_m: 2.19,
    capabilities: { vision: false, tools: true, json: true, thinking: true, streaming: true },
    is_local: false,
    status: "ready",
    description: "Open reasoning model with competitive chain-of-thought.",
  },
  {
    id: "llama-3.3-70b-local",
    name: "Llama 3.3 70B (Local Ollama)",
    provider: "ollama",
    context_length: 131072,
    capabilities: { vision: false, tools: true, json: true, streaming: true },
    is_local: true,
    status: "ready",
    size_gb: 38.5,
    quantization: "Q4_K_M",
    description: "State-of-the-art open weights running directly on GPU/NPU.",
  },
];

const mockProviders: ProviderConfig[] = [
  { id: "p-anthropic", name: "Anthropic", type: "anthropic", has_key: true, api_key_masked: "sk-ant-...49f8", enabled: true, models_count: 3, latency_ms: 180 },
  { id: "p-openai", name: "OpenAI", type: "openai", has_key: true, api_key_masked: "sk-proj-...82a1", enabled: true, models_count: 4, latency_ms: 210 },
  { id: "p-google", name: "Google Gemini", type: "google", has_key: true, api_key_masked: "AIzaSy...739x", enabled: true, models_count: 3, latency_ms: 140 },
  { id: "p-deepseek", name: "DeepSeek", type: "deepseek", has_key: true, api_key_masked: "sk-ds-...19d0", enabled: true, models_count: 2, latency_ms: 320 },
  { id: "p-ollama", name: "Ollama (Local Engine)", type: "ollama", base_url: "http://127.0.0.1:11434", has_key: false, enabled: true, models_count: 1, latency_ms: 18 },
];

const mockBattles: ArenaBattle[] = [
  {
    id: "bat-101",
    prompt: "Write a high-performance concurrent token bucket rate limiter in Rust without using external crates.",
    category: "coding",
    model_a: "claude-3-7-sonnet",
    model_b: "gpt-4o",
    response_a: "```rust\nuse std::sync::atomic::{AtomicU64, Ordering};\nuse std::time::Instant;\n\npub struct TokenBucket {\n    capacity: u64,\n    tokens: AtomicU64,\n    last_refill: Instant,\n}\n```",
    response_b: "```rust\nuse std::sync::Mutex;\nuse std::time::Instant;\n\npub struct RateLimiter {\n    capacity: usize,\n    available: usize,\n}\n```",
    latency_a_ms: 820,
    latency_b_ms: 1140,
    tokens_a: 420,
    tokens_b: 390,
    winner: "a",
    judge_commentary: "Model A properly leveraged lock-free atomic primitives avoiding mutex lock contention.",
    created_at: Date.now() - 3600000 * 2,
  },
];

const mockLeaderboard: ArenaLeaderboardEntry[] = [
  { model_id: "claude-3-7-sonnet", name: "Claude 3.7 Sonnet", provider: "Anthropic", elo: 1294, battles: 48, win_rate: 0.72, avg_latency_ms: 850 },
  { model_id: "gpt-4o", name: "GPT-4o", provider: "OpenAI", elo: 1256, battles: 52, win_rate: 0.65, avg_latency_ms: 1100 },
  { model_id: "deepseek-r1", name: "DeepSeek-R1", provider: "DeepSeek", elo: 1242, battles: 36, win_rate: 0.61, avg_latency_ms: 1650 },
  { model_id: "gemini-2.0-flash", name: "Gemini 2.0 Flash", provider: "Google", elo: 1210, battles: 42, win_rate: 0.54, avg_latency_ms: 320 },
  { model_id: "llama-3.3-70b-local", name: "Llama 3.3 70B", provider: "Ollama", elo: 1180, battles: 28, win_rate: 0.46, avg_latency_ms: 450 },
];

const mockMcpServers: McpServer[] = [
  {
    id: "mcp-filesystem",
    name: "Local Filesystem MCP",
    transport: "stdio",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "x:\\omniget"],
    status: "connected",
    tools_count: 6,
    resources_count: 14,
    prompts_count: 2,
    enabled: true,
    last_ping_ms: 4,
  },
  {
    id: "mcp-git",
    name: "Git VCS Engine",
    transport: "stdio",
    command: "uvx",
    args: ["mcp-server-git", "--repository", "x:\\omniget"],
    status: "connected",
    tools_count: 8,
    resources_count: 3,
    prompts_count: 0,
    enabled: true,
    last_ping_ms: 8,
  },
];

const mockTools: ToolItem[] = [
  { id: "tool-read", server_id: "mcp-filesystem", server_name: "Local Filesystem MCP", name: "read_file", description: "Read UTF-8 contents of a file within permitted workspace paths", parameters: { path: "string" }, risk_level: "low", requires_approval: false, enabled: true, call_count: 240 },
  { id: "tool-write", server_id: "mcp-filesystem", server_name: "Local Filesystem MCP", name: "write_to_file", description: "Write or overwrite file contents safely with rollback snapshot", parameters: { path: "string", content: "string" }, risk_level: "medium", requires_approval: true, enabled: true, call_count: 84 },
  { id: "tool-shell", server_id: "mcp-git", server_name: "Git VCS Engine", name: "run_git_command", description: "Execute read-only or staged git commands", parameters: { args: "array" }, risk_level: "low", requires_approval: false, enabled: true, call_count: 112 },
];

const mockSkills: SkillItem[] = [
  { id: "skill-code-review", name: "Code Review & Hygiene", description: "Zero-slop automated PR diff and invariant review", author: "OmniGet Core", version: "1.2.0", installed: true, enabled: true, tools_required: ["read_file", "run_git_command"] },
  { id: "skill-doc-generator", name: "Docflow Synthesizer", description: "Generates clean human-readable architecture markdown documents", author: "OmniGet Core", version: "1.0.4", installed: true, enabled: true, tools_required: ["read_file", "write_to_file"] },
  { id: "skill-security-audit", name: "Security Invariants Auditor", description: "Scans for secret leakage, insecure endpoints, and handle leaks", author: "Security Pod", version: "2.0.1", installed: true, enabled: true, tools_required: ["read_file"] },
];

const mockJobs: InferenceJob[] = [
  { id: "job-901", model: "claude-3-7-sonnet", prompt_preview: "Audit Category E routes and verify Svelte 5 runes conformance...", status: "completed", tokens_in: 1840, tokens_out: 490, duration_ms: 1420, cost_micros: 12870, created_at: Date.now() - 3600000 },
  { id: "job-902", model: "gemini-2.0-flash", prompt_preview: "Transcribe audio stream chunks from queued video lecture...", status: "completed", tokens_in: 3200, tokens_out: 1200, duration_ms: 840, cost_micros: 800, created_at: Date.now() - 7200000 },
];

const mockLoops: AgentLoop[] = [
  { id: "loop-1", name: "Background Media Sniffer", agent_name: "MosCat Companion", state: "running", iteration: 142, max_iterations: 1000, current_action: "Listening to socket stream buffer...", started_at: Date.now() - 86400000 },
  { id: "loop-2", name: "Continuous Integration Watcher", agent_name: "Code Engineer", state: "idle", iteration: 38, max_iterations: 500, current_action: "Waiting for git filesystem change event", started_at: Date.now() - 86400000 * 2 },
];

const mockMemories: MemoryEntry[] = [
  { id: "mem-1", collection: "architecture", content: "OmniGet uses in-process Rust plugin architecture under src-tauri/plugins/v100 to avoid binary sidecars.", vector_dim: 1536, created_at: Date.now() - 86400000 * 7 },
  { id: "mem-2", collection: "rules", content: "Strictly avoid em dashes in documentation and session logs across all projects.", vector_dim: 1536, created_at: Date.now() - 86400000 * 14 },
];

const mockRetros: RetroItem[] = [
  { id: "retro-1", session_id: "sess-849", agent: "Code Engineer", summary: "Refactored company routes to use local t() store helper to avoid Svelte store calling signature mismatch.", verdict: "success", lesson_learned: "import { t } from $lib/i18n is a Readable store, not a direct function.", created_at: Date.now() - 86400000 },
];

/* Dynamic Dispatcher Function */
export async function llmRpc<T = any>(method: string, args: Record<string, any> = {}): Promise<T> {
  // First attempt native backend / IPC bridge
  try {
    const res = await invokeV100("llm_rpc", { method, args });
    if (res !== null && res !== undefined && !(typeof res === "object" && Object.keys(res).length === 0 && !Array.isArray(res))) {
      return res as T;
    }
  } catch {
    // Continue to mock fallback
  }

  // Fallback handlers
  switch (method) {
    case "models.list":
      return [...mockModels] as any;
    case "models.toggle": {
      const m = mockModels.find((x) => x.id === args.id);
      if (m) m.status = m.status === "ready" ? "unavailable" : "ready";
      return m as any;
    }
    case "providers.list":
      return [...mockProviders] as any;
    case "providers.update": {
      const p = mockProviders.find((x) => x.id === args.id);
      if (p) Object.assign(p, args.patch || {});
      return p as any;
    }
    case "arena.battles":
      return [...mockBattles] as any;
    case "arena.leaderboard":
      return [...mockLeaderboard] as any;
    case "arena.vote": {
      const b = mockBattles.find((x) => x.id === args.battle_id);
      if (b) b.winner = args.winner;
      return { ok: true } as any;
    }
    case "mcp.servers":
      return [...mockMcpServers] as any;
    case "mcp.toggle": {
      const s = mockMcpServers.find((x) => x.id === args.id);
      if (s) s.enabled = !s.enabled;
      return s as any;
    }
    case "tools.list":
      return [...mockTools] as any;
    case "tools.toggle": {
      const t = mockTools.find((x) => x.id === args.id);
      if (t) t.enabled = !t.enabled;
      return t as any;
    }
    case "skills.list":
      return [...mockSkills] as any;
    case "skills.toggle": {
      const sk = mockSkills.find((x) => x.id === args.id);
      if (sk) sk.enabled = !sk.enabled;
      return sk as any;
    }
    case "jobs.list":
      return [...mockJobs] as any;
    case "loops.list":
      return [...mockLoops] as any;
    case "loops.toggle": {
      const l = mockLoops.find((x) => x.id === args.id);
      if (l) l.state = l.state === "running" ? "paused" : "running";
      return l as any;
    }
    case "observatory.stats":
      return {
        active_streams: 2,
        throughput_tokens_per_sec: 142.8,
        p50_latency_ms: 640,
        p95_latency_ms: 1850,
        error_rate_pct: 0.12,
        total_tokens_24h: 1845000,
        total_cost_24h_usd: 12.45,
      } as any;
    case "usage.summary":
      return {
        period: "month",
        total_tokens: 3840000,
        total_cost_usd: 24.85,
        by_provider: {
          anthropic: { tokens: 2100000, cost_usd: 16.2, requests: 280 },
          openai: { tokens: 1200000, cost_usd: 7.15, requests: 140 },
          google: { tokens: 450000, cost_usd: 0.9, requests: 90 },
          deepseek: { tokens: 90000, cost_usd: 0.6, requests: 25 },
        },
        by_model: {
          "claude-3-7-sonnet": { tokens: 2100000, cost_usd: 16.2, requests: 280 },
          "gpt-4o": { tokens: 1200000, cost_usd: 7.15, requests: 140 },
          "gemini-2.0-flash": { tokens: 450000, cost_usd: 0.9, requests: 90 },
        },
        history: [
          { date: "2026-10-15", tokens: 420000, cost_usd: 3.1 },
          { date: "2026-10-16", tokens: 580000, cost_usd: 4.4 },
          { date: "2026-10-17", tokens: 720000, cost_usd: 5.2 },
          { date: "2026-10-18", tokens: 610000, cost_usd: 4.1 },
          { date: "2026-10-19", tokens: 890000, cost_usd: 6.8 },
        ],
      } as any;
    case "memory.list":
      return [...mockMemories] as any;
    case "retro.list":
      return [...mockRetros] as any;
    default:
      return {} as any;
  }
}
