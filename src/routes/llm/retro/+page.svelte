<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type RetroItem,
  } from "$lib/v100/llm-engine-api";
  import {
    BookOpen,
    CheckCircle2,
    AlertCircle,
    XCircle,
    RefreshCw,
    Plus,
    User,
    Calendar,
    Sparkles,
    X,
    Save,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let retros = $state<RetroItem[]>([]);

  // Add Retro Modal
  let showAddModal = $state(false);
  let newAgent = $state("Code Engineer");
  let newSummary = $state("");
  let newVerdict = $state<"success" | "partial" | "failed">("success");
  let newRootCause = $state("");
  let newLesson = $state("");
  let saving = $state(false);

  async function loadRetros(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<RetroItem[]>("retro.list");
      retros = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadRetros();
  });

  async function handleAddRetro(e: SubmitEvent) {
    e.preventDefault();
    if (!newSummary.trim() || !newLesson.trim()) return;
    saving = true;
    try {
      const item: RetroItem = {
        id: `retro-${Date.now()}`,
        session_id: `sess-${Math.floor(Math.random() * 900 + 100)}`,
        agent: newAgent,
        summary: newSummary.trim(),
        verdict: newVerdict,
        root_cause: newRootCause.trim() || undefined,
        lesson_learned: newLesson.trim(),
        created_at: Date.now(),
      };
      retros.unshift(item);
      showAddModal = false;
      newSummary = "";
      newRootCause = "";
      newLesson = "";
    } finally {
      saving = false;
    }
  }
</script>

<svelte:head>
  <title>{t("llm.retro.title", "Run Retrospectives")} - OmniGet</title>
</svelte:head>

<div class="retro-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <BookOpen size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.retro.header", "Execution Retrospectives & Post-Mortems")}</h1>
          <span class="count-pill">{retros.length} Recorded</span>
        </div>
        <p class="subtitle">
          {t("llm.retro.sub", "Diagnostic learnings, prompt refinements, and bug post-mortems for continuous agent improvement.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadRetros(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={() => (showAddModal = true)}>
        <Plus size={14} />
        <span>Record Retrospective</span>
      </button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading retrospectives...")}</p>
    </div>
  {:else}
    <div class="retros-list">
      {#each retros as r (r.id)}
        <div class="retro-card">
          <div class="card-head">
            <div class="head-left">
              <span class="verdict-pill verdict-{r.verdict}">
                {#if r.verdict === "success"}
                  <CheckCircle2 size={13} />
                {:else if r.verdict === "partial"}
                  <AlertCircle size={13} />
                {:else}
                  <XCircle size={13} />
                {/if}
                <span>{r.verdict}</span>
              </span>

              <h3 class="retro-summary">{r.summary}</h3>
            </div>

            <div class="retro-meta">
              <span class="agent-name">{r.agent}</span>
              <span>•</span>
              <span class="date-text">{new Date(r.created_at).toLocaleDateString()}</span>
            </div>
          </div>

          <div class="card-body">
            {#if r.root_cause}
              <div class="detail-box">
                <span class="box-label">Root Cause Diagnostic</span>
                <p>{r.root_cause}</p>
              </div>
            {/if}

            <div class="lesson-box">
              <span class="box-label highlight">
                <Sparkles size={12} />
                <span>Actionable Lesson Learned</span>
              </span>
              <p>{r.lesson_learned}</p>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Add Modal -->
  {#if showAddModal}
    <div class="modal-backdrop" onclick={() => (showAddModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>Record Post-Mortem / Retrospective</h3>
          <button class="icon-btn" onclick={() => (showAddModal = false)}>
            <X size={18} />
          </button>
        </div>

        <form onsubmit={handleAddRetro} class="modal-form">
          <div class="form-group">
            <label for="retro-summary">Summary *</label>
            <input
              id="retro-summary"
              type="text"
              placeholder="e.g. Memory leak during video chunk batch downloads"
              bind:value={newSummary}
              required
            />
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="retro-verdict">Outcome Verdict</label>
              <select id="retro-verdict" bind:value={newVerdict}>
                <option value="success">Success / Breakthrough</option>
                <option value="partial">Partial Resolution</option>
                <option value="failed">Failed / Regression</option>
              </select>
            </div>

            <div class="form-group">
              <label for="retro-agent">Agent Involved</label>
              <input
                id="retro-agent"
                type="text"
                bind:value={newAgent}
                required
              />
            </div>
          </div>

          <div class="form-group">
            <label for="retro-root">Root Cause Analysis</label>
            <textarea
              id="retro-root"
              rows="3"
              placeholder="What triggered the anomaly or unexpected behavior?"
              bind:value={newRootCause}
            ></textarea>
          </div>

          <div class="form-group">
            <label for="retro-lesson">Lesson Learned / Invariant Rule *</label>
            <textarea
              id="retro-lesson"
              rows="3"
              placeholder="What rule, prompt change, or invariant prevents recurrence?"
              bind:value={newLesson}
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
              <span>{saving ? "Saving..." : "Record Retrospective"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .retro-page {
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

  .retros-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .retro-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .head-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .verdict-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .verdict-success { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .verdict-partial { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .verdict-failed { background: rgba(239, 68, 68, 0.2); color: #ef4444; }

  .retro-summary {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .retro-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .detail-box,
  .lesson-box {
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .lesson-box {
    border-color: rgba(59, 130, 246, 0.3);
    background: rgba(59, 130, 246, 0.05);
  }

  .box-label {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
    text-transform: uppercase;
    font-weight: 700;
  }

  .box-label.highlight {
    color: var(--primary, #3b82f6);
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .detail-box p,
  .lesson-box p {
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.45;
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
    max-width: 520px;
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
  .form-group select,
  .form-group textarea {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
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
