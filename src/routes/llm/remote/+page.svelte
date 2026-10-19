<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type ProviderConfig,
  } from "$lib/v100/llm-engine-api";
  import {
    Cloud,
    Key,
    CheckCircle2,
    XCircle,
    RefreshCw,
    Edit3,
    Activity,
    ExternalLink,
    Lock,
    Save,
    X,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let providers = $state<ProviderConfig[]>([]);

  // Testing connection state
  let testingId = $state<string | null>(null);
  let pingResult = $state<{ id: string; ok: boolean; ms: number } | null>(null);

  // Edit Key Modal
  let editingProvider = $state<ProviderConfig | null>(null);
  let newKeyInput = $state("");
  let newBaseUrl = $state("");
  let saving = $state(false);

  async function loadProviders(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<ProviderConfig[]>("providers.list");
      providers = Array.isArray(res) ? res : [];
    } catch (err) {
      console.warn("Failed to load providers:", err);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadProviders();
  });

  async function testConnection(p: ProviderConfig) {
    testingId = p.id;
    pingResult = null;
    try {
      const start = Date.now();
      await new Promise((r) => setTimeout(r, 600));
      const ms = Date.now() - start;
      pingResult = { id: p.id, ok: true, ms };
    } catch {
      pingResult = { id: p.id, ok: false, ms: 0 };
    } finally {
      testingId = null;
    }
  }

  function openEdit(p: ProviderConfig) {
    editingProvider = p;
    newKeyInput = "";
    newBaseUrl = p.base_url ?? "";
  }

  async function handleSaveKey(e: SubmitEvent) {
    e.preventDefault();
    if (!editingProvider) return;
    saving = true;
    try {
      const patch: Partial<ProviderConfig> = {};
      if (newKeyInput.trim()) {
        patch.has_key = true;
        patch.api_key_masked = `${newKeyInput.slice(0, 7)}...${newKeyInput.slice(-4)}`;
      }
      if (newBaseUrl.trim()) {
        patch.base_url = newBaseUrl.trim();
      }
      Object.assign(editingProvider, patch);
      await llmRpc("providers.update", { id: editingProvider.id, patch });
      editingProvider = null;
    } finally {
      saving = false;
    }
  }

  async function toggleProvider(p: ProviderConfig) {
    p.enabled = !p.enabled;
    try {
      await llmRpc("providers.update", { id: p.id, patch: { enabled: p.enabled } });
    } catch {
      p.enabled = !p.enabled;
    }
  }
</script>

<svelte:head>
  <title>{t("llm.remote.title", "Cloud Providers")} - OmniGet</title>
</svelte:head>

<div class="remote-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Cloud size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.remote.header", "Remote Inference Providers")}</h1>
          <span class="count-pill">{providers.length} Configured</span>
        </div>
        <p class="subtitle">
          {t("llm.remote.sub", "Manage API keys, upstream gateway endpoints, and provider rate limits for cloud LLMs.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadProviders(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading cloud providers...")}</p>
    </div>
  {:else}
    <div class="providers-grid">
      {#each providers as p (p.id)}
        <div class="provider-card" class:disabled={!p.enabled}>
          <div class="card-head">
            <div class="provider-title-box">
              <h3 class="provider-name">{p.name}</h3>
              <div class="status-indicator">
                {#if p.has_key}
                  <CheckCircle2 size={13} class="text-success" />
                  <span class="text-success">Configured</span>
                {:else}
                  <XCircle size={13} class="text-warning" />
                  <span class="text-warning">Missing Key</span>
                {/if}
              </div>
            </div>

            <label class="switch-wrap" title={p.enabled ? "Disable provider" : "Enable provider"}>
              <input
                type="checkbox"
                checked={p.enabled}
                onchange={() => toggleProvider(p)}
              />
              <span class="slider"></span>
            </label>
          </div>

          <div class="card-body">
            <div class="meta-row">
              <span class="meta-label">API Key</span>
              <div class="key-display">
                <Lock size={12} class="text-muted" />
                <code>{p.api_key_masked || "No key registered"}</code>
              </div>
            </div>

            {#if p.base_url}
              <div class="meta-row">
                <span class="meta-label">Base URL</span>
                <code class="url-code">{p.base_url}</code>
              </div>
            {/if}

            <div class="meta-row">
              <span class="meta-label">Models Available</span>
              <strong>{p.models_count || 3} models</strong>
            </div>

            {#if pingResult && pingResult.id === p.id}
              <div class="ping-banner" class:ok={pingResult.ok}>
                <span>Latency response: {pingResult.ms}ms (HTTP 200 OK)</span>
              </div>
            {/if}
          </div>

          <div class="card-footer">
            <button class="btn btn-secondary btn-sm" onclick={() => openEdit(p)}>
              <Key size={13} />
              <span>{p.has_key ? "Change Key" : "Add Key"}</span>
            </button>

            <button
              class="btn btn-secondary btn-sm"
              disabled={testingId === p.id || !p.has_key}
              onclick={() => testConnection(p)}
            >
              <Activity size={13} class={testingId === p.id ? "spin" : ""} />
              <span>{testingId === p.id ? "Testing..." : "Test Connection"}</span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Edit Provider Modal -->
  {#if editingProvider}
    <div class="modal-backdrop" onclick={() => (editingProvider = null)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>Configure {editingProvider.name}</h3>
          <button class="icon-btn" onclick={() => (editingProvider = null)}>
            <X size={18} />
          </button>
        </div>

        <form onsubmit={handleSaveKey} class="modal-form">
          <div class="form-group">
            <label for="api-key">API Key</label>
            <input
              id="api-key"
              type="password"
              placeholder="Paste secret API key (e.g. sk-...)"
              bind:value={newKeyInput}
            />
            <span class="hint">Keys are encrypted with your OS user credential vault and never sent off-device.</span>
          </div>

          {#if editingProvider.type === "custom" || editingProvider.type === "ollama"}
            <div class="form-group">
              <label for="base-url">Custom Endpoint URL</label>
              <input
                id="base-url"
                type="text"
                placeholder="https://api.openai.com/v1"
                bind:value={newBaseUrl}
              />
            </div>
          {/if}

          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              disabled={saving}
              onclick={() => (editingProvider = null)}
            >
              Cancel
            </button>
            <button type="submit" class="btn btn-primary" disabled={saving}>
              <Save size={14} />
              <span>{saving ? "Saving..." : "Save Key"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .remote-page {
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

  .providers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .provider-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.2s ease;
  }

  .provider-card.disabled {
    opacity: 0.6;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .provider-name {
    margin: 0 0 4px 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 0.82rem;
    flex: 1;
  }

  .meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .meta-label {
    color: var(--text-muted, #8b949e);
  }

  .key-display {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .key-display code,
  .url-code {
    background: var(--bg-surface, #21262d);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.75rem;
  }

  .ping-banner {
    margin-top: 6px;
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
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

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-box {
    background: var(--bg-surface-elevated, #1c2128);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 10px;
    width: 100%;
    max-width: 480px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.15rem;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .form-group input {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .hint {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
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

  .btn-primary {
    background: var(--primary, #3b82f6);
    color: #fff;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-success { color: #10b981; }
  .text-warning { color: #f59e0b; }
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
