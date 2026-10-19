<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type ModelInfo,
  } from "$lib/engine/llm-engine-api";
  import {
    Cpu,
    Search,
    Filter,
    CheckCircle2,
    HardDrive,
    Cloud,
    Eye,
    Wrench,
    FileJson,
    Brain,
    RefreshCw,
    ToggleLeft,
    ToggleRight,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let models = $state<ModelInfo[]>([]);

  let searchQuery = $state("");
  let providerFilter = $state<string>("all");
  let filterLocalOnly = $state(false);
  let filterVision = $state(false);
  let filterThinking = $state(false);

  async function loadModels(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<ModelInfo[]>("models.list");
      models = Array.isArray(res) ? res : [];
    } catch (err) {
      console.warn("Failed to load models:", err);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadModels();
  });

  const filteredModels = $derived(
    models.filter((m) => {
      if (filterLocalOnly && !m.is_local) return false;
      if (filterVision && !m.capabilities.vision) return false;
      if (filterThinking && !m.capabilities.thinking) return false;
      if (providerFilter !== "all" && m.provider !== providerFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        m.name.toLowerCase().includes(q) ||
        m.id.toLowerCase().includes(q) ||
        (m.description ?? "").toLowerCase().includes(q)
      );
    })
  );

  async function toggleModel(model: ModelInfo) {
    const prev = model.status;
    model.status = prev === "ready" ? "unavailable" : "ready";
    try {
      await llmRpc("models.toggle", { id: model.id });
    } catch {
      model.status = prev;
    }
  }
</script>

<svelte:head>
  <title>{t("llm.models.title", "Model Registry")} - OmniGet</title>
</svelte:head>

<div class="models-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Cpu size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.models.header", "Model Registry & Capabilities")}</h1>
          <span class="count-pill">{models.length} Models</span>
        </div>
        <p class="subtitle">
          {t("llm.models.sub", "Registry of local neural weights, cloud inference APIs, and multi-modal tool endpoints.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadModels(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  <!-- Filters Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("llm.models.search_placeholder", "Filter models by name, ID...")}
        bind:value={searchQuery}
      />
    </div>

    <div class="filters-row">
      <select bind:value={providerFilter} class="filter-select">
        <option value="all">All Providers</option>
        <option value="anthropic">Anthropic</option>
        <option value="openai">OpenAI</option>
        <option value="google">Google Gemini</option>
        <option value="deepseek">DeepSeek</option>
        <option value="ollama">Ollama (Local)</option>
      </select>

      <button
        class="filter-chip"
        class:active={filterLocalOnly}
        onclick={() => (filterLocalOnly = !filterLocalOnly)}
      >
        <HardDrive size={13} />
        <span>Local Only</span>
      </button>

      <button
        class="filter-chip"
        class:active={filterVision}
        onclick={() => (filterVision = !filterVision)}
      >
        <Eye size={13} />
        <span>Vision</span>
      </button>

      <button
        class="filter-chip"
        class:active={filterThinking}
        onclick={() => (filterThinking = !filterThinking)}
      >
        <Brain size={13} />
        <span>Thinking</span>
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading models...")}</p>
    </div>
  {:else}
    <div class="models-grid">
      {#each filteredModels as m (m.id)}
        <div class="model-card" class:disabled={m.status === "unavailable"}>
          <div class="card-head">
            <div class="head-info">
              <div class="badges-row">
                <span class="provider-pill">{m.provider}</span>
                {#if m.is_local}
                  <span class="local-pill">
                    <HardDrive size={11} />
                    <span>Local</span>
                  </span>
                {:else}
                  <span class="cloud-pill">
                    <Cloud size={11} />
                    <span>Cloud API</span>
                  </span>
                {/if}
              </div>
              <h3 class="model-name">{m.name}</h3>
              <code class="model-id">{m.id}</code>
            </div>

            <button
              class="toggle-btn"
              onclick={() => toggleModel(m)}
              title={m.status === "ready" ? "Disable model" : "Enable model"}
            >
              {#if m.status === "ready"}
                <ToggleRight size={26} class="text-primary" />
              {:else}
                <ToggleLeft size={26} class="text-muted" />
              {/if}
            </button>
          </div>

          {#if m.description}
            <p class="model-desc">{m.description}</p>
          {/if}

          <!-- Context & Specs -->
          <div class="specs-grid">
            <div class="spec-item">
              <span class="spec-label">Context Window</span>
              <strong class="spec-val">{(m.context_length / 1000).toFixed(0)}K tok</strong>
            </div>

            {#if m.input_price_per_m != null}
              <div class="spec-item">
                <span class="spec-label">Pricing (in/out)</span>
                <strong class="spec-val">${m.input_price_per_m} / ${m.output_price_per_m}</strong>
              </div>
            {:else if m.size_gb}
              <div class="spec-item">
                <span class="spec-label">Disk Footprint</span>
                <strong class="spec-val">{m.size_gb} GB ({m.quantization})</strong>
              </div>
            {/if}
          </div>

          <!-- Capabilities -->
          <div class="caps-row">
            {#if m.capabilities.vision}
              <span class="cap-tag"><Eye size={12} /> Vision</span>
            {/if}
            {#if m.capabilities.tools}
              <span class="cap-tag"><Wrench size={12} /> Tool Use</span>
            {/if}
            {#if m.capabilities.json}
              <span class="cap-tag"><FileJson size={12} /> JSON Schema</span>
            {/if}
            {#if m.capabilities.thinking}
              <span class="cap-tag highlight"><Brain size={12} /> Extended Thinking</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .models-page {
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

  .filters-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .filter-select {
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .filter-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-muted, #8b949e);
    font-size: 0.82rem;
    cursor: pointer;
  }

  .filter-chip.active {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--primary, #3b82f6);
    color: var(--text-normal, #e6edf3);
  }

  .models-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .model-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 0.2s ease;
  }

  .model-card.disabled {
    opacity: 0.55;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .badges-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }

  .provider-pill {
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    font-size: 0.72rem;
    text-transform: capitalize;
  }

  .local-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    font-size: 0.72rem;
  }

  .cloud-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.15);
    color: #3b82f6;
    font-size: 0.72rem;
  }

  .model-name {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .model-id {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .toggle-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .model-desc {
    margin: 0;
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
    line-height: 1.4;
  }

  .specs-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding: 10px;
    background: var(--bg-surface, #0d1117);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .spec-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .spec-label {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .spec-val {
    font-size: 0.85rem;
    font-weight: 600;
  }

  .caps-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }

  .cap-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.72rem;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .cap-tag.highlight {
    background: rgba(139, 92, 246, 0.15);
    color: #8b5cf6;
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
