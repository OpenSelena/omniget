<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type ReadingItem,
  } from "$lib/engine/workspace-api";
  import {
    BookOpen,
    Plus,
    Search,
    ExternalLink,
    Tag,
    Clock,
    FileText,
    CheckCircle,
    Check,
  } from "lucide-svelte";

  let loading = $state(true);
  let items = $state<ReadingItem[]>([]);
  let searchQuery = $state("");
  let statusFilter = $state<string>("all");
  let showAddModal = $state(false);

  // New item inputs
  let newTitle = $state("");
  let newUrl = $state("");
  let newSummary = $state("");
  let newTags = $state("");

  async function loadReading() {
    loading = true;
    try {
      const res = await workspaceRpc<ReadingItem[]>("reading.list");
      items = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadReading();
  });

  const filtered = $derived(
    items.filter((it) => {
      if (statusFilter !== "all" && it.status !== statusFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        it.title.toLowerCase().includes(q) ||
        it.summary.toLowerCase().includes(q) ||
        it.tags.some((tg) => tg.toLowerCase().includes(q))
      );
    })
  );

  async function addItem() {
    if (!newTitle.trim()) return;
    const tagList = newTags
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);
    const it = await workspaceRpc<ReadingItem>("reading.add", {
      title: newTitle,
      url: newUrl,
      summary: newSummary,
      tags: tagList,
    });
    items = [it, ...items];
    showAddModal = false;
    newTitle = "";
    newUrl = "";
    newSummary = "";
    newTags = "";
  }
</script>

<div class="reading-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("reading.title", "Reading List & Research Digest")}</h1>
      <p class="page-subtitle">
        {t("reading.subtitle", "Extracted papers, documentation digests, and autonomous research takeaways.")}
      </p>
    </div>
    <div class="head-actions">
      <button type="button" class="btn btn-primary" onclick={() => (showAddModal = true)}>
        <Plus size={15} class="mr-1 inline" />
        {t("reading.add_item", "Add Article / Paper")}
      </button>
    </div>
  </header>

  <!-- Filters Bar -->
  <div class="controls-bar">
    <div class="search-box">
      <Search size={16} class="search-icon" />
      <input
        type="search"
        placeholder={t("reading.search", "Search reading digest...")}
        bind:value={searchQuery}
        class="search-input"
      />
    </div>

    <div class="filters">
      <select bind:value={statusFilter} class="filter-select">
        <option value="all">All Statuses</option>
        <option value="unread">Unread</option>
        <option value="reading">Reading</option>
        <option value="analyzed">Analyzed</option>
        <option value="archived">Archived</option>
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading reading lists...")}</p>
    </div>
  {:else if filtered.length === 0}
    <div class="empty-state">
      <BookOpen size={44} class="text-muted" />
      <h3>{t("reading.none", "No articles in reading list")}</h3>
      <p class="text-muted">{t("reading.none_hint", "Add research papers or technical links to extract takeaways.")}</p>
    </div>
  {:else}
    <div class="items-grid">
      {#each filtered as it (it.id)}
        <article class="reading-card">
          <div class="card-top">
            <span class="type-pill {it.source_type}">{it.source_type}</span>
            <span class="status-pill {it.status}">{it.status}</span>
          </div>

          <h2 class="item-title">
            {#if it.source_url}
              <a href={it.source_url} target="_blank" rel="noopener noreferrer">
                {it.title}
                <ExternalLink size={12} class="inline ml-1 opacity-70" />
              </a>
            {:else}
              {it.title}
            {/if}
          </h2>

          <p class="summary-text">{it.summary}</p>

          {#if it.key_takeaways && it.key_takeaways.length > 0}
            <div class="takeaways-box">
              <h4>Key Insights:</h4>
              <ul>
                {#each it.key_takeaways as pt}
                  <li>{pt}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <div class="card-foot">
            <div class="tags">
              {#each it.tags as tag}
                <span class="tag-chip">#{tag}</span>
              {/each}
            </div>
            <div class="meta">
              <span>{it.word_count.toLocaleString()} words</span>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {/if}

  <!-- Add Modal -->
  {#if showAddModal}
    <div class="modal-backdrop" onclick={() => (showAddModal = false)}></div>
    <div class="modal-dialog" role="dialog" aria-modal="true">
      <h2>Add to Reading List</h2>
      <div class="form-group">
        <label for="read-title">Title</label>
        <input
          id="read-title"
          type="text"
          bind:value={newTitle}
          placeholder="e.g. Tree-of-Thought Problem Solving in LLMs"
          class="form-input"
        />
      </div>
      <div class="form-group">
        <label for="read-url">URL (Optional)</label>
        <input
          id="read-url"
          type="url"
          bind:value={newUrl}
          placeholder="https://..."
          class="form-input"
        />
      </div>
      <div class="form-group">
        <label for="read-sum">Brief Summary</label>
        <textarea
          id="read-sum"
          rows="3"
          bind:value={newSummary}
          placeholder="Abstract or key question..."
          class="form-input"
        ></textarea>
      </div>
      <div class="form-group">
        <label for="read-tags">Tags (comma separated)</label>
        <input
          id="read-tags"
          type="text"
          bind:value={newTags}
          placeholder="ai, transformers, svelte"
          class="form-input"
        />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn btn-secondary" onclick={() => (showAddModal = false)}>
          Cancel
        </button>
        <button type="button" class="btn btn-primary" onclick={addItem}>
          Save Article
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .reading-page {
    padding: 24px 32px;
    max-width: 1400px;
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
    gap: 12px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    padding: 10px 16px;
  }

  .search-box {
    position: relative;
    flex: 1;
    max-width: 400px;
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

  .filter-select {
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .items-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .reading-card {
    display: flex;
    flex-direction: column;
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .type-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(56, 139, 253, 0.2);
    color: #58a6ff;
  }

  .status-pill {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .item-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .item-title a {
    color: var(--text, #fff);
    text-decoration: none;
  }

  .item-title a:hover {
    color: var(--accent-hi, #58a6ff);
  }

  .summary-text {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    line-height: 1.5;
    margin: 0 0 12px;
  }

  .takeaways-box {
    padding: 10px 14px;
    border-radius: 8px;
    background: var(--surface-mut, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    margin-bottom: 14px;
    flex: 1;
  }

  .takeaways-box h4 {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--accent-hi, #58a6ff);
  }

  .takeaways-box ul {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--text, #e6edf3);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .card-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 10px;
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .tag-chip {
    margin-right: 6px;
    color: var(--accent-hi, #58a6ff);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 50;
  }

  .modal-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(500px, 90vw);
    background: var(--surface-hi, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.15));
    border-radius: 12px;
    padding: 24px;
    z-index: 51;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-dialog h2 {
    font-size: 18px;
    font-weight: 700;
    margin: 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .form-input {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
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
