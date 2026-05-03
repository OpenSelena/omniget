<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { createReadingSession, type ReadingSession } from "$lib/reading-session";
  import {
    applyFocusMode,
    getCursorLine,
    getPaperFilter,
    getReadingTheme,
    setCursorLine,
    setPaperFilter,
    pushReadingTheme,
    popReadingTheme,
  } from "$lib/reader-theme";
  import ReaderThemeMenu from "$lib/reader-components/ReaderThemeMenu.svelte";
  import ReaderTypographyMenu from "$lib/reader-components/ReaderTypographyMenu.svelte";
  import {
    DEFAULT_TYPOGRAPHY,
    fontStack,
    loadBookSettings,
    loadGlobal,
    saveBookSettings,
    saveGlobal,
    type Typography,
  } from "$lib/reader-typography";
  import "$lib/reader-theme.css";

  type Book = {
    id: number;
    file_path: string;
    format: string;
    title: string | null;
    author: string | null;
    last_location: string | null;
  };

  type OpenResult = {
    book_id: number;
    format: string;
    content: string;
    byte_offset: number;
    bytes_returned: number;
    byte_size: number;
    truncated: boolean;
    title: string;
  };

  type ChunkResult = {
    content: string;
    byte_offset: number;
    bytes_returned: number;
    eof: boolean;
  };

  let { book, onBack }: { book: Book; onBack: () => void } = $props();

  let meta = $state<OpenResult | null>(null);
  let chunks = $state<string[]>([]);
  let nextOffset = $state(0);
  let totalBytes = $state(0);
  let eof = $state(true);
  let loadingMore = $state(false);
  let sentinelEl: HTMLDivElement | null = $state(null);
  let loadingBook = $state(true);
  let errorMsg = $state("");
  let session: ReadingSession | null = null;

  let typography = $state<Typography>({ ...DEFAULT_TYPOGRAPHY });
  let typoMenuOpen = $state(false);
  $effect(() => {
    saveGlobal(typography);
    if (book) void saveBookSettings(book.id, typography);
  });
  let readerTheme = $state<string>("app");
  let paperFilter = $state(false);
  let cursorLine = $state(false);
  let focusMode = $state(false);
  let themeMenuOpen = $state(false);
  let cursorY = $state(-1);

  $effect(() => { setPaperFilter(paperFilter); });
  $effect(() => { setCursorLine(cursorLine); });
  $effect(() => { applyFocusMode(focusMode); });

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    if (e.key === "f" || e.key === "F") {
      e.preventDefault();
      focusMode = !focusMode;
    } else if (e.key === "Escape" && focusMode) {
      e.preventDefault();
      focusMode = false;
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (!cursorLine) return;
    cursorY = e.clientY;
  }

  const sizeLabel = $derived(`${typography.font_size}px`);

  async function loadBook() {
    try {
      meta = await pluginInvoke<OpenResult>("study", "study:read:txt:open", {
        bookId: book.id,
      });
      chunks = meta.content ? [meta.content] : [];
      nextOffset = meta.byte_offset + meta.bytes_returned;
      totalBytes = meta.byte_size;
      eof = !meta.truncated;
      const perBook = await loadBookSettings(book.id);
      typography = perBook ?? loadGlobal();
      session = createReadingSession({
        bookId: book.id,
        getLocation: () => book.last_location ?? "",
      });
      await session.start();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      loadingBook = false;
    }
  }

  async function loadMore() {
    if (eof || loadingMore || !meta) return;
    loadingMore = true;
    try {
      const chunk = await pluginInvoke<ChunkResult>("study", "study:read:txt:read_chunk", {
        bookId: book.id,
        byteOffset: nextOffset,
      });
      if (chunk.content) chunks = [...chunks, chunk.content];
      nextOffset = chunk.byte_offset + chunk.bytes_returned;
      eof = chunk.eof || chunk.bytes_returned === 0;
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
      eof = true;
    } finally {
      loadingMore = false;
    }
  }

  $effect(() => {
    if (!sentinelEl || eof) return;
    const el = sentinelEl;
    const obs = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) void loadMore();
      },
      { rootMargin: "600px" },
    );
    obs.observe(el);
    return () => obs.disconnect();
  });

  function zoomIn() {
    typography = { ...typography, font_size: Math.min(32, typography.font_size + 1) };
  }

  function zoomOut() {
    typography = { ...typography, font_size: Math.max(12, typography.font_size - 1) };
  }

  async function openExternal() {
    try {
      await invoke("open_path_default", { path: book.file_path });
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    readerTheme = getReadingTheme();
    paperFilter = getPaperFilter();
    cursorLine = getCursorLine();
    pushReadingTheme();
    loadBook();
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousemove", onMouseMove);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKey);
    window.removeEventListener("mousemove", onMouseMove);
    applyFocusMode(false);
    popReadingTheme();
    void session?.stop(false);
  });
</script>

<section
  class="reader-page"
  data-read-paper={paperFilter ? "1" : "0"}
  data-read-focus={focusMode ? "1" : "0"}
>
  <header class="head">
    <button type="button" class="back-btn" onclick={onBack}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M15 18l-6-6 6-6"></path>
      </svg>
      <span>{$t("study.read.back_to_library")}</span>
    </button>
    <h1 class="title">{book.title ?? book.file_path.split(/[\\/]/).pop()}</h1>
    {#if meta}
      <div class="toolbar">
        <button type="button" class="tool-btn" onclick={zoomOut} title={$t("study.read.zoom_out")}>−</button>
        <button type="button" class="tool-btn tool-label mono">{sizeLabel}</button>
        <button type="button" class="tool-btn" onclick={zoomIn} title={$t("study.read.zoom_in")}>+</button>
        <div class="theme-wrap">
          <button
            type="button"
            class="tool-btn"
            class:active={typoMenuOpen}
            onclick={() => (typoMenuOpen = !typoMenuOpen)}
            title={$t("study.read.typo_title")}
            aria-label={$t("study.read.typo_title")}
          >
            <span style="font-family: serif; font-weight: 600; font-size: 13px;">Aa</span>
          </button>
          {#if typoMenuOpen}
            <ReaderTypographyMenu
              bind:typography
              onClose={() => (typoMenuOpen = false)}
              onReset={() => (typography = { ...DEFAULT_TYPOGRAPHY })}
            />
          {/if}
        </div>
        <span class="sep"></span>
        <div class="theme-wrap">
          <button
            type="button"
            class="tool-btn"
            class:active={themeMenuOpen}
            onclick={() => (themeMenuOpen = !themeMenuOpen)}
            title={$t("study.read.reading_style")}
            aria-label={$t("study.read.reading_style")}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="9"></circle>
              <path d="M12 3a9 9 0 0 0 0 18"></path>
            </svg>
          </button>
          {#if themeMenuOpen}
            <ReaderThemeMenu
              bind:theme={readerTheme}
              bind:paper={paperFilter}
              bind:cursor={cursorLine}
              onClose={() => (themeMenuOpen = false)}
            />
          {/if}
        </div>
        <button
          type="button"
          class="tool-btn"
          class:active={focusMode}
          onclick={() => (focusMode = !focusMode)}
          title={$t("study.read.focus_mode")}
          aria-label={$t("study.read.focus_mode")}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M4 8V4h4"></path>
            <path d="M20 8V4h-4"></path>
            <path d="M4 16v4h4"></path>
            <path d="M20 16v4h-4"></path>
          </svg>
        </button>
        <button type="button" class="tool-btn" onclick={openExternal} title={$t("study.read.open_external")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
            <path d="M15 3h6v6"></path>
            <path d="M10 14L21 3"></path>
          </svg>
        </button>
      </div>
    {/if}
  </header>

  {#if loadingBook}
    <p class="state">{$t("study.read.loading_book")}</p>
  {:else if errorMsg}
    <div class="state">
      <p class="error">{errorMsg}</p>
      <button type="button" class="btn" onclick={openExternal}>{$t("study.read.open_external")}</button>
    </div>
  {:else if meta}
    <div class="viewer">
      <div class="text-body-wrap">
        {#each chunks as chunk, i (i)}
          <pre
            class="text-body"
            style:font-size="{typography.font_size}px"
            style:font-family={fontStack(typography.font_family)}
            style:line-height={typography.line_height}
            style:text-align={typography.justify === "justify" ? "justify" : "left"}
          >{chunk}</pre>
        {/each}
        {#if !eof}
          <div bind:this={sentinelEl} class="sentinel">
            {#if loadingMore}
              <span class="muted small">{$t("study.common.loading")}</span>
            {:else}
              <button type="button" class="btn" onclick={loadMore}>
                {$t("study.read.txt_load_more")}
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
  {#if cursorLine && cursorY > 0}
    <div class="reader-cursor-line" style:top="{cursorY}px"></div>
  {/if}
</section>

<style>
  .reader-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    flex-shrink: 0;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .back-btn:hover {
    color: var(--text);
    background: var(--surface);
  }

  .title {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: 28px;
    padding: 0 6px;
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
  }
  .tool-btn:hover {
    color: var(--text);
    background: var(--surface);
  }
  .tool-label {
    cursor: default;
  }
  .tool-btn.active {
    color: var(--text);
    background: var(--surface);
    border-color: var(--accent);
  }

  .theme-wrap {
    position: relative;
    display: inline-flex;
  }

  .sep {
    display: inline-block;
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 4px;
  }

  .state {
    padding: 40px;
    text-align: center;
    color: var(--text-muted);
  }
  .state .error {
    color: var(--danger);
    font-size: 13px;
  }

  .btn {
    display: inline-block;
    padding: 6px 14px;
    background: transparent;
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    margin-top: 8px;
  }

  .viewer {
    flex: 1;
    overflow-y: auto;
    padding: 32px 48px;
    display: flex;
    justify-content: center;
  }

  .text-body-wrap {
    max-width: 72ch;
    width: 100%;
    display: flex;
    flex-direction: column;
  }

  .text-body {
    width: 100%;
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: "DM Sans", system-ui, sans-serif;
    line-height: 1.7;
    color: var(--text);
    margin: 0;
  }

  .sentinel {
    display: flex;
    justify-content: center;
    padding: 24px 0;
  }

  .muted { color: var(--text-muted); }
  .small { font-size: 11px; }

  .mono {
    font-family: "IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace;
  }
</style>
