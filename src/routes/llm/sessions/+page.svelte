<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    formatRelativeTime,
    type SessionItem,
  } from "$lib/engine/workspace-api";
  import {
    MessageSquare,
    Terminal,
    Code2,
    Search,
    Plus,
    Cpu,
    Clock,
    Zap,
    ChevronRight,
  } from "lucide-svelte";

  let loading = $state(true);
  let sessions = $state<SessionItem[]>([]);
  let searchQuery = $state("");
  let toolFilter = $state<string>("all");

  const tools = [
    { id: "all", label: "All Sessions" },
    { id: "chat", label: "Interactive Chat" },
    { id: "terminal", label: "Shell & Commands" },
    { id: "code", label: "Code Assistant" },
  ];

  async function loadSessions() {
    loading = true;
    try {
      const res = await workspaceRpc<SessionItem[]>("sessions.list");
      sessions = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSessions();
  });

  const filtered = $derived(
    sessions.filter((s) => {
      if (toolFilter !== "all" && s.tool !== toolFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.title.toLowerCase().includes(q) ||
        s.model.toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q)
      );
    })
  );

  function createNewSession(tool: string = "chat") {
    const id = `sess-${Date.now()}`;
    window.location.href = `/llm/sessions/${tool}/${id}`;
  }
</script>

<div class="sessions-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("sessions.title", "Interactive Sessions Hub")}</h1>
      <p class="page-subtitle">
        {t("sessions.subtitle", "Multi-turn conversations, terminal runners, and live reasoning transcripts.")}
      </p>
    </div>
    <div class="head-actions">
      <button type="button" class="btn btn-primary" onclick={() => createNewSession("chat")}>
        <Plus size={15} class="mr-1 inline" />
        {t("sessions.new_chat", "New Chat Session")}
      </button>
      <button type="button" class="btn btn-secondary" onclick={() => createNewSession("terminal")}>
        <Terminal size={15} class="mr-1 inline" />
        {t("sessions.new_terminal", "New Terminal Session")}
      </button>
    </div>
  </header>

  <!-- Controls -->
  <div class="controls-bar">
    <div class="search-box">
      <Search size={16} class="search-icon" />
      <input
        type="search"
        placeholder={t("sessions.search", "Search sessions by title, model or id...")}
        bind:value={searchQuery}
        class="search-input"
      />
    </div>

    <div class="tool-tabs" role="tablist">
      {#each tools as t}
        <button
          type="button"
          class="tab-btn"
          class:active={toolFilter === t.id}
          onclick={() => (toolFilter = t.id)}
        >
          {t.label}
        </button>
      {/each}
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading sessions index...")}</p>
    </div>
  {:else if filtered.length === 0}
    <div class="empty-state">
      <MessageSquare size={44} class="text-muted" />
      <h3>{t("sessions.none", "No sessions found")}</h3>
      <p class="text-muted">{t("sessions.none_hint", "Start a new session to run inference or execute code.")}</p>
      <button type="button" class="btn btn-primary mt-3" onclick={() => createNewSession("chat")}>
        Start Chat Session
      </button>
    </div>
  {:else}
    <div class="sessions-grid">
      {#each filtered as s (s.id)}
        <a href="/llm/sessions/{s.tool}/{s.id}" class="session-card">
          <div class="card-top">
            <div class="tool-badge {s.tool}">
              {#if s.tool === "terminal"}
                <Terminal size={12} class="mr-1 inline" />
              {:else if s.tool === "code"}
                <Code2 size={12} class="mr-1 inline" />
              {:else}
                <MessageSquare size={12} class="mr-1 inline" />
              {/if}
              {s.tool}
            </div>
            <span class="status-indicator {s.status}">{s.status}</span>
          </div>

          <h3 class="session-title">{s.title}</h3>

          <div class="card-meta">
            <div class="meta-item">
              <Cpu size={12} class="opacity-70 mr-1 inline" />
              <span class="model-name">{s.model}</span>
            </div>
            <div class="meta-item">
              <Zap size={12} class="opacity-70 mr-1 inline" />
              <span>{s.token_count.toLocaleString()} tok</span>
            </div>
            <div class="meta-item">
              <Clock size={12} class="opacity-70 mr-1 inline" />
              <span>{formatRelativeTime(s.updated_at)}</span>
            </div>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sessions-page {
    padding: 24px 32px;
    max-width: 1300px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .page-title {
    font-size: 22px;
    font-weight: 700;
    margin: 0 0 4px;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .head-actions {
    display: flex;
    gap: 8px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn-primary:hover {
    background: #388bfd;
  }

  .btn-secondary {
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: var(--text, #fff);
  }

  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    padding: 10px 16px;
  }

  .search-box {
    position: relative;
    flex: 1;
    max-width: 380px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-input {
    width: 100%;
    padding: 6px 12px 6px 32px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .tool-tabs {
    display: flex;
    gap: 4px;
  }

  .tab-btn {
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-muted, #8b949e);
    font-size: 12px;
    cursor: pointer;
  }

  .tab-btn.active {
    background: var(--fill-2, rgba(255, 255, 255, 0.1));
    color: var(--text, #fff);
    font-weight: 600;
  }

  .sessions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .session-card {
    display: flex;
    flex-direction: column;
    padding: 18px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    text-decoration: none;
    color: var(--text, #fff);
    transition: all 0.15s ease;
  }

  .session-card:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .tool-badge {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: capitalize;
  }

  .tool-badge.chat { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .tool-badge.terminal { background: rgba(63, 185, 80, 0.2); color: #3fb950; }
  .tool-badge.code { background: rgba(163, 113, 247, 0.2); color: #a371f7; }

  .status-indicator {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .status-indicator.active { color: #3fb950; }

  .session-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 14px;
  }

  .card-meta {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-muted, #8b949e);
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    padding-top: 10px;
    margin-top: auto;
  }

  .model-name {
    font-family: var(--font-mono, monospace);
  }

  .loading-state, .empty-state {
    text-align: center;
    padding: 60px 20px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: var(--accent, #58a6ff);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 12px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
