<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type ToolItem,
  } from "$lib/engine/llm-engine-api";
  import {
    Wrench,
    Search,
    ShieldAlert,
    ShieldCheck,
    CheckCircle2,
    RefreshCw,
    Code,
    Sliders,
    ChevronDown,
    ChevronUp,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let tools = $state<ToolItem[]>([]);

  let searchQuery = $state("");
  let riskFilter = $state<string>("all");
  let expandedId = $state<string | null>(null);

  async function loadTools(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<ToolItem[]>("tools.list");
      tools = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadTools();
  });

  const filteredTools = $derived(
    tools.filter((t) => {
      if (riskFilter !== "all" && t.risk_level !== riskFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        t.name.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        (t.server_name ?? "").toLowerCase().includes(q)
      );
    })
  );

  async function toggleTool(tool: ToolItem) {
    tool.enabled = !tool.enabled;
    try {
      await llmRpc("tools.toggle", { id: tool.id });
    } catch {
      tool.enabled = !tool.enabled;
    }
  }

  function toggleRequireApproval(tool: ToolItem) {
    tool.requires_approval = !tool.requires_approval;
  }
</script>

<svelte:head>
  <title>{t("llm.tools.title", "Function Tools")} - OmniGet</title>
</svelte:head>

<div class="tools-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Wrench size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.tools.header", "Function Calling Tools Registry")}</h1>
          <span class="count-pill">{tools.length} Tools</span>
        </div>
        <p class="subtitle">
          {t("llm.tools.sub", "Deterministic tool functions available to agents, parameter schemas, risk levels, and human-in-the-loop policies.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadTools(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("llm.tools.search_placeholder", "Filter tools by name, description...")}
        bind:value={searchQuery}
      />
    </div>

    <div class="filters-row">
      <select bind:value={riskFilter} class="filter-select">
        <option value="all">All Risk Levels</option>
        <option value="low">Low Risk</option>
        <option value="medium">Medium Risk</option>
        <option value="high">High Risk</option>
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading tool definitions...")}</p>
    </div>
  {:else}
    <div class="tools-list">
      {#each filteredTools as t (t.id)}
        <div class="tool-card" class:disabled={!t.enabled}>
          <div class="tool-main-row">
            <div class="tool-left">
              <span class="risk-badge risk-{t.risk_level}">
                {#if t.risk_level === "low"}
                  <ShieldCheck size={12} />
                {:else}
                  <ShieldAlert size={12} />
                {/if}
                <span>{t.risk_level}</span>
              </span>

              <div>
                <div class="name-line">
                  <h3 class="tool-name">{t.name}</h3>
                  {#if t.server_name}
                    <span class="server-badge">{t.server_name}</span>
                  {/if}
                </div>
                <p class="tool-desc">{t.description}</p>
              </div>
            </div>

            <div class="tool-right">
              <!-- Require Approval Toggle -->
              <label class="approval-checkbox" title="Require user confirmation before execution">
                <input
                  type="checkbox"
                  checked={t.requires_approval}
                  onchange={() => toggleRequireApproval(t)}
                />
                <span>Requires Approval</span>
              </label>

              <!-- Enable Switch -->
              <label class="switch-wrap" title={t.enabled ? "Disable tool" : "Enable tool"}>
                <input
                  type="checkbox"
                  checked={t.enabled}
                  onchange={() => toggleTool(t)}
                />
                <span class="slider"></span>
              </label>

              <!-- Expand Schema Button -->
              <button
                class="icon-btn"
                onclick={() => (expandedId = expandedId === t.id ? null : t.id)}
                title="View Parameters Schema"
              >
                {#if expandedId === t.id}
                  <ChevronUp size={16} />
                {:else}
                  <ChevronDown size={16} />
                {/if}
              </button>
            </div>
          </div>

          {#if expandedId === t.id}
            <div class="schema-box">
              <span class="schema-label">JSON Schema Parameters</span>
              <pre><code>{JSON.stringify(t.parameters, null, 2)}</code></pre>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tools-page {
    padding: 24px;
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .icon-bubble {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    background: var(--bg-surface-elevated, #1e222a);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-color, #2d3340);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h1 {
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0;
  }

  .count-pill {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--primary, #3b82f6);
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--border-color, #2d3340);
    padding-bottom: 12px;
  }

  .search-wrap {
    position: relative;
    min-width: 260px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-wrap input {
    width: 100%;
    padding: 6px 10px 6px 30px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .filter-select {
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .tool-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: opacity 0.2s ease;
  }

  .tool-card.disabled {
    opacity: 0.55;
  }

  .tool-main-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .tool-left {
    display: flex;
    align-items: center;
    gap: 14px;
    flex: 1;
  }

  .risk-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .risk-low { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .risk-medium { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .risk-high { background: rgba(239, 68, 68, 0.2); color: #ef4444; }

  .name-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tool-name {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    font-family: monospace;
  }

  .server-badge {
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
    font-size: 0.7rem;
  }

  .tool-desc {
    margin: 2px 0 0 0;
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
  }

  .tool-right {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .approval-checkbox {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
  }

  .schema-box {
    background: #0d1117;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    padding: 10px 14px;
    font-size: 0.75rem;
  }

  .schema-label {
    font-size: 0.7rem;
    color: var(--text-muted, #8b949e);
    text-transform: uppercase;
    font-weight: 700;
    display: block;
    margin-bottom: 4px;
  }

  .schema-box pre {
    margin: 0;
    font-family: monospace;
  }

  /* Switch */
  .switch-wrap {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
    cursor: pointer;
  }

  .switch-wrap input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    inset: 0;
    background-color: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 20px;
    transition: 0.2s;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 12px;
    width: 12px;
    left: 3px;
    bottom: 3px;
    background-color: var(--text-muted, #8b949e);
    border-radius: 50%;
    transition: 0.2s;
  }

  input:checked + .slider {
    background-color: var(--primary, #3b82f6);
    border-color: var(--primary, #3b82f6);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
    background-color: #fff;
  }

  /* Utilities */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .btn-sm {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .text-primary { color: var(--primary, #3b82f6); }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 0;
    color: var(--text-muted, #8b949e);
  }
</style>
