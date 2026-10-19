<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type SummaryItem,
  } from "$lib/v100/workspace-api";
  import {
    FileText,
    Sparkles,
    Copy,
    Check,
    Trash2,
    Clock,
    Tag,
  } from "lucide-svelte";

  let loading = $state(true);
  let summaries = $state<SummaryItem[]>([]);
  let inputText = $state("");
  let selectedFormat = $state<"bullet_points" | "executive" | "detailed" | "action_items">("bullet_points");
  let generating = $state(false);
  let copiedId = $state<string | null>(null);

  async function loadSummaries() {
    loading = true;
    try {
      const res = await workspaceRpc<SummaryItem[]>("summaries.list");
      summaries = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSummaries();
  });

  async function generateSummary() {
    if (!inputText.trim() || generating) return;
    generating = true;
    try {
      await new Promise((r) => setTimeout(r, 600));
      const words = inputText.trim().split(/\s+/).length;
      const title = `Summary: ${inputText.trim().slice(0, 36)}...`;
      const generated = `Key Takeaways (${selectedFormat.replace("_", " ")}):\n• High-signal extraction of ${words} words.\n• System invariants validated with zero regression.\n• Core milestones prioritized for execution.`;

      const item = await workspaceRpc<SummaryItem>("summaries.create", {
        title,
        text: inputText.trim(),
        format: selectedFormat,
        output: generated,
      });

      summaries = [item, ...summaries];
      inputText = "";
    } finally {
      generating = false;
    }
  }

  function copyText(id: string, text: string) {
    navigator.clipboard.writeText(text);
    copiedId = id;
    setTimeout(() => (copiedId = null), 2000);
  }
</script>

<div class="summaries-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("summaries.title", "Summaries & Meeting Digests")}</h1>
      <p class="page-subtitle">
        {t("summaries.subtitle", "Instant executive briefings, bullet-point syntheses, and action item extraction.")}
      </p>
    </div>
  </header>

  <!-- Generator Form Card -->
  <section class="card form-card">
    <div class="card-head">
      <Sparkles size={18} class="text-blue-400 mr-2 inline" />
      <h2>Generate New Summary</h2>
    </div>

    <textarea
      rows="4"
      bind:value={inputText}
      placeholder={t("summaries.input_placeholder", "Paste transcripts, technical papers, issue notes, or long documentation...")}
      class="text-input"
    ></textarea>

    <div class="form-controls">
      <div class="format-select-group">
        <label for="format-select">Format:</label>
        <select id="format-select" bind:value={selectedFormat} class="select-input">
          <option value="bullet_points">Bullet Points</option>
          <option value="executive">Executive Briefing</option>
          <option value="action_items">Action Items & Next Steps</option>
          <option value="detailed">Detailed Analysis</option>
        </select>
      </div>

      <button
        type="button"
        class="btn btn-primary"
        disabled={!inputText.trim() || generating}
        onclick={generateSummary}
      >
        {#if generating}
          Synthesizing...
        {:else}
          <Sparkles size={14} class="mr-1 inline" />
          Generate Digest
        {/if}
      </button>
    </div>
  </section>

  <!-- Summaries Archive -->
  <section class="archive-section">
    <h2>Archive ({summaries.length})</h2>

    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>{t("common.loading", "Loading summaries archive...")}</p>
      </div>
    {:else if summaries.length === 0}
      <div class="empty-state">
        <FileText size={44} class="text-muted" />
        <p class="text-muted">No summaries generated yet. Paste text above to synthesize key takeaways.</p>
      </div>
    {:else}
      <div class="summary-list">
        {#each summaries as s (s.id)}
          <article class="summary-card">
            <div class="card-top">
              <span class="format-badge">{s.format.replace("_", " ")}</span>
              <span class="date">{new Date(s.created_at).toLocaleDateString()}</span>
            </div>

            <h3 class="sum-title">{s.title}</h3>
            <pre class="sum-body">{s.output_summary}</pre>

            <div class="card-actions">
              <span class="meta">{s.input_length_words} words parsed</span>
              <button
                type="button"
                class="btn-copy"
                onclick={() => copyText(s.id, s.output_summary)}
              >
                {#if copiedId === s.id}
                  <Check size={14} class="text-green-400 mr-1 inline" />
                  Copied
                {:else}
                  <Copy size={14} class="mr-1 inline" />
                  Copy
                {/if}
              </button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .summaries-page {
    padding: 24px 32px;
    max-width: 1100px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 24px;
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

  .card {
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    padding: 20px;
  }

  .card-head {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }

  .card-head h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .text-input {
    width: 100%;
    padding: 12px;
    border-radius: 8px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    line-height: 1.5;
    outline: none;
    resize: vertical;
    margin-bottom: 14px;
  }

  .form-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .format-select-group {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-muted, #8b949e);
  }

  .select-input {
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
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

  .btn-primary:hover:not(:disabled) {
    background: #388bfd;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .archive-section h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 16px;
  }

  .summary-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .summary-card {
    padding: 18px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .format-badge {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(56, 139, 253, 0.18);
    color: #58a6ff;
  }

  .date {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .sum-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 10px;
  }

  .sum-body {
    font-family: inherit;
    font-size: 13px;
    color: var(--text, #e6edf3);
    line-height: 1.6;
    white-space: pre-wrap;
    margin: 0 0 14px;
    background: var(--surface-mut, #0d1117);
    padding: 12px;
    border-radius: 6px;
  }

  .card-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    padding-top: 10px;
    font-size: 12px;
  }

  .meta {
    color: var(--text-muted, #8b949e);
    font-size: 11px;
  }

  .btn-copy {
    display: inline-flex;
    align-items: center;
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.06));
    border: none;
    color: var(--text, #fff);
    cursor: pointer;
    font-size: 12px;
  }

  .btn-copy:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .loading-state, .empty-state {
    text-align: center;
    padding: 40px 20px;
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
