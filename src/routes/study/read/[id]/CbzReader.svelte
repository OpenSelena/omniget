<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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
  import "$lib/reader-theme.css";

  type Book = {
    id: number;
    file_path: string;
    format: string;
    title: string | null;
    author: string | null;
    last_location: string | null;
  };

  type CbzPage = { index: number; abs_path: string };

  type OpenResult = {
    book_id: number;
    format: string;
    page_count: number;
    title: string;
    pages: CbzPage[];
    cover_path: string | null;
    extract_dir: string;
  };

  let { book, onBack }: { book: Book; onBack: () => void } = $props();

  let meta = $state<OpenResult | null>(null);
  let pageIdx = $state(0);
  let imageSrc = $state<string>("");
  let loadingBook = $state(true);
  let errorMsg = $state("");
  let fitMode = $state<"height" | "width">("height");
  let session: ReadingSession | null = null;
  let readerTheme = $state<string>("app");
  let paperFilter = $state(false);
  let cursorLine = $state(false);
  let focusMode = $state(false);
  let themeMenuOpen = $state(false);
  let cursorY = $state(-1);

  type ViewMode = "paged" | "scroll";
  function getViewMode(): ViewMode {
    if (typeof localStorage === "undefined") return "paged";
    return localStorage.getItem("study.read.view_mode") === "scroll" ? "scroll" : "paged";
  }
  let viewMode = $state<ViewMode>("paged");
  $effect(() => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("study.read.view_mode", viewMode);
    }
  });

  function rtlKey(bookId: number): string {
    return `study.read.cbz.rtl.${bookId}`;
  }
  let mangaRtl = $state(false);
  $effect(() => {
    if (typeof localStorage === "undefined") return;
    const raw = localStorage.getItem(rtlKey(book.id));
    mangaRtl = raw === "1";
  });
  function toggleRtl() {
    mangaRtl = !mangaRtl;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(rtlKey(book.id), mangaRtl ? "1" : "0");
    }
  }

  $effect(() => { setPaperFilter(paperFilter); });
  $effect(() => { setCursorLine(cursorLine); });
  $effect(() => { applyFocusMode(focusMode); });

  const total = $derived(meta?.page_count ?? 0);

  function parseLastLocation(raw: string | null): number | null {
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw);
      if (typeof parsed?.page === "number") return parsed.page;
    } catch {}
    const n = Number(raw);
    return Number.isFinite(n) ? n - 1 : null;
  }

  async function loadBook() {
    try {
      meta = await pluginInvoke<OpenResult>("study", "study:read:cbz:open", {
        bookId: book.id,
      });
      const saved = parseLastLocation(book.last_location) ?? 0;
      pageIdx = Math.max(0, Math.min(meta.page_count - 1, saved));
      renderPage();
      session = createReadingSession({
        bookId: book.id,
        getLocation: () => JSON.stringify({ page: pageIdx }),
      });
      await session.start();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      loadingBook = false;
    }
  }

  function renderPage() {
    if (!meta || pageIdx < 0 || pageIdx >= meta.page_count) return;
    imageSrc = convertFileSrc(meta.pages[pageIdx].abs_path);
    persistLocation();
  }

  async function persistLocation() {
    if (!meta) return;
    const pct = total > 0 ? (pageIdx + 1) / total : 0;
    try {
      await pluginInvoke("study", "study:read:books:update_location", {
        bookId: book.id,
        location: JSON.stringify({ page: pageIdx }),
        readingPct: pct,
      });
    } catch (e) {
      console.error("persist location failed", e);
    }
  }

  async function goTo(i: number) {
    if (!meta) return;
    const clamped = Math.max(0, Math.min(meta.page_count - 1, i));
    if (clamped === pageIdx) return;
    pageIdx = clamped;
    renderPage();
    session?.notePageChange();
  }

  function goPrev() { goTo(pageIdx - 1); }
  function goNext() { goTo(pageIdx + 1); }
  function goLogicalPrev() { mangaRtl ? goNext() : goPrev(); }
  function goLogicalNext() { mangaRtl ? goPrev() : goNext(); }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    if (e.key === "ArrowLeft" || e.key === "PageUp") {
      e.preventDefault();
      goLogicalPrev();
    } else if (e.key === "ArrowRight" || e.key === "PageDown" || e.key === " ") {
      e.preventDefault();
      goLogicalNext();
    } else if (e.key === "f" || e.key === "F") {
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

  function toggleFit() {
    fitMode = fitMode === "height" ? "width" : "height";
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
    pushReadingTheme();
    paperFilter = getPaperFilter();
    cursorLine = getCursorLine();
    viewMode = getViewMode();
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
        <button type="button" class="tool-btn" onclick={goPrev} disabled={pageIdx <= 0} title={$t("study.read.prev_page")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 18l-6-6 6-6"></path></svg>
        </button>
        <span class="page-label mono">{$t("study.read.page_of", { current: pageIdx + 1, total: meta.page_count })}</span>
        <button type="button" class="tool-btn" onclick={goNext} disabled={pageIdx >= meta.page_count - 1} title={$t("study.read.next_page")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 6l6 6-6 6"></path></svg>
        </button>
        <span class="sep"></span>
        <button type="button" class="tool-btn" onclick={toggleFit} title={$t("study.read.fit_width")}>
          {fitMode === "height" ? "↕" : "↔"}
        </button>
        <button
          type="button"
          class="tool-btn"
          class:active={mangaRtl}
          onclick={toggleRtl}
          title={$t("study.read.manga_rtl")}
        >
          {mangaRtl ? "←" : "→"}
        </button>
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
        <button
          type="button"
          class="tool-btn"
          class:active={viewMode === "scroll"}
          onclick={() => (viewMode = viewMode === "scroll" ? "paged" : "scroll")}
          title={viewMode === "scroll" ? $t("study.read.view_paged") : $t("study.read.view_scroll")}
          aria-label="view mode"
        >
          {#if viewMode === "scroll"}
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="4" y="5" width="16" height="14" rx="1"></rect>
              <path d="M9 5v14"></path>
            </svg>
          {:else}
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M8 6h12"></path>
              <path d="M8 12h12"></path>
              <path d="M8 18h12"></path>
              <path d="M3 6h2"></path>
              <path d="M3 12h2"></path>
              <path d="M3 18h2"></path>
            </svg>
          {/if}
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
  {:else if meta && viewMode === "scroll"}
    <div class="viewer scroll">
      <div class="scroll-stack">
        {#each meta.pages as page (page.index)}
          <img
            class="page-image stacked"
            src={convertFileSrc(page.abs_path)}
            alt="page {page.index + 1}"
            loading="lazy"
          />
        {/each}
      </div>
    </div>
  {:else if meta && imageSrc}
    <div class="viewer" class:fit-width={fitMode === "width"}>
      <img class="page-image" src={imageSrc} alt="page {pageIdx + 1}" />
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
  .tool-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }
  .tool-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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

  .page-label {
    font-size: 11px;
    color: var(--text-muted);
    padding: 0 6px;
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
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
    background: #111;
    padding: 12px;
  }

  .page-image {
    max-height: 100%;
    width: auto;
    object-fit: contain;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
  }

  .viewer.fit-width .page-image {
    max-height: none;
    max-width: 100%;
    height: auto;
    width: 100%;
  }
  .viewer.scroll {
    display: block;
    padding: 16px;
    background: #111;
    overflow-y: auto;
  }
  .scroll-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 1000px;
    margin: 0 auto;
  }
  .page-image.stacked {
    max-height: none;
    max-width: 100%;
    width: auto;
    height: auto;
    display: block;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
  }

  .mono {
    font-family: "IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace;
  }
</style>
