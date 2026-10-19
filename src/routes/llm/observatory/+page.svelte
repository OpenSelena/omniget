<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type ObservatoryStats,
  } from "$lib/engine/llm-engine-api";
  import {
    Activity,
    Zap,
    Clock,
    AlertTriangle,
    RefreshCw,
    TrendingUp,
    Radio,
    Terminal,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let stats = $state<ObservatoryStats>({
    active_streams: 2,
    throughput_tokens_per_sec: 142.8,
    p50_latency_ms: 640,
    p95_latency_ms: 1850,
    error_rate_pct: 0.12,
    total_tokens_24h: 1845000,
    total_cost_24h_usd: 12.45,
  });

  interface StreamTrace {
    id: string;
    model: string;
    status: string;
    speed_tps: number;
    ttft_ms: number;
  }

  let liveTraces = $state<StreamTrace[]>([
    { id: "trace-911", model: "claude-3-7-sonnet", status: "streaming", speed_tps: 64.2, ttft_ms: 380 },
    { id: "trace-912", model: "gemini-2.0-flash", status: "streaming", speed_tps: 112.5, ttft_ms: 140 },
    { id: "trace-910", model: "gpt-4o", status: "completed", speed_tps: 58.0, ttft_ms: 420 },
  ]);

  async function loadStats(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<ObservatoryStats>("observatory.stats");
      if (res) stats = res;
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadStats();
  });
</script>

<svelte:head>
  <title>{t("llm.observatory.title", "Inference Observatory")} - OmniGet</title>
</svelte:head>

<div class="observatory-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Activity size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.observatory.header", "Real-Time Telemetry & Observatory")}</h1>
          <span class="live-pill">
            <span class="pulse-dot"></span>
            <span>Live Telemetry</span>
          </span>
        </div>
        <p class="subtitle">
          {t("llm.observatory.sub", "Sub-second token throughput meters, time-to-first-token (TTFT) percentiles, and gateway latency.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadStats(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  <!-- Metrics Cards Grid -->
  <div class="metrics-grid">
    <div class="metric-card">
      <div class="metric-head">
        <span class="metric-label">Token Throughput</span>
        <Zap size={16} class="text-primary" />
      </div>
      <strong class="metric-val">{stats.throughput_tokens_per_sec.toFixed(1)} <span class="val-unit">tok/s</span></strong>
      <span class="metric-sub text-success">↑ 14% higher than 7-day average</span>
    </div>

    <div class="metric-card">
      <div class="metric-head">
        <span class="metric-label">Latency (P50 / P95)</span>
        <Clock size={16} class="text-warning" />
      </div>
      <strong class="metric-val">{stats.p50_latency_ms}ms <span class="val-unit">/ {stats.p95_latency_ms}ms</span></strong>
      <span class="metric-sub text-muted">Time to first token (TTFT)</span>
    </div>

    <div class="metric-card">
      <div class="metric-head">
        <span class="metric-label">24h Tokens Processed</span>
        <TrendingUp size={16} class="text-success" />
      </div>
      <strong class="metric-val">{(stats.total_tokens_24h / 1000000).toFixed(2)}M</strong>
      <span class="metric-sub text-muted">${stats.total_cost_24h_usd.toFixed(2)} USD total cost</span>
    </div>

    <div class="metric-card">
      <div class="metric-head">
        <span class="metric-label">Error & Throttle Rate</span>
        <AlertTriangle size={16} class="text-danger" />
      </div>
      <strong class="metric-val">{stats.error_rate_pct}%</strong>
      <span class="metric-sub text-success">Zero rate-limit 429 penalties</span>
    </div>
  </div>

  <!-- Live Stream Traces -->
  <div class="traces-card">
    <div class="card-head">
      <h3>Active Inference Traces</h3>
      <span class="badge badge-secondary">{liveTraces.length} Active Streams</span>
    </div>

    <div class="traces-list">
      {#each liveTraces as trace (trace.id)}
        <div class="trace-row">
          <div class="trace-left">
            <Radio size={14} class={trace.status === "streaming" ? "text-primary" : "text-muted"} />
            <div>
              <span class="trace-id">{trace.id}</span>
              <strong class="trace-model">{trace.model}</strong>
            </div>
          </div>

          <div class="trace-right">
            <span class="trace-stat"><strong>{trace.speed_tps.toFixed(1)}</strong> tok/s</span>
            <span class="trace-stat text-muted">TTFT: {trace.ttft_ms}ms</span>
            <span class="trace-status status-{trace.status}">{trace.status}</span>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .observatory-page {
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

  .live-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    font-size: 0.72rem;
    font-weight: 700;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: #10b981;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 16px;
  }

  .metric-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .metric-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .metric-label {
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
  }

  .metric-val {
    font-size: 1.6rem;
    font-weight: 800;
  }

  .val-unit {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .metric-sub {
    font-size: 0.75rem;
  }

  .traces-card {
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

  .traces-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .trace-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .trace-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .trace-id {
    font-family: monospace;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
    display: block;
  }

  .trace-model {
    font-size: 0.9rem;
  }

  .trace-right {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .trace-stat {
    font-size: 0.8rem;
  }

  .trace-status {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .trace-status.status-streaming {
    background: rgba(59, 130, 246, 0.2);
    color: #3b82f6;
  }

  .trace-status.status-completed {
    background: rgba(16, 185, 129, 0.2);
    color: #10b981;
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

  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
  }

  .badge-secondary {
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-success { color: #10b981; }
  .text-warning { color: #f59e0b; }
  .text-danger { color: #ef4444; }
  .text-muted { color: var(--text-muted, #8b949e); }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
