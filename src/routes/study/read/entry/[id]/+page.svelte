<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
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

  type EntryMeta = {
    id: number;
    series_id: number;
    number: number | null;
    label: string | null;
    page_count: number | null;
    is_read: boolean;
    last_page_read: number;
    sort_index: number;
    series_title: string;
    reading_direction: string;
  };

  type EntryNeighbor = { id: number; sort_index: number };

  type PagesResult = {
    entry_id: number;
    page_count: number;
    pages: { index: number; abs_path?: string; url?: string }[];
    remote?: boolean;
  };

  type ViewMode = "paged" | "scroll" | "spread";

  const entryId = $derived(Number($page.params.id));

  let meta = $state<EntryMeta | null>(null);
  let pages = $state<{ index: number; abs_path?: string; url?: string }[]>([]);

  function pageSrc(p: { abs_path?: string; url?: string } | undefined): string {
    if (!p) return "";
    if (p.abs_path) return convertFileSrc(p.abs_path);
    if (p.url) return p.url;
    return "";
  }
  let neighbors = $state<{ prev?: EntryNeighbor; next?: EntryNeighbor }>({});
  let pageIdx = $state(0);
  let loading = $state(true);
  let errorMsg = $state("");
  let sidebarOpen = $state(false);

  let viewMode = $state<ViewMode>("paged");
  const readingDir = $derived(meta?.reading_direction ?? "ltr");
  const rtl = $derived(readingDir === "rtl");
  const isWebtoon = $derived(readingDir === "vertical");
  const total = $derived(pages.length);

  let readerTheme = $state<string>("app");
  let paperFilter = $state(false);
  let cursorLine = $state(false);
  let focusMode = $state(false);
  let themeMenuOpen = $state(false);
  let cursorY = $state(-1);
  let session: ReadingSession | null = null;

  $effect(() => { setPaperFilter(paperFilter); });
  $effect(() => { setCursorLine(cursorLine); });
  $effect(() => { applyFocusMode(focusMode); });

  const currentSrc = $derived(pageSrc(pages[pageIdx]));
  const spreadLeftSrc = $derived.by(() => {
    if (viewMode !== "spread") return "";
    const left = rtl ? pageIdx + 1 : pageIdx;
    return pageSrc(pages[left]);
  });
  const spreadRightSrc = $derived.by(() => {
    if (viewMode !== "spread") return "";
    const right = rtl ? pageIdx : pageIdx + 1;
    return pageSrc(pages[right]);
  });
  const preloadSrcs = $derived.by(() => {
    const out: string[] = [];
    for (let k = 1; k <= 3; k++) {
      const src = pageSrc(pages[pageIdx + k]);
      if (src) out.push(src);
    }
    return out;
  });

  async function load() {
    loading = true;
    try {
      meta = await pluginInvoke<EntryMeta>("study", "study:read:entry:get", {
        entryId,
      });
      if (isWebtoon) {
        viewMode = "scroll";
      }
      const pagesRes = await pluginInvoke<PagesResult>("study", "study:read:entry:pages", {
        entryId,
      });
      pages = Array.isArray(pagesRes?.pages) ? pagesRes.pages : [];
      pageIdx = Math.max(0, Math.min(pages.length - 1, meta.last_page_read));
      loadNeighbors();

      session = createReadingSession({
        bookId: meta.series_id,
        getLocation: () => JSON.stringify({ entry: entryId, page: pageIdx }),
      });
      await session.start();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function loadNeighbors() {
    if (!meta) return;
    try {
      const arr = await pluginInvoke<{ id: number; sort_index: number }[]>(
        "study",
        "study:read:series:entries",
        { seriesId: meta.series_id },
      );
      const list = Array.isArray(arr) ? arr : [];
      const myIdx = list.findIndex((x) => x.id === entryId);
      neighbors = {
        prev: myIdx > 0 ? list[myIdx - 1] : undefined,
        next: myIdx < list.length - 1 ? list[myIdx + 1] : undefined,
      };
    } catch (e) {
      neighbors = {};
    }
  }

  async function persistProgress() {
    if (!meta) return;
    const reachedEnd = total > 0 && pageIdx >= total - 1;
    try {
      await pluginInvoke("study", "study:read:series:mark_entry", {
        entryId,
        isRead: reachedEnd,
        lastPageRead: pageIdx,
      });
    } catch (e) {
      console.error(e);
    }
  }

  function stepPage(delta: number) {
    if (viewMode === "spread") {
      const next = Math.max(0, Math.min(total - 1, pageIdx + delta * 2));
      if (next === pageIdx) return;
      pageIdx = next;
    } else {
      const next = Math.max(0, Math.min(total - 1, pageIdx + delta));
      if (next === pageIdx) return;
      pageIdx = next;
    }
    void persistProgress();
    session?.notePageChange();
  }

  function goPrev() { stepPage(-1); }
  function goNext() { stepPage(1); }

  async function goPrevEntry() {
    if (session) await session.stop(false);
    if (neighbors.prev) goto(`/study/read/entry/${neighbors.prev.id}`);
  }

  async function goNextEntry() {
    if (session) await session.stop(false);
    if (neighbors.next) goto(`/study/read/entry/${neighbors.next.id}`);
  }

  function toggleSpread() {
    viewMode = viewMode === "spread" ? "paged" : "spread";
  }

  function toggleScroll() {
    viewMode = viewMode === "scroll" ? "paged" : "scroll";
  }

  function onKey(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
    if (tag === "input" || tag === "textarea") return;
    const prevKey = rtl ? "ArrowRight" : "ArrowLeft";
    const nextKey = rtl ? "ArrowLeft" : "ArrowRight";
    if (e.key === prevKey || e.key === "PageUp") {
      e.preventDefault();
      goPrev();
    } else if (e.key === nextKey || e.key === "PageDown" || e.key === " ") {
      e.preventDefault();
      goNext();
    } else if (e.key === "Home") {
      e.preventDefault();
      pageIdx = 0;
      void persistProgress();
    } else if (e.key === "End") {
      e.preventDefault();
      pageIdx = Math.max(0, total - 1);
      void persistProgress();
    } else if (e.key === "[") {
      e.preventDefault();
      void goPrevEntry();
    } else if (e.key === "]") {
      e.preventDefault();
      void goNextEntry();
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

  onMount(() => {
    readerTheme = getReadingTheme();
    pushReadingTheme();
    paperFilter = getPaperFilter();
    cursorLine = getCursorLine();
    load();
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

  function back() {
    if (meta) goto(`/study/read/manga/${meta.series_id}`);
    else goto("/study/read/manga");
  }
</script>

<section
  class="reader-page"
  data-read-paper={paperFilter ? "1" : "0"}
  data-read-focus={focusMode ? "1" : "0"}
>
  <header class="head">
    <button type="button" class="back-btn" onclick={back}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M15 18l-6-6 6-6"></path>
      </svg>
      <span>{$t("study.read.back_to_manga_detail")}</span>
    </button>
    {#if meta}
      <h1 class="title">
        <span class="series">{meta.series_title}</span>
        <span class="sep">·</span>
        <span class="chap">{meta.number != null ? `Ch ${meta.number}` : (meta.label ?? `#${meta.sort_index + 1}`)}</span>
      </h1>
      <div class="toolbar">
        <button type="button" class="tool-btn" onclick={() => (sidebarOpen = !sidebarOpen)} title={$t("study.read.panel_outline")} aria-label="toc">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <path d="M3 6h18M3 12h18M3 18h12"></path>
          </svg>
        </button>
        <button type="button" class="tool-btn" onclick={goPrevEntry} disabled={!neighbors.prev} title={$t("study.read.prev_chapter")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
            <path d="M11 17l-5-5 5-5"></path>
            <path d="M19 17l-5-5 5-5"></path>
          </svg>
        </button>
        <button type="button" class="tool-btn" onclick={goPrev} disabled={pageIdx <= 0} title={$t("study.read.prev_page")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M15 18l-6-6 6-6"></path></svg>
        </button>
        <span class="page-label mono">{$t("study.read.page_of", { current: pageIdx + 1, total })}</span>
        <button type="button" class="tool-btn" onclick={goNext} disabled={pageIdx >= total - 1} title={$t("study.read.next_page")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M9 6l6 6-6 6"></path></svg>
        </button>
        <button type="button" class="tool-btn" onclick={goNextEntry} disabled={!neighbors.next} title={$t("study.read.next_chapter")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
            <path d="M5 17l5-5-5-5"></path>
            <path d="M13 17l5-5-5-5"></path>
          </svg>
        </button>
        <span class="sep-v"></span>
        <button type="button" class="tool-btn" class:active={viewMode === "scroll"} onclick={toggleScroll} title={$t("study.read.view_scroll")} aria-label="scroll">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <path d="M8 6h12M8 12h12M8 18h12M3 6h2M3 12h2M3 18h2"></path>
          </svg>
        </button>
        {#if !isWebtoon}
          <button type="button" class="tool-btn" class:active={viewMode === "spread"} onclick={toggleSpread} title={$t("study.read.spread_toggle")} aria-label="spread">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
              <rect x="4" y="5" width="7" height="14" rx="1"></rect>
              <rect x="13" y="5" width="7" height="14" rx="1"></rect>
            </svg>
          </button>
        {/if}
        {#if rtl}
          <span class="tag">RTL</span>
        {/if}
        <div class="theme-wrap">
          <button
            type="button"
            class="tool-btn"
            class:active={themeMenuOpen}
            onclick={() => (themeMenuOpen = !themeMenuOpen)}
            title={$t("study.read.reading_style")}
            aria-label={$t("study.read.reading_style")}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
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
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <path d="M4 8V4h4"></path>
            <path d="M20 8V4h-4"></path>
            <path d="M4 16v4h4"></path>
            <path d="M20 16v4h-4"></path>
          </svg>
        </button>
      </div>
    {/if}
  </header>

  {#if loading}
    <p class="state muted">{$t("study.read.loading_book")}</p>
  {:else if errorMsg}
    <p class="state error">{errorMsg}</p>
  {:else if pages.length > 0}
    <div class="layout">
      {#if sidebarOpen}
        <aside class="sidebar">
          <span class="panel-title">{$t("study.read.series_chapters")}</span>
          <ul class="thumb-strip">
            {#each pages as p, i (p.index)}
              <li>
                <button
                  type="button"
                  class="thumb-btn"
                  class:active={i === pageIdx}
                  onclick={() => { pageIdx = i; void persistProgress(); }}
                  aria-label={`page ${i + 1}`}
                >
                  <img src={pageSrc(p)} alt="p{i + 1}" loading="lazy" />
                  <span class="thumb-idx mono">{i + 1}</span>
                </button>
              </li>
            {/each}
          </ul>
        </aside>
      {/if}
      <div class="viewer" class:scroll={viewMode === "scroll"} class:rtl>
        {#if viewMode === "scroll"}
          <div class="scroll-stack" class:webtoon={isWebtoon}>
            {#each pages as p (p.index)}
              <img class="stacked" src={pageSrc(p)} alt="p{p.index + 1}" loading="lazy" />
            {/each}
          </div>
        {:else if viewMode === "spread"}
          <div class="spread" class:rtl>
            {#if spreadLeftSrc}
              <img class="page-image half" src={spreadLeftSrc} alt="left" />
            {/if}
            {#if spreadRightSrc}
              <img class="page-image half" src={spreadRightSrc} alt="right" />
            {/if}
          </div>
        {:else if currentSrc}
          <img class="page-image" src={currentSrc} alt="page {pageIdx + 1}" />
        {/if}
        {#each preloadSrcs as src (src)}
          <img class="preload" src={src} alt="" aria-hidden="true" />
        {/each}
      </div>
    </div>
  {:else}
    <p class="state muted">{$t("study.read.entry_empty")}</p>
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
    background: var(--primary);
    color: var(--secondary);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--content-border);
    background: var(--button-elevated);
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    color: var(--tertiary);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }
  .back-btn:hover {
    color: var(--secondary);
    background: var(--sidebar-highlight);
  }
  .title {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    gap: 6px;
    align-items: baseline;
  }
  .series {
    color: var(--secondary);
  }
  .chap {
    color: var(--tertiary);
    font-size: 12px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: 28px;
    padding: 0 6px;
    background: transparent;
    color: var(--tertiary);
    border: 1px solid var(--input-border);
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
  }
  .tool-btn:hover:not(:disabled) {
    color: var(--secondary);
    background: var(--sidebar-highlight);
  }
  .tool-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .tool-btn.active {
    color: var(--accent);
    border-color: var(--accent);
  }
  .page-label {
    font-size: 11px;
    color: var(--tertiary);
    padding: 0 6px;
  }
  .sep-v {
    display: inline-block;
    width: 1px;
    height: 18px;
    background: var(--content-border);
    margin: 0 4px;
  }
  .sep {
    color: var(--tertiary);
  }
  .tag {
    padding: 1px 6px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.4px;
  }
  .state {
    padding: 40px;
    text-align: center;
    color: var(--tertiary);
  }
  .state.error {
    color: var(--error);
  }
  .muted { color: var(--tertiary); }
  .theme-wrap {
    position: relative;
    display: inline-flex;
  }
  .layout {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .sidebar {
    width: 160px;
    flex-shrink: 0;
    background: var(--button-elevated);
    border-right: 1px solid var(--content-border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 8px;
    gap: 6px;
  }
  .panel-title {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--tertiary);
    padding: 4px 4px 8px;
  }
  .thumb-strip {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .thumb-btn {
    position: relative;
    display: block;
    width: 100%;
    padding: 0;
    background: transparent;
    border: 2px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    overflow: hidden;
  }
  .thumb-btn.active {
    border-color: var(--accent);
  }
  .thumb-btn img {
    display: block;
    width: 100%;
    height: auto;
  }
  .thumb-idx {
    position: absolute;
    bottom: 2px;
    right: 2px;
    padding: 1px 4px;
    background: rgba(0, 0, 0, 0.6);
    color: white;
    font-size: 10px;
    border-radius: 3px;
  }
  .viewer {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #111;
    padding: 12px;
    overflow: auto;
  }
  .viewer.scroll {
    display: block;
    align-items: stretch;
  }
  .scroll-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 1000px;
    margin: 0 auto;
  }
  .scroll-stack.webtoon {
    gap: 0;
  }
  .stacked {
    max-width: 100%;
    height: auto;
    display: block;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
  }
  .page-image {
    max-height: 100%;
    max-width: 100%;
    width: auto;
    height: auto;
    object-fit: contain;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
  }
  .spread {
    display: flex;
    flex-direction: row;
    gap: 2px;
    align-items: stretch;
    max-height: 100%;
  }
  .spread.rtl {
    flex-direction: row-reverse;
  }
  .page-image.half {
    max-height: 100%;
    max-width: 50%;
    height: auto;
    width: auto;
  }
  .preload {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  .mono {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-variant-numeric: tabular-nums;
  }
</style>
