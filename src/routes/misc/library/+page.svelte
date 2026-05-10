<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { pluginInvoke } from "$lib/plugin-invoke";

  type MediaKind = "video" | "audio" | "pdf" | "clip";

  type MediaItem = {
    id: number;
    kind: MediaKind;
    title: string;
    file_path: string;
    file_size_bytes: number | null;
    duration_secs: number | null;
    page_count: number | null;
    thumbnail_path: string | null;
    added_at: number;
    last_opened_at: number | null;
    progress_pct: number;
    source_url: string | null;
    metadata_json: string | null;
  };

  type SearchHit = {
    media_item_id: number;
    kind: string;
    title: string;
    file_path: string;
    score: number;
  };

  let items = $state<MediaItem[]>([]);
  let searchHits = $state<SearchHit[]>([]);
  let isSearching = $state(false);
  let totalCount = $state(0);
  let kindFilter = $state<MediaKind | "all">("all");
  let query = $state("");
  let lastError = $state<string | null>(null);
  let loading = $state(true);
  let searchDebounce: number | null = null;

  type TranscribeState = "idle" | "running" | "done" | "error";
  let transcribeStates = $state<Record<number, TranscribeState>>({});
  let transcribeMessages = $state<Record<number, string>>({});
  let transcribeProgress = $state<Record<number, number>>({});
  let transcribeQueueIds = $state<Record<number, number>>({});

  let unlistenStarted: UnlistenFn | null = null;
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  function findItemByPath(path: string): MediaItem | undefined {
    const norm = (p: string) => p.replace(/\\/g, "/").toLowerCase();
    return items.find((it) => norm(it.file_path) === norm(path));
  }

  onMount(() => {
    void refresh();
    void initListeners();
  });

  onDestroy(() => {
    unlistenStarted?.();
    unlistenProgress?.();
    unlistenDone?.();
  });

  async function initListeners() {
    unlistenStarted = await listen("misc:transcribe:started", (e) => {
      const p = e.payload as { queue_id: number; input_path: string };
      const item = findItemByPath(p.input_path);
      if (item) {
        transcribeQueueIds = { ...transcribeQueueIds, [p.queue_id]: item.id };
        transcribeStates = { ...transcribeStates, [item.id]: "running" };
        transcribeMessages = {
          ...transcribeMessages,
          [item.id]: "Extracting audio…",
        };
        transcribeProgress = { ...transcribeProgress, [item.id]: 0 };
      }
    });
    unlistenProgress = await listen("misc:transcribe:progress", (e) => {
      const p = e.payload as { queue_id: number; percent: number };
      const itemId = transcribeQueueIds[p.queue_id];
      if (itemId != null) {
        transcribeProgress = { ...transcribeProgress, [itemId]: p.percent };
        transcribeMessages = {
          ...transcribeMessages,
          [itemId]: `Transcribing… ${p.percent}%`,
        };
      }
    });
    unlistenDone = await listen("misc:transcribe:done", (e) => {
      const p = e.payload as {
        queue_id: number;
        srt_path: string;
        elapsed_s: number;
        segment_count: number;
        language: string;
      };
      const itemId = transcribeQueueIds[p.queue_id];
      if (itemId != null) {
        transcribeStates = { ...transcribeStates, [itemId]: "done" };
        transcribeMessages = {
          ...transcribeMessages,
          [itemId]: `${p.segment_count} segmentos · ${p.elapsed_s.toFixed(1)}s · ${p.language} → ${p.srt_path}`,
        };
        transcribeProgress = { ...transcribeProgress, [itemId]: 100 };
      }
    });
  }

  async function refresh() {
    loading = true;
    lastError = null;
    try {
      const args: Record<string, unknown> = { limit: 200 };
      if (kindFilter !== "all") args.kind = kindFilter;
      items = await pluginInvoke<MediaItem[]>("misc", "misc:library:list", args);
      const c = await pluginInvoke<{ count: number }>("misc", "misc:library:count", {});
      totalCount = c.count;
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function runSearch() {
    if (!query.trim()) {
      isSearching = false;
      searchHits = [];
      return;
    }
    isSearching = true;
    try {
      searchHits = await pluginInvoke<SearchHit[]>("misc", "misc:library:search", {
        query: query.trim(),
        limit: 50,
      });
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  function onQueryInput() {
    if (searchDebounce != null) clearTimeout(searchDebounce);
    searchDebounce = window.setTimeout(() => void runSearch(), 250);
  }

  $effect(() => {
    void kindFilter;
    void refresh();
  });

  async function removeItem(id: number) {
    try {
      await pluginInvoke("misc", "misc:library:remove", { id });
      await refresh();
      if (isSearching) await runSearch();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  async function revealFile(path: string) {
    try {
      await invoke("reveal_file", { path });
    } catch (e) {
      console.warn("[library] reveal_file failed:", e);
    }
  }

  async function transcribeItem(item: MediaItem) {
    if (item.kind !== "video" && item.kind !== "audio" && item.kind !== "clip") {
      return;
    }
    if (transcribeStates[item.id] === "running") return;
    transcribeStates = { ...transcribeStates, [item.id]: "running" };
    transcribeMessages = { ...transcribeMessages, [item.id]: "Starting…" };
    transcribeProgress = { ...transcribeProgress, [item.id]: 0 };
    try {
      await pluginInvoke<{
        srt_path: string;
        elapsed_s: number;
        duration_s: number;
        segment_count: number;
      }>("misc", "misc:transcribe:run", {
        path: item.file_path,
        wordTimestamps: true,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      transcribeStates = { ...transcribeStates, [item.id]: "error" };
      transcribeMessages = { ...transcribeMessages, [item.id]: msg };
      if (msg.includes("no whisper model installed")) {
        lastError = "No whisper model installed. Go to /misc/models and install at least 'Whisper Tiny' (~75 MB).";
      } else if (msg.includes("Error code: -6")) {
        lastError =
          "Whisper falhou no encode (-6). Tenta clip mais longo (>2s) ou um modelo maior. Whisper-tiny pode falhar em audio muito curto.";
      } else {
        lastError = msg;
      }
    }
  }

  function canTranscribe(kind: MediaKind): boolean {
    return kind === "video" || kind === "audio" || kind === "clip";
  }

  function fmtSize(bytes: number | null): string {
    if (!bytes) return "—";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function fmtDuration(secs: number | null): string {
    if (!secs) return "—";
    const total = Math.floor(secs);
    const m = Math.floor(total / 60);
    const s = total % 60;
    if (m >= 60) {
      const h = Math.floor(m / 60);
      const mm = m % 60;
      return `${h}h${String(mm).padStart(2, "0")}`;
    }
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function fmtRel(unixSecs: number): string {
    const diff = Date.now() / 1000 - unixSecs;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return new Date(unixSecs * 1000).toLocaleDateString();
  }

  function kindBadge(kind: MediaKind | string): string {
    switch (kind) {
      case "video": return "🎬";
      case "audio": return "🎵";
      case "pdf": return "📄";
      case "clip": return "✂️";
      default: return "•";
    }
  }
</script>

<section class="library-page">
  <header class="head">
    <h1>Library</h1>
    <p class="subtitle">
      All media you've imported, recorded, or transcribed. {totalCount} item{totalCount === 1 ? "" : "s"}.
    </p>
  </header>

  <div class="controls">
    <input
      type="search"
      class="search-input"
      placeholder="Search title, transcript, notes…"
      bind:value={query}
      oninput={onQueryInput}
    />
    <div class="kind-chips" role="group" aria-label="Filter by kind">
      {#each (["all", "video", "audio", "pdf", "clip"] as const) as k}
        <button
          type="button"
          class="chip"
          class:active={kindFilter === k}
          onclick={() => (kindFilter = k)}
        >
          {k === "all" ? "All" : `${kindBadge(k)} ${k}`}
        </button>
      {/each}
    </div>
  </div>

  {#if lastError}
    <div class="error" role="alert"><strong>Error:</strong> {lastError}</div>
  {/if}

  {#if loading}
    <p class="empty">Loading…</p>
  {:else if isSearching}
    <h2 class="section-title">{searchHits.length} result{searchHits.length === 1 ? "" : "s"} for "{query}"</h2>
    {#if searchHits.length === 0}
      <p class="empty">No matches. Try simpler terms or different keywords.</p>
    {:else}
      <ul class="hit-list">
        {#each searchHits as hit (hit.media_item_id)}
          <li class="hit-row">
            <span class="hit-kind">{kindBadge(hit.kind)}</span>
            <div class="hit-meta">
              <span class="hit-title">{hit.title}</span>
              <span class="hit-path mono">{hit.file_path}</span>
            </div>
            <span class="hit-score mono">{hit.score.toFixed(2)}</span>
            <button type="button" class="btn-secondary" onclick={() => revealFile(hit.file_path)}>
              Open
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if items.length === 0}
    <p class="empty">
      {#if kindFilter !== "all"}
        No {kindFilter} items in library.
      {:else}
        Library empty. Record something in Studio or import a file.
      {/if}
    </p>
  {:else}
    <div class="grid">
      {#each items as item (item.id)}
        <article class="card">
          <div class="card-head">
            <span class="card-kind">{kindBadge(item.kind)}</span>
            <span class="card-title" title={item.title}>{item.title}</span>
          </div>
          <div class="card-meta mono">
            {#if item.kind === "pdf"}
              {item.page_count ?? "?"} pages
            {:else}
              {fmtDuration(item.duration_secs)}
            {/if}
            · {fmtSize(item.file_size_bytes)}
          </div>
          <div class="card-path mono" title={item.file_path}>{item.file_path}</div>
          {#if item.progress_pct > 0}
            <div class="progress" aria-label="Progress">
              <div class="progress-fill" style="width: {(item.progress_pct * 100).toFixed(0)}%"></div>
            </div>
          {/if}
          <div class="card-foot">
            <span class="when">{fmtRel(item.added_at)}</span>
            <div class="actions">
              {#if canTranscribe(item.kind)}
                <button
                  type="button"
                  class="btn-secondary"
                  onclick={() => transcribeItem(item)}
                  disabled={transcribeStates[item.id] === "running"}
                >
                  {#if transcribeStates[item.id] === "running"}
                    Transcribing…
                  {:else if transcribeStates[item.id] === "done"}
                    ✓ Transcribed
                  {:else}
                    Transcribe
                  {/if}
                </button>
              {/if}
              <button type="button" class="btn-secondary" onclick={() => revealFile(item.file_path)}>
                Open
              </button>
              <button type="button" class="btn-danger" onclick={() => removeItem(item.id)}>
                Remove
              </button>
            </div>
          </div>
          {#if transcribeMessages[item.id]}
            <div class="transcribe-msg" class:error={transcribeStates[item.id] === "error"}>
              {#if transcribeStates[item.id] === "running"}
                <div class="progress-line">
                  <div class="progress-bar-mini">
                    <div
                      class="progress-fill-mini"
                      style="width: {transcribeProgress[item.id] ?? 0}%"
                    ></div>
                  </div>
                  <span class="progress-pct">{transcribeProgress[item.id] ?? 0}%</span>
                </div>
              {/if}
              {transcribeMessages[item.id]}
            </div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .library-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 1100px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }
  .head h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }
  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--tertiary);
  }
  .controls {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 13px;
    font-family: inherit;
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .kind-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .chip {
    padding: 5px 12px;
    border: 1px solid var(--input-border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--tertiary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    text-transform: capitalize;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .chip.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, var(--surface));
  }

  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
    font-size: 13px;
  }

  .empty {
    color: var(--tertiary);
    font-size: 13px;
    margin: 0;
    padding: 20px 0;
  }

  .section-title {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--tertiary);
  }

  .hit-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hit-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .hit-kind {
    font-size: 16px;
    flex-shrink: 0;
  }
  .hit-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .hit-title {
    font-size: 13px;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hit-path {
    font-size: 11px;
    color: var(--tertiary);
    opacity: 0.7;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hit-score {
    font-size: 11px;
    color: var(--tertiary);
    flex-shrink: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .card-kind {
    font-size: 16px;
    flex-shrink: 0;
  }
  .card-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .card-meta {
    font-size: 11px;
    color: var(--tertiary);
  }
  .card-path {
    font-size: 10px;
    color: var(--tertiary);
    opacity: 0.65;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .progress {
    height: 3px;
    border-radius: 2px;
    background: color-mix(in oklab, var(--input-border) 60%, transparent);
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
  }
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 4px;
  }
  .when {
    font-size: 10px;
    color: var(--tertiary);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .btn-secondary,
  .btn-danger {
    padding: 5px 10px;
    border-radius: var(--border-radius);
    background: transparent;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn-secondary {
    border: 1px solid var(--input-border);
    color: var(--secondary);
  }
  .btn-secondary:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-danger {
    border: 1px solid color-mix(in oklab, oklch(60% 0.22 25) 40%, var(--input-border));
    color: oklch(70% 0.22 25);
  }
  .btn-danger:hover {
    border-color: oklch(60% 0.22 25);
    background: color-mix(in oklab, oklch(60% 0.22 25) 10%, transparent);
  }
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .transcribe-msg {
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .transcribe-msg.error {
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
  }
  .progress-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .progress-bar-mini {
    flex: 1;
    height: 4px;
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill-mini {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }
  .progress-pct {
    font-size: 10px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
    min-width: 32px;
    text-align: right;
  }

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }
</style>
