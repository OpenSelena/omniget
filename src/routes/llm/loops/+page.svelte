<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type AgentLoop,
  } from "$lib/v100/llm-engine-api";
  import {
    Repeat,
    Play,
    Pause,
    RefreshCw,
    Activity,
    CheckCircle2,
    AlertCircle,
    User,
    Square,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let loops = $state<AgentLoop[]>([]);

  async function loadLoops(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<AgentLoop[]>("loops.list");
      loops = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadLoops();
  });

  async function toggleLoop(loop: AgentLoop) {
    const nextState = loop.state === "running" ? "paused" : "running";
    loop.state = nextState;
    try {
      await llmRpc("loops.toggle", { id: loop.id });
    } catch {
      loop.state = nextState === "running" ? "paused" : "running";
    }
  }

  function stopLoop(loop: AgentLoop) {
    loop.state = "idle";
  }
</script>

<svelte:head>
  <title>{t("llm.loops.title", "Agent Loops")} - OmniGet</title>
</svelte:head>

<div class="loops-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Repeat size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.loops.header", "Autonomous Agent Reasoning Loops")}</h1>
          <span class="count-pill">{loops.length} Active Loops</span>
        </div>
        <p class="subtitle">
          {t("llm.loops.sub", "Iterative self-evaluating execution loops, automated background triggers, and continuous telemetry.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadLoops(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading agent loops...")}</p>
    </div>
  {:else}
    <div class="loops-grid">
      {#each loops as loop (loop.id)}
        <div class="loop-card" class:paused={loop.state === "paused"}>
          <div class="card-head">
            <div>
              <h3 class="loop-name">{loop.name}</h3>
              <div class="agent-tag">
                <User size={12} />
                <span>{loop.agent_name}</span>
              </div>
            </div>

            <div class="state-pill state-{loop.state}">
              <span class="dot"></span>
              <span>{loop.state}</span>
            </div>
          </div>

          <div class="card-body">
            <div class="iter-row">
              <span class="iter-label">Iteration Step</span>
              <strong>{loop.iteration} / {loop.max_iterations}</strong>
            </div>

            <div class="progress-wrap">
              <div
                class="progress-bar"
                style="width: {(loop.iteration / loop.max_iterations) * 100}%;"
              ></div>
            </div>

            {#if loop.current_action}
              <div class="action-box">
                <Activity size={13} class="text-primary" />
                <span>{loop.current_action}</span>
              </div>
            {/if}
          </div>

          <div class="card-footer">
            <button
              class="btn btn-secondary btn-sm"
              onclick={() => toggleLoop(loop)}
            >
              {#if loop.state === "running"}
                <Pause size={13} />
                <span>Pause Loop</span>
              {:else}
                <Play size={13} />
                <span>Resume Loop</span>
              {/if}
            </button>

            <button
              class="btn btn-danger-outline btn-sm"
              onclick={() => stopLoop(loop)}
            >
              <Square size={12} />
              <span>Stop</span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .loops-page {
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

  .loops-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .loop-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.2s ease;
  }

  .loop-card.paused {
    opacity: 0.65;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .loop-name {
    margin: 0 0 4px 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .agent-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .state-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
    text-transform: capitalize;
    font-weight: 600;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
  }

  .state-running { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .state-running .dot { background: #10b981; }

  .state-idle { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .state-idle .dot { background: #8b949e; }

  .state-paused { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .state-paused .dot { background: #f59e0b; }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
  }

  .iter-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .iter-label {
    color: var(--text-muted, #8b949e);
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

  .action-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    background: #0d1117;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    font-size: 0.78rem;
    color: var(--text-normal, #e6edf3);
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
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

  .btn-danger-outline {
    background: transparent;
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
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
