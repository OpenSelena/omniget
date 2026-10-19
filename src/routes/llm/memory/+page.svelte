<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type MemoryEntry,
  } from "$lib/engine/llm-engine-api";
  import {
    Brain,
    Search,
    Plus,
    RefreshCw,
    Database,
    Tag,
    Trash2,
    Save,
    X,
    Sparkles,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let memories = $state<MemoryEntry[]>([]);

  let searchQuery = $state("");
  let collectionFilter = $state<string>("all");

  // Add Memory Modal
  let showAddModal = $state(false);
  let newCollection = $state("architecture");
  let newContent = $state("");
  let saving = $state(false);

  async function loadMemories(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<MemoryEntry[]>("memory.list");
      memories = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadMemories();
  });

  const filteredMemories = $derived(
    memories.filter((m) => {
      if (collectionFilter !== "all" && m.collection !== collectionFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return m.content.toLowerCase().includes(q) || m.collection.toLowerCase().includes(q);
    })
  );

  async function handleAddMemory(e: SubmitEvent) {
    e.preventDefault();
    if (!newContent.trim()) return;
    saving = true;
    try {
      const item: MemoryEntry = {
        id: `mem-${Date.now()}`,
        collection: newCollection.trim().toLowerCase(),
        content: newContent.trim(),
        vector_dim: 1536,
        created_at: Date.now(),
      };
      memories.unshift(item);
      showAddModal = false;
      newContent = "";
    } finally {
      saving = false;
    }
  }

  function deleteMemory(item: MemoryEntry) {
    if (!confirm("Delete this memory entry from vector index?")) return;
    memories = memories.filter((m) => m.id !== item.id);
  }
</script>

<svelte:head>
  <title>{t("llm.memory.title", "Vector Memory Bank")} - OmniGet</title>
</svelte:head>

<div class="memory-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Brain size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.memory.header", "Long-Term Vector Memory Bank")}</h1>
          <span class="count-pill">{memories.length} Facts Stored</span>
        </div>
        <p class="subtitle">
          {t("llm.memory.sub", "Semantic associative memory, cross-session knowledge retention, and cosine similarity lookup.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadMemories(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={() => (showAddModal = true)}>
        <Plus size={14} />
        <span>Store New Fact</span>
      </button>
    </div>
  </header>

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("llm.memory.search_placeholder", "Search memory embeddings...")}
        bind:value={searchQuery}
      />
    </div>

    <div class="filters-row">
      <select bind:value={collectionFilter} class="filter-select">
        <option value="all">All Collections</option>
        <option value="architecture">Architecture</option>
        <option value="rules">Engineering Rules</option>
        <option value="preferences">User Preferences</option>
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Querying vector memory...")}</p>
    </div>
  {:else}
    <div class="memories-grid">
      {#each filteredMemories as m (m.id)}
        <div class="memory-card">
          <div class="card-head">
            <span class="collection-pill">{m.collection}</span>
            <div class="vector-spec">
              <Database size={11} />
              <span>{m.vector_dim} dim (cosine)</span>
            </div>
          </div>

          <p class="memory-content">{m.content}</p>

          <div class="card-footer">
            <span class="date-text">{new Date(m.created_at).toLocaleDateString()}</span>
            <button
              class="btn btn-danger-outline btn-sm icon-only"
              onclick={() => deleteMemory(m)}
              title="Delete fact"
            >
              <Trash2 size={13} />
            </button>
          </div>
        </div>
      {/each}

      {#if filteredMemories.length === 0}
        <div class="empty-state">
          <Brain size={36} class="text-muted" />
          <h4>No vector memories found</h4>
          <p>Store project conventions, engineering invariants, or user preferences here.</p>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Add Memory Modal -->
  {#if showAddModal}
    <div class="modal-backdrop" onclick={() => (showAddModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>Store Long-Term Fact</h3>
          <button class="icon-btn" onclick={() => (showAddModal = false)}>
            <X size={18} />
          </button>
        </div>

        <form onsubmit={handleAddMemory} class="modal-form">
          <div class="form-group">
            <label for="mem-col">Collection Namespace</label>
            <input
              id="mem-col"
              type="text"
              placeholder="e.g. architecture, conventions, security"
              bind:value={newCollection}
              required
            />
          </div>

          <div class="form-group">
            <label for="mem-content">Memory Text / Invariant Fact *</label>
            <textarea
              id="mem-content"
              rows="4"
              placeholder="Enter factual context or architectural constraint to index into agent memory..."
              bind:value={newContent}
              required
            ></textarea>
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
              <span>{saving ? "Embedding..." : "Store in Memory"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .memory-page {
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

  .memories-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .memory-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .collection-pill {
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.15);
    color: var(--primary, #3b82f6);
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .vector-spec {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .memory-content {
    margin: 0;
    font-size: 0.88rem;
    line-height: 1.5;
    color: var(--text-normal, #e6edf3);
    flex: 1;
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .date-text {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .icon-only {
    padding: 5px 8px;
  }

  .empty-state {
    grid-column: 1 / -1;
    text-align: center;
    padding: 40px;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px dashed var(--border-color, #2d3340);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .empty-state h4 {
    margin: 0;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
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
    max-width: 500px;
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
  .form-group textarea {
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
