<script lang="ts">
  import type { Snippet } from "svelte";
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import { t } from "$lib/engine/workspace-api";
  import { llmRpc } from "$lib/engine/llm-engine-api";
  import {
    Activity,
    Sliders,
    ChevronDown,
    ExternalLink,
    Zap,
    ShieldAlert,
    Cpu,
  } from "lucide-svelte";

  let { children }: { children?: Snippet } = $props();

  let showMonitor = $state(false);
  let telemetry = $state({
    active_streams: 1,
    throughput_tokens_per_sec: 142.8,
    p50_latency_ms: 640,
    total_tokens_24h: 1845000,
    total_cost_24h_usd: 12.45,
  });

  const pathname = $derived($page.url.pathname);

  const mainTabs = [
    { href: "/llm/work", label: "Work", badge: 2 },
    { href: "/llm/threads", label: "Threads" },
    { href: "/llm/reading", label: "Reading" },
    { href: "/llm/missions", label: "Missions" },
    { href: "/llm/sessions", label: "Sessions" },
    { href: "/llm/summaries", label: "Summaries" },
    { href: "/llm/company", label: "Company" },
  ];

  const configureTabs = [
    { href: "/llm/models", label: "Models" },
    { href: "/llm/local", label: "Local Engine" },
    { href: "/llm/remote", label: "Remote" },
    { href: "/llm/accounts", label: "Accounts" },
    { href: "/llm/mcp", label: "MCP" },
    { href: "/llm/tools", label: "Tools" },
    { href: "/llm/skills", label: "Skills" },
    { href: "/llm/catalog", label: "Catalog" },
    { href: "/llm/installed", label: "Installed" },
    { href: "/llm/roster", label: "Roster" },
    { href: "/llm/arena", label: "Arena" },
    { href: "/llm/jobs", label: "Jobs" },
    { href: "/llm/loops", label: "Loops" },
    { href: "/llm/observatory", label: "Observatory" },
    { href: "/llm/usage", label: "Usage" },
    { href: "/llm/retro", label: "Retro" },
    { href: "/llm/memory", label: "Memory" },
  ];

  const isConfigRoute = $derived(
    configureTabs.some((tab) => pathname.startsWith(tab.href))
  );

  async function loadTelemetry() {
    try {
      const stats = await llmRpc<any>("observatory.stats");
      if (stats && stats.throughput_tokens_per_sec) {
        telemetry = stats;
      }
    } catch {
      // fallback
    }
  }

  onMount(() => {
    loadTelemetry();
  });
</script>

<div class="llm-root">
  <header class="workspace-header">
    <nav class="llm-tabs" aria-label="Main LLM navigation">
      {#each mainTabs as tab}
        {@const isActive = pathname.startsWith(tab.href)}
        <a
          href={tab.href}
          class="llm-tab"
          aria-current={isActive ? "page" : undefined}
        >
          {t(`llm.nav.${tab.label.toLowerCase()}`, tab.label)}
          {#if tab.badge}
            <span class="work-badge orange" aria-live="polite">
              <span class="dot" aria-hidden="true"></span>
              {tab.badge}
            </span>
          {/if}
        </a>
      {/each}

      <a
        href="/llm/models"
        class="llm-tab llm-configure"
        aria-current={isConfigRoute ? "page" : undefined}
      >
        <Sliders size={14} class="mr-1 inline" />
        {t("llm.nav.configure", "Ecosystem & Models")}
      </a>
    </nav>

    <div class="header-trailing">
      <!-- Live Telemetry Monitor -->
      <div class="monitor">
        <button
          type="button"
          class="monitor-trigger"
          aria-controls="usage-monitor"
          aria-expanded={showMonitor}
          onclick={() => (showMonitor = !showMonitor)}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            aria-hidden="true"
          >
            <path d="M3 12h4l3-8 4 16 3-8h4" />
          </svg>
          <span class="tok-sec">{telemetry.throughput_tokens_per_sec.toFixed(0)} tok/s</span>
          <span class="p50">p50: {telemetry.p50_latency_ms}ms</span>
          <ChevronDown size={13} class="opacity-70" />
        </button>

        {#if showMonitor}
          <!-- Backdrop close -->
          <button
            type="button"
            class="monitor-backdrop"
            aria-label="Close monitor"
            onclick={() => (showMonitor = false)}
          ></button>

          <section id="usage-monitor" class="monitor-panel" role="region" aria-label="Usage telemetry">
            <h2>{t("llm.monitor.title", "Inference & Rate Telemetry")}</h2>
            <p class="hint">{t("llm.monitor.hint", "Real-time engine metrics and rolling 24h usage.")}</p>

            <div class="source">
              <div class="stat-row">
                <span>{t("llm.monitor.active_streams", "Active Streams")}</span>
                <strong>{telemetry.active_streams}</strong>
              </div>
              <div class="stat-row">
                <span>{t("llm.monitor.tokens_24h", "24h Token Volume")}</span>
                <strong>{(telemetry.total_tokens_24h / 1_000_000).toFixed(2)}M tokens</strong>
              </div>
              <div class="stat-row">
                <span>{t("llm.monitor.spend_24h", "24h Est. Spend")}</span>
                <strong class="text-green">${telemetry.total_cost_24h_usd.toFixed(2)}</strong>
              </div>
            </div>

            <div class="panel-links">
              <a href="/llm/accounts" onclick={() => (showMonitor = false)}>
                {t("llm.monitor.view_accounts", "Manage API Balances & Limits")} &rarr;
              </a>
              <a href="/llm/observatory" onclick={() => (showMonitor = false)}>
                {t("llm.monitor.view_observatory", "Open Observability Dashboard")} &rarr;
              </a>
            </div>
          </section>
        {/if}
      </div>
    </div>
  </header>

  <!-- Secondary Configure Subnav (when in config / tools mode) -->
  {#if isConfigRoute}
    <nav class="workspace-tools" aria-label="Ecosystem tools navigation">
      {#each configureTabs as ctab}
        {@const isCurrent = pathname === ctab.href || pathname.startsWith(ctab.href + "/")}
        <a href={ctab.href} aria-current={isCurrent ? "page" : undefined}>
          {t(`llm.tools.${ctab.label.toLowerCase()}`, ctab.label)}
        </a>
      {/each}
    </nav>
  {/if}

  <main class="llm-body">
    {#if children}
      {@render children()}
    {/if}
  </main>
</div>

<style>
  .llm-root {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-1, #0d1117);
    color: var(--text, #e6edf3);
  }

  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    background: var(--surface, rgba(22, 27, 34, 0.8));
    backdrop-filter: blur(8px);
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .llm-tabs {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }

  .llm-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
    text-decoration: none;
    transition: all 0.15s ease;
  }

  .llm-tab:hover {
    color: var(--text, #fff);
    background: var(--fill-2, rgba(255, 255, 255, 0.05));
  }

  .llm-tab[aria-current="page"] {
    background: var(--accent-soft, rgba(56, 139, 253, 0.15));
    color: var(--accent-hi, #58a6ff);
    font-weight: 600;
  }

  .llm-configure {
    margin-inline-start: 6px;
    border-inline-start: 1px solid var(--separator, rgba(255, 255, 255, 0.15));
    padding-inline-start: 12px;
  }

  .work-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
  }

  .work-badge.orange {
    color: #f0883e;
    background: rgba(240, 136, 62, 0.18);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .header-trailing {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .monitor {
    position: relative;
  }

  .monitor-trigger {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: var(--surface-hi, rgba(30, 36, 46, 0.8));
    color: var(--text, #e6edf3);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    border-radius: 8px;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .monitor-trigger:hover {
    background: var(--surface-hover, rgba(40, 48, 62, 0.9));
  }

  .tok-sec {
    color: #3fb950;
    font-family: var(--font-mono, monospace);
    font-weight: 600;
  }

  .p50 {
    color: var(--text-muted, #8b949e);
    font-size: 11px;
  }

  .monitor-backdrop {
    position: fixed;
    inset: 0;
    z-index: 39;
    background: transparent;
    border: none;
    cursor: default;
  }

  .monitor-panel {
    position: absolute;
    z-index: 40;
    inset-inline-end: 0;
    top: calc(100% + 8px);
    width: min(340px, calc(100vw - 32px));
    padding: 16px;
    background: var(--surface-hi, #161b22);
    color: var(--text, #e6edf3);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.15));
    border-radius: 12px;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.4);
  }

  .monitor-panel h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 4px;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
    margin: 0 0 12px;
  }

  .source {
    padding-block: 8px;
    border-block: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    margin-bottom: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }

  .stat-row span {
    color: var(--text-muted, #8b949e);
  }

  .text-green {
    color: #3fb950;
  }

  .panel-links a {
    display: block;
    padding-block: 4px;
    color: var(--accent-hi, #58a6ff);
    font-size: 12px;
    text-decoration: none;
  }

  .panel-links a:hover {
    text-decoration: underline;
  }

  .workspace-tools {
    display: flex;
    gap: 8px 16px;
    flex-wrap: wrap;
    padding: 8px 16px;
    background: var(--surface-subtle, rgba(22, 27, 34, 0.5));
    border-bottom: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    font-size: 12px;
  }

  .workspace-tools a {
    color: var(--text-muted, #8b949e);
    text-decoration: none;
    padding: 2px 4px;
    border-radius: 4px;
    transition: color 0.15s ease;
  }

  .workspace-tools a:hover {
    color: var(--text, #fff);
  }

  .workspace-tools a[aria-current="page"] {
    color: var(--accent-hi, #58a6ff);
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 4px;
  }

  .llm-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  @media (max-width: 768px) {
    .workspace-header {
      padding: 8px 12px;
    }
    .llm-tab {
      padding: 4px 8px;
      font-size: 12px;
    }
  }
</style>
