<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type UsageSummary,
  } from "$lib/engine/llm-engine-api";
  import {
    BarChart3,
    DollarSign,
    RefreshCw,
    Calendar,
    Layers,
    Cpu,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let period = $state<"day" | "week" | "month" | "all">("month");
  let summary = $state<UsageSummary | null>(null);

  async function loadUsage(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<UsageSummary>("usage.summary", { period });
      if (res) summary = res;
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadUsage();
  });
</script>

<svelte:head>
  <title>{t("llm.usage.title", "Token Usage & Cost")} - OmniGet</title>
</svelte:head>

<div class="usage-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <BarChart3 size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.usage.header", "Token Usage & Spend Analytics")}</h1>
        </div>
        <p class="subtitle">
          {t("llm.usage.sub", "Detailed token accounting, prompt cache savings, and cost allocation per model.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <div class="period-toggle">
        <button
          class="period-btn"
          class:active={period === "day"}
          onclick={() => { period = "day"; loadUsage(); }}
        >
          Day
        </button>
        <button
          class="period-btn"
          class:active={period === "week"}
          onclick={() => { period = "week"; loadUsage(); }}
        >
          Week
        </button>
        <button
          class="period-btn"
          class:active={period === "month"}
          onclick={() => { period = "month"; loadUsage(); }}
        >
          Month
        </button>
      </div>

      <button class="btn btn-secondary btn-sm" onclick={() => loadUsage(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
      </button>
    </div>
  </header>

  {#if loading || !summary}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Calculating token metrics...")}</p>
    </div>
  {:else}
    <!-- Big Numbers -->
    <div class="kpi-grid">
      <div class="kpi-card">
        <span class="kpi-label">Total Incurred Cost</span>
        <strong class="kpi-val text-primary">${summary.total_cost_usd.toFixed(2)} USD</strong>
        <span class="kpi-sub">Across all inference pipelines</span>
      </div>

      <div class="kpi-card">
        <span class="kpi-label">Total Tokens Consumed</span>
        <strong class="kpi-val text-success">{(summary.total_tokens / 1000000).toFixed(2)}M <span class="val-unit">tokens</span></strong>
        <span class="kpi-sub">Prompt input + output generation</span>
      </div>
    </div>

    <!-- Provider & Model Breakdowns -->
    <div class="breakdown-grid">
      <!-- By Provider -->
      <div class="breakdown-card">
        <div class="card-head">
          <h3>Spend by Provider</h3>
          <Layers size={16} class="text-primary" />
        </div>

        <div class="items-list">
          {#each Object.entries(summary.by_provider) as [pName, pData]}
            {@const pct = Math.round((pData.cost_usd / summary.total_cost_usd) * 100)}
            <div class="stat-row">
              <div class="stat-top">
                <span class="stat-name capitalize">{pName}</span>
                <strong>${pData.cost_usd.toFixed(2)} ({pct}%)</strong>
              </div>
              <div class="progress-wrap">
                <div class="progress-bar" style="width: {pct}%;"></div>
              </div>
              <div class="stat-bottom">
                <span>{(pData.tokens / 1000).toFixed(0)}k tokens</span>
                <span>{pData.requests} requests</span>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- By Model -->
      <div class="breakdown-card">
        <div class="card-head">
          <h3>Spend by Model</h3>
          <Cpu size={16} class="text-warning" />
        </div>

        <div class="items-list">
          {#each Object.entries(summary.by_model) as [mName, mData]}
            {@const pct = Math.round((mData.cost_usd / summary.total_cost_usd) * 100)}
            <div class="stat-row">
              <div class="stat-top">
                <span class="stat-name">{mName}</span>
                <strong>${mData.cost_usd.toFixed(2)} ({pct}%)</strong>
              </div>
              <div class="progress-wrap">
                <div class="progress-bar bar-orange" style="width: {pct}%;"></div>
              </div>
              <div class="stat-bottom">
                <span>{(mData.tokens / 1000).toFixed(0)}k tokens</span>
                <span>{mData.requests} requests</span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Daily Spend History -->
    <div class="history-card">
      <div class="card-head">
        <h3>Daily Inference Timeline</h3>
        <Calendar size={16} class="text-muted" />
      </div>

      <div class="history-table-wrap">
        <table class="history-table">
          <thead>
            <tr>
              <th>Date</th>
              <th>Tokens</th>
              <th>Spend</th>
            </tr>
          </thead>
          <tbody>
            {#each summary.history as day}
              <tr>
                <td><code>{day.date}</code></td>
                <td>{(day.tokens / 1000).toFixed(0)}k tok</td>
                <td><strong>${day.cost_usd.toFixed(2)}</strong></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

<style>
  .usage-page {
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

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .period-toggle {
    display: flex;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 6px;
    padding: 2px;
  }

  .period-btn {
    padding: 4px 10px;
    border-radius: 4px;
    background: transparent;
    border: none;
    color: var(--text-muted, #8b949e);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .period-btn.active {
    background: var(--bg-surface-elevated, #21262d);
    color: var(--text-normal, #e6edf3);
    font-weight: 600;
  }

  .kpi-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px;
  }

  .kpi-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .kpi-label {
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
  }

  .kpi-val {
    font-size: 1.7rem;
    font-weight: 800;
  }

  .val-unit {
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .kpi-sub {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .breakdown-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  @media (max-width: 800px) {
    .breakdown-grid {
      grid-template-columns: 1fr;
    }
  }

  .breakdown-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-head h3 {
    margin: 0;
    font-size: 1.05rem;
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .stat-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.85rem;
  }

  .stat-name {
    font-weight: 500;
  }

  .capitalize {
    text-transform: capitalize;
  }

  .progress-wrap {
    height: 6px;
    background: var(--bg-surface, #0d1117);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary, #3b82f6);
    border-radius: 999px;
  }

  .progress-bar.bar-orange {
    background: #f59e0b;
  }

  .stat-bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .history-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .history-table-wrap {
    overflow-x: auto;
  }

  .history-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .history-table th {
    text-align: left;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-color, #2d3340);
    color: var(--text-muted, #8b949e);
  }

  .history-table td {
    padding: 8px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
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
    padding: 5px 8px;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-success { color: #10b981; }
  .text-muted { color: var(--text-muted, #8b949e); }

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
