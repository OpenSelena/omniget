<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    formatMicroUsd,
    formatDuration,
    statusColor,
    type InferenceJob,
  } from "$lib/engine/llm-engine-api";
  import {
    Layers,
    Search,
    RefreshCw,
    Play,
    XCircle,
    CheckCircle2,
    Clock,
    DollarSign,
    Zap,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let jobs = $state<InferenceJob[]>([]);

  let searchQuery = $state("");
  let statusFilter = $state<string>("all");

  async function loadJobs(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<InferenceJob[]>("jobs.list");
      jobs = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadJobs();
  });

  const filteredJobs = $derived(
    jobs.filter((j) => {
      if (statusFilter !== "all" && j.status !== statusFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        j.id.toLowerCase().includes(q) ||
        j.model.toLowerCase().includes(q) ||
        j.prompt_preview.toLowerCase().includes(q)
      );
    })
  );

  function cancelJob(job: InferenceJob) {
    job.status = "cancelled";
  }
</script>

<svelte:head>
  <title>{t("llm.jobs.title", "Inference Jobs Queue")} - OmniGet</title>
</svelte:head>

<div class="jobs-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Layers size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.jobs.header", "Inference Jobs & Queue")}</h1>
          <span class="count-pill">{jobs.length} Total</span>
        </div>
        <p class="subtitle">
          {t("llm.jobs.sub", "Asynchronous generation workloads, background batch jobs, and token execution telemetry.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadJobs(true)} disabled={refreshing}>
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
        placeholder={t("llm.jobs.search_placeholder", "Filter jobs by ID, model, prompt...")}
        bind:value={searchQuery}
      />
    </div>

    <div class="filters-row">
      <select bind:value={statusFilter} class="filter-select">
        <option value="all">All Statuses</option>
        <option value="running">Running</option>
        <option value="queued">Queued</option>
        <option value="completed">Completed</option>
        <option value="failed">Failed</option>
        <option value="cancelled">Cancelled</option>
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading inference jobs...")}</p>
    </div>
  {:else}
    <div class="jobs-table-card">
      <table class="jobs-table">
        <thead>
          <tr>
            <th style="width: 100px;">Job ID</th>
            <th style="width: 140px;">Model</th>
            <th>Prompt Task Preview</th>
            <th style="width: 110px;">Status</th>
            <th style="width: 130px;">Tokens (In/Out)</th>
            <th style="width: 100px;">Duration</th>
            <th style="width: 90px;">Cost</th>
            <th style="width: 90px;">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredJobs as j (j.id)}
            <tr>
              <td>
                <span class="job-id">{j.id}</span>
              </td>
              <td>
                <span class="model-badge">{j.model}</span>
              </td>
              <td>
                <p class="prompt-text">"{j.prompt_preview}"</p>
              </td>
              <td>
                <span class="status-pill status-{j.status}">{j.status}</span>
              </td>
              <td>
                <span class="tokens-info">{j.tokens_in} / {j.tokens_out}</span>
              </td>
              <td>
                <span class="text-muted">{j.duration_ms}ms</span>
              </td>
              <td>
                <strong class="cost-val">${formatMicroUsd(j.cost_micros)}</strong>
              </td>
              <td>
                {#if j.status === "running"}
                  <button
                    class="btn btn-danger-outline btn-sm"
                    onclick={() => cancelJob(j)}
                    title="Cancel Job"
                  >
                    Cancel
                  </button>
                {:else}
                  <button
                    class="btn btn-secondary btn-sm"
                    onclick={() => alert(`Re-running job ${j.id}`)}
                    title="Re-run Job"
                  >
                    Re-run
                  </button>
                {/if}
              </td>
            </tr>
          {/each}

          {#if filteredJobs.length === 0}
            <tr>
              <td colspan="8" class="table-empty">
                No inference jobs match the current filter.
              </td>
            </tr>
          {/if}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .jobs-page {
    padding: 24px;
    max-width: 1300px;
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

  .jobs-table-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    overflow-x: auto;
  }

  .jobs-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .jobs-table th {
    text-align: left;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-color, #2d3340);
    color: var(--text-muted, #8b949e);
    font-weight: 600;
  }

  .jobs-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .job-id {
    font-family: monospace;
    font-weight: 700;
    color: var(--primary, #3b82f6);
  }

  .model-badge {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    font-size: 0.75rem;
  }

  .prompt-text {
    margin: 0;
    font-size: 0.82rem;
    color: var(--text-normal, #e6edf3);
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-pill {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: capitalize;
  }

  .status-completed { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .status-running { background: rgba(59, 130, 246, 0.2); color: #3b82f6; }
  .status-queued { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .status-failed { background: rgba(239, 68, 68, 0.2); color: #ef4444; }
  .status-cancelled { background: rgba(72, 79, 88, 0.2); color: #484f58; }

  .tokens-info {
    font-size: 0.78rem;
    font-family: monospace;
  }

  .cost-val {
    font-size: 0.85rem;
    color: #10b981;
  }

  .table-empty {
    text-align: center;
    padding: 40px;
    color: var(--text-muted, #8b949e);
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
    padding: 4px 8px;
    font-size: 0.75rem;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .btn-danger-outline {
    background: transparent;
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  .text-primary { color: var(--primary, #3b82f6); }
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
