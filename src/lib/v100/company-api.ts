/**
 * OmniGet v1.0.0 Company & Organization Management API
 *
 * Provides strongly-typed models, RPC dispatchers, reactive event helpers,
 * and styling/format utilities for all /llm/company routes.
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

export interface Company {
  id: string;
  name: string;
  mission?: string;
  issue_prefix: string;
  status: "active" | "paused" | "archived";
  pause_reason?: string;
  workspace?: string | null;
  created_at?: number;
  updated_at?: number;
}

export interface Staff {
  id: string;
  company_id: string;
  name: string;
  role: string;
  status: "idle" | "running" | "paused" | "terminated";
  reports_to?: string | null;
  agent_id?: string;
  model?: string;
  system_prompt?: string;
  capabilities?: string[];
  created_at?: number;
}

export interface StaffTreeNode {
  staff: Staff;
  reports: StaffTreeNode[];
}

export interface Issue {
  id: string;
  company_id: string;
  identifier: string; // e.g. OMNI-1
  title: string;
  description?: string;
  status: "backlog" | "todo" | "in_progress" | "in_review" | "blocked" | "done" | "cancelled";
  blocked_reason?: string;
  priority?: "low" | "medium" | "high" | "urgent";
  assignee_id?: string | null;
  creator?: string;
  created_at?: number;
  updated_at?: number;
  tags?: string[];
  project_id?: string | null;
  goal_id?: string | null;
}

export interface Goal {
  id: string;
  company_id: string;
  title: string;
  description?: string;
  level: "objective" | "milestone" | "task";
  status: "planned" | "active" | "achieved" | "paused" | "cancelled";
  target_date?: number | null;
  progress?: number;
  owner_staff_id?: string | null;
  parent_goal_id?: string | null;
  milestones?: { id: string; title: string; done: boolean }[];
  created_at?: number;
}

export interface Project {
  id: string;
  company_id: string;
  name: string;
  key: string;
  description?: string;
  color?: string;
  workspace_folder?: string | null;
  created_at?: number;
}

export interface Routine {
  id: string;
  company_id: string;
  name: string;
  description?: string;
  cron: string;
  tz?: string;
  enabled: boolean;
  agent_id?: string;
  prompt?: string;
  last_run_at?: number | null;
  last_run_status?: string | null;
  next_run_at?: number | null;
  created_at?: number;
}

export interface BudgetPolicy {
  id: string;
  company_id: string;
  scope_type: "company" | "agent" | "project";
  scope_id?: string | null;
  metric: "usd" | "tokens";
  window: "daily" | "weekly" | "monthly";
  amount: number;
  warn_pct: number;
  action: "notify" | "pause";
  enabled: boolean;
}

export interface BudgetStatus {
  policy: BudgetPolicy;
  used: number;
  pct: number;
}

export interface CostSummary {
  cost_usd: number;
  spend_today_usd: number;
  spend_this_week_usd: number;
  spend_this_month_usd: number;
  total_usd: number;
  models: Record<string, { calls: number; tokens: number; cost_usd: number }>;
  agents: Record<string, { calls: number; tokens: number; cost_usd: number }>;
}

export interface RunRecord {
  id: string;
  company_id: string;
  staff_id: string;
  issue_id?: string | null;
  source: string;
  state: "running" | "queued" | "succeeded" | "failed" | "cancelled";
  liveness?: string;
  cost_micros: number;
  started_ms: number;
  finished_ms?: number | null;
  created_ms: number;
}

export interface AuditEntry {
  seq: number;
  company_id: string;
  ts_ms: number;
  action: string;
  entity_kind: string;
  entity_id: string;
  actor_kind: "user" | "agent" | "system";
  actor_id: string;
  channel: string;
  hash: string;
  run_id?: string | null;
  details?: any;
}

export interface ApprovalItem {
  id: string;
  company_id: string;
  kind: "hire" | "spend" | "routine" | "diff" | "action";
  title: string;
  requested_by_staff_id?: string;
  note?: string;
  created_ms: number;
  candidate?: any;
  diff?: string;
  amount_usd?: number;
}

export interface IncidentItem {
  id: string;
  company_id: string;
  title: string;
  kind: "soft_limit" | "hard_limit" | "runtime_error" | "rate_limited";
  severity: "warning" | "error";
  raised_by?: string;
  created_ms: number;
  details?: string;
}

export interface InboxTaskItem {
  id: string;
  company_id: string;
  issue_id: string;
  identifier: string;
  title: string;
  why: "review" | "blocked" | "assigned";
  unread: boolean;
  created_ms: number;
}

export interface InboxData {
  approvals: ApprovalItem[];
  incidents: IncidentItem[];
  issues: InboxTaskItem[];
}

export interface DashboardData {
  company: Company;
  running: number;
  open_approvals: number;
  staff: Staff[];
  goals: Goal[];
  projects: Project[];
  budgets: BudgetStatus[];
  recent_runs: RunRecord[];
  issue_counts: Record<string, number>;
  costs: CostSummary;
}

export interface Template {
  id: string;
  name: string;
  summary: string;
  roles: string[];
}

export interface ImportPlan {
  plan_hash: string;
  company: string;
  issue_prefix: string;
  staff: string[];
  goals: number;
  projects: number;
  routines: number;
  budgets: number;
  issues: number;
  warnings: string[];
  secrets_to_fill: string[];
}

/* Palette & Format Helpers */
const PALETTE = ["--blue", "--green", "--orange", "--purple", "--teal", "--red"];

export function avatarColor(name: string = ""): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  return `var(${PALETTE[Math.abs(hash) % PALETTE.length]})`;
}

export function getInitials(name: string = ""): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return "?";
  const first = parts[0][0] ?? "";
  const last = parts.length > 1 ? parts[parts.length - 1][0] ?? "" : "";
  return (first + last).toUpperCase();
}

export function statusColor(status: string): string {
  switch (status) {
    case "running":
    case "in_progress":
    case "active":
      return "green";
    case "succeeded":
    case "done":
    case "achieved":
    case "completed":
      return "blue";
    case "queued":
    case "todo":
    case "in_review":
    case "planned":
      return "blue";
    case "paused":
    case "blocked":
      return "orange";
    case "pending_approval":
      return "yellow";
    case "error":
    case "failed":
    case "timed_out":
      return "red";
    case "terminated":
    case "archived":
    case "cancelled":
      return "faint";
    default:
      return "gray";
  }
}

export function formatUsd(amount: number | null | undefined): string {
  if (amount == null) return "0.00";
  return amount < 0.01 && amount > 0 ? amount.toFixed(4) : amount.toFixed(2);
}

export function formatMicroUsd(micros: number | null | undefined): string {
  return formatUsd((micros ?? 0) / 1_000_000);
}

export function formatDuration(startMs: number, endMs?: number | null): string {
  if (!startMs) return "0s";
  const deltaSec = Math.max(0, Math.round(((endMs ?? Date.now()) - startMs) / 1000));
  if (deltaSec < 60) return `${deltaSec}s`;
  const mins = Math.floor(deltaSec / 60);
  if (mins < 60) return `${mins}m ${deltaSec % 60}s`;
  return `${Math.floor(mins / 60)}h ${mins % 60}m`;
}

export function formatRelativeTime(
  ts: number | null | undefined,
  translate?: (key: string, fallback?: string, params?: any) => string,
  now: number = Date.now()
): string {
  if (!ts) return translate ? translate("company.common.never", "never") : "never";
  const diff = ts - now;
  const abs = Math.abs(diff);
  const mins = Math.round(abs / 60_000);
  if (mins < 1) return translate ? translate("company.common.now", "just now") : "just now";
  const [unit, count] = mins < 60
    ? ["minutes", mins]
    : mins < 2880
    ? ["hours", Math.round(mins / 60)]
    : ["days", Math.round(mins / 1440)];
  const key = diff >= 0 ? `company.common.in_${unit}` : `company.common.${unit}_ago`;
  const fallback = diff >= 0 ? `in ${count} ${unit}` : `${count} ${unit} ago`;
  return translate ? translate(key, fallback, { count }) : fallback;
}

export function cleanError(err: any): string {
  const str = String(err?.message || err || "Unknown error");
  const idx = str.indexOf(": ");
  return str.startsWith("ERR_") && idx > 0 ? str.slice(idx + 2) : str;
}

export const CRON_PRESETS = [
  { id: "every_15", cron: "*/15 * * * *" },
  { id: "hourly", cron: "0 * * * *" },
  { id: "daily", cron: "0 9 * * *" },
  { id: "weekdays", cron: "0 9 * * 1-5" },
  { id: "weekly", cron: "0 9 * * 1" },
  { id: "monthly", cron: "0 9 1 * *" },
];

export const ISSUE_TRANSITIONS: Record<string, string[]> = {
  backlog: ["todo", "cancelled"],
  todo: ["in_progress", "blocked", "cancelled", "backlog"],
  in_progress: ["in_review", "blocked", "done", "cancelled", "todo"],
  in_review: ["in_progress", "done", "cancelled", "todo"],
  blocked: ["todo", "in_progress", "cancelled"],
  done: ["todo"],
  cancelled: ["todo"],
};

export function buildOrgTree(staffList: Staff[], includeTerminated: boolean = false): StaffTreeNode[] {
  const active = includeTerminated ? staffList : staffList.filter((s) => s.status !== "terminated");
  const idSet = new Set(active.map((s) => s.id));
  const childrenMap = new Map<string | null, Staff[]>();

  for (const staff of active) {
    const parent = staff.reports_to && idSet.has(staff.reports_to) ? staff.reports_to : null;
    childrenMap.set(parent, [...(childrenMap.get(parent) ?? []), staff]);
  }

  const visited = new Set<string>();
  const buildNode = (staff: Staff): StaffTreeNode => {
    visited.add(staff.id);
    const children = (childrenMap.get(staff.id) ?? [])
      .filter((child) => !visited.has(child.id))
      .map(buildNode);
    return { staff, reports: children };
  };

  return (childrenMap.get(null) ?? []).map(buildNode);
}

export function downloadFile(filename: string, content: string, mimeType: string = "application/json") {
  const url = URL.createObjectURL(new Blob([content], { type: mimeType }));
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/* Local in-memory mock store for resilient browser testing and fallbacks */
const mockCompanies: Company[] = [
  {
    id: "omniget-core",
    name: "OmniGet Operations",
    issue_prefix: "OMNI",
    mission: "Universal media downloading, spatial intelligence, and multi-agent systems orchestration.",
    status: "active",
    workspace: "x:\\omniget",
    created_at: Date.now() - 86400000 * 30,
  },
];

const mockStaff: Staff[] = [
  {
    id: "staff-lead",
    company_id: "omniget-core",
    name: "Omni Commander",
    role: "System Orchestrator",
    status: "running",
    reports_to: null,
    capabilities: ["terminal", "fs", "git", "browser"],
    created_at: Date.now() - 86400000 * 15,
  },
  {
    id: "staff-analyst",
    company_id: "omniget-core",
    name: "MosCat Companion",
    role: "Quality Assurance & Telemetry",
    status: "running",
    reports_to: "staff-lead",
    capabilities: ["telemetry", "cleanup", "pet"],
    created_at: Date.now() - 86400000 * 10,
  },
];

const mockIssues: Issue[] = [
  {
    id: "issue-1",
    company_id: "omniget-core",
    identifier: "OMNI-1",
    title: "Complete v1.0.0 Company & Organization Suite Decompilation",
    description: "Fully decompile all 12 company routes into pure Svelte 5 runes with 100% test coverage.",
    status: "in_progress",
    priority: "urgent",
    assignee_id: "staff-lead",
    creator: "User",
    created_at: Date.now() - 3600000 * 4,
    tags: ["core", "v100", "svelte5"],
  },
  {
    id: "issue-2",
    company_id: "omniget-core",
    identifier: "OMNI-2",
    title: "Verify media sniffer and universal downloader pipelines",
    description: "Ensure DRM detection, multi-threading, and cookie sync pass all acceptance gates.",
    status: "done",
    priority: "high",
    assignee_id: "staff-analyst",
    creator: "User",
    created_at: Date.now() - 86400000 * 2,
    tags: ["media", "pipeline"],
  },
];

const mockGoals: Goal[] = [
  {
    id: "goal-1",
    company_id: "omniget-core",
    title: "Achieve 100% Native OmniGet v1.0.0 Integration",
    description: "Eliminate all foreign binary sidecars in favor of in-process Rust plugin architecture.",
    level: "objective",
    status: "active",
    owner_staff_id: "staff-lead",
    created_at: Date.now() - 86400000 * 7,
  },
];

const mockProjects: Project[] = [
  {
    id: "proj-1",
    company_id: "omniget-core",
    name: "OmniGet Desktop",
    key: "DESK",
    description: "Tauri v2 + Svelte 5 Native Desktop Suite",
    color: "var(--blue)",
    workspace_folder: "x:\\omniget",
  },
];

const mockRoutines: Routine[] = [
  {
    id: "routine-health",
    company_id: "omniget-core",
    name: "System Health & Memory Integrity",
    description: "Monitors process handles, token spend, and cache cleanliness.",
    cron: "0 * * * *",
    enabled: true,
    agent_id: "staff-analyst",
    prompt: "Perform periodic sanity check on active download streams and process handles.",
    last_run_at: Date.now() - 1800000,
    last_run_status: "succeeded",
  },
];

const mockBudgets: BudgetStatus[] = [
  {
    policy: {
      id: "b-monthly",
      company_id: "omniget-core",
      scope_type: "company",
      metric: "usd",
      window: "monthly",
      amount: 100,
      warn_pct: 80,
      action: "notify",
      enabled: true,
    },
    used: 12.45,
    pct: 12.45,
  },
];

const mockRuns: RunRecord[] = [
  {
    id: "run-101",
    company_id: "omniget-core",
    staff_id: "staff-lead",
    issue_id: "issue-1",
    source: "linear_worker",
    state: "running",
    liveness: "active",
    cost_micros: 25400,
    started_ms: Date.now() - 360000,
    created_ms: Date.now() - 360000,
  },
];

const mockAudit: AuditEntry[] = [
  {
    seq: 1,
    company_id: "omniget-core",
    ts_ms: Date.now() - 86400000,
    action: "company.created",
    entity_kind: "company",
    entity_id: "omniget-core",
    actor_kind: "user",
    actor_id: "mint",
    channel: "app",
    hash: "a1b2c3d4e5f67890",
    details: { name: "OmniGet Operations", issue_prefix: "OMNI" },
  },
];

const mockInbox: InboxData = {
  approvals: [
    {
      id: "appr-1",
      company_id: "omniget-core",
      kind: "spend",
      title: "Authorize extra $50 GPU budget for high-definition whisper transcription",
      requested_by_staff_id: "staff-lead",
      note: "Batch transcription of queued 4K video lectures requires burst compute.",
      amount_usd: 50.0,
      created_ms: Date.now() - 3600000 * 2,
    },
    {
      id: "appr-2",
      company_id: "omniget-core",
      kind: "diff",
      title: "Review automated patch for libcurl connection pool leak",
      requested_by_staff_id: "staff-analyst",
      note: "Applies retry exponential backoff on HTTP 429.",
      diff: "--- a/src/net/pool.rs\n+++ b/src/net/pool.rs\n@@ -42,3 +42,5 @@\n+    if status == 429 {\n+        backoff.wait().await;\n+    }",
      created_ms: Date.now() - 1800000,
    },
  ],
  incidents: [
    {
      id: "inc-1",
      company_id: "omniget-core",
      title: "High API latency detected on upstream LLM gateway",
      kind: "rate_limited",
      severity: "warning",
      raised_by: "System Telemetry",
      created_ms: Date.now() - 7200000,
      details: "Claude 3.7 Sonnet response latency exceeded 4500ms over 3 consecutive turns.",
    },
  ],
  issues: [
    {
      id: "inbox-iss-1",
      company_id: "omniget-core",
      issue_id: "issue-1",
      identifier: "OMNI-1",
      title: "Complete v1.0.0 Company & Organization Suite Decompilation",
      why: "review",
      unread: true,
      created_ms: Date.now() - 3600000 * 4,
    },
  ],
};

const mockComments: Record<string, { id: string; author: string; text: string; created_at: number }[]> = {
  "issue-1": [
    {
      id: "c-1",
      author: "Omni Commander",
      text: "Decompiled Category E models, Rust RPC handlers, and first 6 routes. All passing verification.",
      created_at: Date.now() - 3600000,
    },
  ],
};

export async function companyRpc<T = any>(method: string, args: Record<string, any> = {}): Promise<T> {
  // First attempt native backend / IPC bridge
  try {
    const res = await invokeV100("company_rpc", { method, args });
    if (res !== null && res !== undefined && !(typeof res === "object" && Object.keys(res).length === 0 && !Array.isArray(res))) {
      return res as T;
    }
  } catch {
    // Continue to mock fallback
  }

  // Fallback handlers
  switch (method) {
    case "company.list":
      return [...mockCompanies] as any;
    case "company.get": {
      const c = mockCompanies.find((x) => x.id === args.company) ?? mockCompanies[0];
      return c as any;
    }
    case "company.create": {
      const newC: Company = {
        id: `c-${Date.now()}`,
        name: args.company?.name || "New Company",
        mission: args.company?.mission || "",
        issue_prefix: (args.company?.name || "COMP").slice(0, 4).toUpperCase(),
        status: "active",
        workspace: args.company?.workspace || null,
        created_at: Date.now(),
      };
      mockCompanies.push(newC);
      return newC as any;
    }
    case "company.status": {
      const c = mockCompanies.find((x) => x.id === args.company);
      if (c) c.status = args.status;
      return c as any;
    }
    case "company.update": {
      const c = mockCompanies.find((x) => x.id === args.company);
      if (c) Object.assign(c, args.patch || {});
      return c as any;
    }
    case "company.delete": {
      const idx = mockCompanies.findIndex((x) => x.id === args.company);
      if (idx >= 0) mockCompanies.splice(idx, 1);
      return { ok: true } as any;
    }
    case "company.templates":
      return [
        { id: "software", name: "Software Development", summary: "Autonomous coding & review team", roles: ["Architect", "Developer", "Reviewer"] },
        { id: "content", name: "Content & Research", summary: "Media, documentation & knowledge synthesis", roles: ["Researcher", "Copywriter", "Editor"] },
        { id: "operations", name: "Operations & DevOps", summary: "Automated loops, alerts & pipeline telemetry", roles: ["SRE", "Security Auditor", "Support"] },
      ] as any;
    case "company.dashboard": {
      const c = mockCompanies.find((x) => x.id === args.company) ?? mockCompanies[0];
      const runningCount = mockStaff.filter((s) => s.status === "running").length;
      return {
        company: c,
        running: runningCount,
        open_approvals: mockInbox.approvals.length,
        staff: mockStaff,
        goals: mockGoals,
        projects: mockProjects,
        budgets: mockBudgets,
        recent_runs: mockRuns,
        issue_counts: {
          backlog: 0,
          todo: 1,
          in_progress: 1,
          in_review: 0,
          blocked: 0,
          done: 1,
          cancelled: 0,
        },
        costs: {
          cost_usd: 12.45,
          spend_today_usd: 1.2,
          spend_this_week_usd: 4.8,
          spend_this_month_usd: 12.45,
          total_usd: 48.9,
          models: {
            "claude-3-7-sonnet": { calls: 140, tokens: 420000, cost_usd: 8.5 },
            "gpt-4o": { calls: 60, tokens: 180000, cost_usd: 3.95 },
          },
          agents: {
            "Omni Commander": { calls: 120, tokens: 360000, cost_usd: 7.2 },
            "MosCat Companion": { calls: 80, tokens: 240000, cost_usd: 5.25 },
          },
        },
      } as any;
    }
    case "staff.list":
      return [...mockStaff] as any;
    case "staff.hire": {
      const staff: Staff = {
        id: `staff-${Date.now()}`,
        company_id: args.company,
        name: args.staff?.name || "Agent",
        role: args.staff?.role || "Specialist",
        status: "idle",
        reports_to: args.staff?.reports_to || null,
        capabilities: args.staff?.capabilities || ["shell"],
        created_at: Date.now(),
      };
      mockStaff.push(staff);
      return staff as any;
    }
    case "staff.update": {
      const s = mockStaff.find((x) => x.id === args.staff_id);
      if (s) Object.assign(s, args.patch || {});
      return s as any;
    }
    case "staff.fire": {
      const s = mockStaff.find((x) => x.id === args.staff_id);
      if (s) s.status = "terminated";
      return { ok: true } as any;
    }
    case "issue.list":
      return [...mockIssues] as any;
    case "issue.get": {
      const issue = mockIssues.find((i) => i.id === args.id || i.identifier === args.identifier || i.identifier === args.issue);
      return (issue || mockIssues[0]) as any;
    }
    case "issue.create": {
      const num = mockIssues.length + 1;
      const newIssue: Issue = {
        id: `issue-${Date.now()}`,
        company_id: args.company,
        identifier: `OMNI-${num}`,
        title: args.issue?.title || "Untitled Issue",
        description: args.issue?.description || "",
        status: args.issue?.status || "todo",
        priority: args.issue?.priority || "medium",
        assignee_id: args.issue?.assignee_id || null,
        creator: "User",
        created_at: Date.now(),
        tags: args.issue?.tags || [],
      };
      mockIssues.push(newIssue);
      return newIssue as any;
    }
    case "issue.update": {
      const issue = mockIssues.find((i) => i.id === args.id || i.identifier === args.identifier);
      if (issue) Object.assign(issue, args.patch || {});
      return issue as any;
    }
    case "issue.delete": {
      const idx = mockIssues.findIndex((i) => i.id === args.id || i.identifier === args.id);
      if (idx >= 0) mockIssues.splice(idx, 1);
      return { ok: true } as any;
    }
    case "issue.comments": {
      const comments = mockComments[args.issue_id] || [];
      return [...comments] as any;
    }
    case "issue.add_comment": {
      const comments = mockComments[args.issue_id] || [];
      const newComment = {
        id: `c-${Date.now()}`,
        author: args.author || "User",
        text: args.text || "",
        created_at: Date.now(),
      };
      comments.push(newComment);
      mockComments[args.issue_id] = comments;
      return newComment as any;
    }
    case "goal.list":
    case "goals.list":
      return [...mockGoals] as any;
    case "goal.create": {
      const newGoal: Goal = {
        id: `goal-${Date.now()}`,
        company_id: args.company,
        title: args.goal?.title || "New Goal",
        description: args.goal?.description || "",
        level: args.goal?.level || "milestone",
        status: "active",
        owner_staff_id: args.goal?.owner_staff_id || null,
        created_at: Date.now(),
      };
      mockGoals.push(newGoal);
      return newGoal as any;
    }
    case "goal.update": {
      const g = mockGoals.find((x) => x.id === args.id);
      if (g) Object.assign(g, args.patch || {});
      return g as any;
    }
    case "goal.delete": {
      const idx = mockGoals.findIndex((x) => x.id === args.id);
      if (idx >= 0) mockGoals.splice(idx, 1);
      return { ok: true } as any;
    }
    case "project.list":
      return [...mockProjects] as any;
    case "project.create": {
      const p: Project = {
        id: `proj-${Date.now()}`,
        company_id: args.company,
        name: args.project?.name || "Project",
        key: args.project?.key || "PROJ",
        description: args.project?.description || "",
        color: args.project?.color || "var(--blue)",
        workspace_folder: args.project?.workspace_folder || null,
        created_at: Date.now(),
      };
      mockProjects.push(p);
      return p as any;
    }
    case "routine.list":
    case "routines.list":
      return [...mockRoutines] as any;
    case "routine.create": {
      const r: Routine = {
        id: `routine-${Date.now()}`,
        company_id: args.company,
        name: args.routine?.name || "New Routine",
        cron: args.routine?.cron || "0 9 * * *",
        enabled: args.routine?.enabled ?? true,
        agent_id: args.routine?.agent_id || null,
        prompt: args.routine?.prompt || "",
        created_at: Date.now(),
      };
      mockRoutines.push(r);
      return r as any;
    }
    case "routine.update": {
      const r = mockRoutines.find((x) => x.id === args.id);
      if (r) Object.assign(r, args.patch || {});
      return r as any;
    }
    case "routine.delete": {
      const idx = mockRoutines.findIndex((x) => x.id === args.id);
      if (idx >= 0) mockRoutines.splice(idx, 1);
      return { ok: true } as any;
    }
    case "routine.trigger": {
      const run: RunRecord = {
        id: `run-${Date.now()}`,
        company_id: args.company || "omniget-core",
        staff_id: args.staff_id || "staff-analyst",
        source: "routine_manual",
        state: "running",
        liveness: "active",
        cost_micros: 1200,
        started_ms: Date.now(),
        created_ms: Date.now(),
      };
      mockRuns.unshift(run);
      return run as any;
    }
    case "inbox.list":
      return {
        approvals: [...mockInbox.approvals],
        incidents: [...mockInbox.incidents],
        issues: [...mockInbox.issues],
      } as any;
    case "inbox.approve": {
      const idx = mockInbox.approvals.findIndex((a) => a.id === args.id || a.id === args.approval_id);
      if (idx >= 0) mockInbox.approvals.splice(idx, 1);
      return { ok: true } as any;
    }
    case "inbox.reject": {
      const idx = mockInbox.approvals.findIndex((a) => a.id === args.id || a.id === args.approval_id);
      if (idx >= 0) mockInbox.approvals.splice(idx, 1);
      return { ok: true } as any;
    }
    case "incident.resolve": {
      const idx = mockInbox.incidents.findIndex((i) => i.id === args.id);
      if (idx >= 0) mockInbox.incidents.splice(idx, 1);
      return { ok: true } as any;
    }
    case "incident.ack": {
      return { ok: true } as any;
    }
    case "audit.list":
      return [...mockAudit] as any;
    case "audit.verify":
      return { intact: true, lines: mockAudit.length } as any;
    case "audit.csv":
      return { csv: "seq,ts,action,entity,actor\\n1,1700000000,company.created,company,mint\\n" } as any;
    case "wake.list":
      return [] as any;
    case "costs.summary":
      return {
        cost_usd: 12.45,
        spend_today_usd: 1.2,
        spend_this_week_usd: 4.8,
        spend_this_month_usd: 12.45,
        total_usd: 48.9,
        models: {
          "claude-3-7-sonnet": { calls: 140, tokens: 420000, cost_usd: 8.5 },
          "gpt-4o": { calls: 60, tokens: 180000, cost_usd: 3.95 },
        },
        agents: {
          "Omni Commander": { calls: 120, tokens: 360000, cost_usd: 7.2 },
          "MosCat Companion": { calls: 80, tokens: 240000, cost_usd: 5.25 },
        },
      } as any;
    case "import.preview":
      return {
        plan_hash: "mock-preview-hash-12345",
        company: args.options?.name || "Imported Organization",
        issue_prefix: "IMP",
        staff: ["Lead Architect", "Test Engineer"],
        goals: 3,
        projects: 1,
        routines: 2,
        budgets: 1,
        issues: 4,
        warnings: [],
        secrets_to_fill: [],
      } as any;
    case "import.apply": {
      const impCompany: Company = {
        id: `c-imp-${Date.now()}`,
        name: args.options?.name || "Imported Organization",
        mission: args.options?.mission || "Auto-imported enterprise workspace",
        issue_prefix: "IMP",
        status: "active",
        created_at: Date.now(),
      };
      mockCompanies.push(impCompany);
      return impCompany as any;
    }
    default:
      return {} as any;
  }
}

export function onCompanyChanged(companyId: string | null, callback: (payload: any) => void): () => void {
  // In browser, event subscription can be wired to window or tauri listen
  if (typeof window !== "undefined") {
    const handler = (ev: CustomEvent) => {
      if (!companyId || ev.detail?.company_id === companyId) {
        callback(ev.detail);
      }
    };
    window.addEventListener("company-change" as any, handler as any);
    return () => window.removeEventListener("company-change" as any, handler as any);
  }
  return () => {};
}
