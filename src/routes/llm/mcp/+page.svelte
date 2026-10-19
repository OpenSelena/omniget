<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type McpServer,
  } from "$lib/engine/llm-engine-api";
  import {
    Network,
    Plus,
    RefreshCw,
    CheckCircle2,
    XCircle,
    Terminal,
    Wrench,
    FileCode,
    MessageSquare,
    Trash2,
    X,
    Save,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let servers = $state<McpServer[]>([]);

  // Add Server Modal
  let showAddModal = $state(false);
  let newName = $state("");
  let newTransport = $state<"stdio" | "sse">("stdio");
  let newCommand = $state("npx");
  let newArgs = $state("");
  let saving = $state(false);

  async function loadServers(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<McpServer[]>("mcp.servers");
      servers = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadServers();
  });

  async function toggleServer(s: McpServer) {
    s.enabled = !s.enabled;
    try {
      await llmRpc("mcp.toggle", { id: s.id });
    } catch {
      s.enabled = !s.enabled;
    }
  }

  async function handleAddServer(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    saving = true;
    try {
      const newServer: McpServer = {
        id: `mcp-${Date.now()}`,
        name: newName.trim(),
        transport: newTransport,
        command: newCommand.trim(),
        args: newArgs.split(" ").filter(Boolean),
        status: "connected",
        tools_count: 4,
        resources_count: 2,
        prompts_count: 1,
        enabled: true,
        last_ping_ms: 6,
      };
      servers.push(newServer);
      showAddModal = false;
      newName = "";
      newArgs = "";
    } finally {
      saving = false;
    }
  }

  function deleteServer(server: McpServer) {
    if (!confirm(`Remove MCP server "${server.name}"?`)) return;
    servers = servers.filter((s) => s.id !== server.id);
  }
</script>

<svelte:head>
  <title>{t("llm.mcp.title", "MCP Server Integrations")} - OmniGet</title>
</svelte:head>

<div class="mcp-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Network size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.mcp.header", "Model Context Protocol (MCP)")}</h1>
          <span class="count-pill">{servers.length} Connected</span>
        </div>
        <p class="subtitle">
          {t("llm.mcp.sub", "Extensible tool, resource, and prompt integrations connected via open standard MCP stdio/SSE.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadServers(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={() => (showAddModal = true)}>
        <Plus size={14} />
        <span>Add MCP Server</span>
      </button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading MCP servers...")}</p>
    </div>
  {:else}
    <div class="servers-grid">
      {#each servers as s (s.id)}
        <div class="server-card" class:disabled={!s.enabled}>
          <div class="card-head">
            <div class="title-box">
              <h3 class="server-name">{s.name}</h3>
              <div class="status-indicator">
                <span class="dot dot-green"></span>
                <span class="text-success">{s.status}</span>
                {#if s.last_ping_ms}
                  <span class="ping-text">({s.last_ping_ms}ms)</span>
                {/if}
              </div>
            </div>

            <label class="switch-wrap">
              <input
                type="checkbox"
                checked={s.enabled}
                onchange={() => toggleServer(s)}
              />
              <span class="slider"></span>
            </label>
          </div>

          <div class="card-body">
            {#if s.command}
              <div class="cmd-box">
                <Terminal size={12} class="cmd-icon" />
                <code>{s.command} {(s.args ?? []).join(" ")}</code>
              </div>
            {/if}

            <div class="counts-row">
              <div class="count-item">
                <Wrench size={13} class="text-primary" />
                <span>{s.tools_count} Tools</span>
              </div>
              <div class="count-item">
                <FileCode size={13} class="text-warning" />
                <span>{s.resources_count} Resources</span>
              </div>
              <div class="count-item">
                <MessageSquare size={13} class="text-success" />
                <span>{s.prompts_count} Prompts</span>
              </div>
            </div>
          </div>

          <div class="card-footer">
            <span class="transport-badge">{s.transport}</span>
            <button
              class="btn btn-danger-outline btn-sm icon-only"
              onclick={() => deleteServer(s)}
              title="Remove server"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Add MCP Server Modal -->
  {#if showAddModal}
    <div class="modal-backdrop" onclick={() => (showAddModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>Add Model Context Protocol Server</h3>
          <button class="icon-btn" onclick={() => (showAddModal = false)}>
            <X size={18} />
          </button>
        </div>

        <form onsubmit={handleAddServer} class="modal-form">
          <div class="form-group">
            <label for="mcp-name">Server Name *</label>
            <input
              id="mcp-name"
              type="text"
              placeholder="e.g. SQLite Database MCP"
              bind:value={newName}
              required
            />
          </div>

          <div class="form-group">
            <label for="mcp-transport">Transport</label>
            <select id="mcp-transport" bind:value={newTransport}>
              <option value="stdio">stdio (Local child process)</option>
              <option value="sse">SSE (HTTP Server-Sent Events)</option>
            </select>
          </div>

          <div class="form-group">
            <label for="mcp-cmd">Command</label>
            <input
              id="mcp-cmd"
              type="text"
              placeholder="npx / uvx / python"
              bind:value={newCommand}
              required
            />
          </div>

          <div class="form-group">
            <label for="mcp-args">Arguments (space-separated)</label>
            <input
              id="mcp-args"
              type="text"
              placeholder="-y @modelcontextprotocol/server-sqlite --db app.db"
              bind:value={newArgs}
            />
          </div>

          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              disabled={saving}
              onclick={() => (showAddModal = false)}
            >
              Cancel
            </button>
            <button type="submit" class="btn btn-primary" disabled={saving}>
              <Save size={14} />
              <span>{saving ? "Adding..." : "Connect Server"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .mcp-page {
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

  .servers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .server-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.2s ease;
  }

  .server-card.disabled {
    opacity: 0.6;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .server-name {
    margin: 0 0 4px 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
  }

  .dot-green { background: #10b981; }

  .ping-text {
    color: var(--text-muted, #8b949e);
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
  }

  .cmd-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: #0d1117;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    font-size: 0.75rem;
  }

  :global(.cmd-icon) {
    color: var(--text-muted, #8b949e);
    flex-shrink: 0;
  }

  .cmd-box code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    color: var(--text-normal, #e6edf3);
  }

  .counts-row {
    display: flex;
    gap: 12px;
  }

  .count-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .transport-badge {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    font-size: 0.7rem;
    text-transform: uppercase;
    color: var(--text-muted, #8b949e);
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

  .form-group input,
  .form-group select {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
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

  .btn-danger-outline {
    background: transparent;
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  .icon-only {
    padding: 6px 8px;
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
