<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import {
    notesPagesListByTag,
    notesSettingsGet,
    notesSettingsSet,
    type PageSummary,
    type TagSummary,
  } from "$lib/notes-bridge";

  type Props = {
    pages: PageSummary[];
    tags: TagSummary[];
    selectedPageId: number | null;
    searchValue: string;
    recentLimit: number;
    onCreate: () => void;
    onSelect: (pageId: number) => void;
    onJournalToday: () => void;
    onSearchInput: (value: string) => void;
    onShowMore: () => void;
  };

  let {
    pages,
    tags,
    selectedPageId,
    searchValue,
    recentLimit,
    onCreate,
    onSelect,
    onJournalToday,
    onSearchInput,
    onShowMore,
  }: Props = $props();

  let favorites = $state<PageSummary[]>([]);
  let sidebarWidth = $state<number | null>(null);
  let resizing = false;
  let resizeStartX = 0;
  let resizeStartWidth = 0;

  const SIDEBAR_WIDTH_KEY = "sidebar:width";
  const SIDEBAR_WIDTH_DEFAULT = 280;
  const SIDEBAR_WIDTH_MIN = 200;
  const SIDEBAR_WIDTH_MAX = 480;

  async function loadFavorites() {
    try {
      favorites = await notesPagesListByTag("favorite");
    } catch (e) {
      console.error("loadFavorites failed", e);
    }
  }

  async function loadWidth() {
    try {
      const r = await notesSettingsGet(SIDEBAR_WIDTH_KEY);
      const v = r.value;
      if (typeof v === "number" && v >= SIDEBAR_WIDTH_MIN && v <= SIDEBAR_WIDTH_MAX) {
        sidebarWidth = v;
      } else {
        sidebarWidth = SIDEBAR_WIDTH_DEFAULT;
      }
    } catch {
      sidebarWidth = SIDEBAR_WIDTH_DEFAULT;
    }
  }

  async function saveWidth(w: number) {
    try {
      await notesSettingsSet({ key: SIDEBAR_WIDTH_KEY, value: w });
    } catch (e) {
      console.error("saveWidth failed", e);
    }
  }

  function onResizeStart(e: PointerEvent) {
    resizing = true;
    resizeStartX = e.clientX;
    resizeStartWidth = sidebarWidth ?? SIDEBAR_WIDTH_DEFAULT;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onResizeMove(e: PointerEvent) {
    if (!resizing) return;
    const next = Math.max(
      SIDEBAR_WIDTH_MIN,
      Math.min(SIDEBAR_WIDTH_MAX, resizeStartWidth + (e.clientX - resizeStartX)),
    );
    sidebarWidth = next;
  }

  function onResizeEnd(e: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    if (sidebarWidth) void saveWidth(sidebarWidth);
  }

  onMount(() => {
    void loadFavorites();
    void loadWidth();
  });

  function fmtDay(secs: number): string {
    if (!secs) return "—";
    const d = new Date(secs * 1000);
    const today = new Date();
    const same =
      d.getFullYear() === today.getFullYear() &&
      d.getMonth() === today.getMonth() &&
      d.getDate() === today.getDate();
    if (same) return "hoje";
    const yest = new Date(today);
    yest.setDate(today.getDate() - 1);
    if (
      d.getFullYear() === yest.getFullYear() &&
      d.getMonth() === yest.getMonth() &&
      d.getDate() === yest.getDate()
    )
      return "ontem";
    return d.toLocaleDateString();
  }

  function matchesSearch(p: PageSummary, q: string): boolean {
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      (p.title ?? "").toLowerCase().includes(q)
    );
  }

  function isCoursePage(p: PageSummary): boolean {
    return p.name.startsWith("course/");
  }

  const query = $derived(searchValue.trim().toLowerCase());

  const sectionFavorites = $derived.by(() =>
    favorites.filter((p) => matchesSearch(p, query)),
  );

  const sectionRecent = $derived.by(() =>
    pages
      .filter((p) => matchesSearch(p, query))
      .filter((p) => !isCoursePage(p))
      .slice()
      .sort((a, b) => b.updated_at - a.updated_at)
      .slice(0, 10),
  );

  const sectionPages = $derived.by(() =>
    pages
      .filter((p) => matchesSearch(p, query))
      .filter((p) => !isCoursePage(p))
      .slice()
      .sort((a, b) =>
        (a.title ?? a.name).localeCompare(b.title ?? b.name, "pt-BR", {
          sensitivity: "base",
        }),
      ),
  );

  const sectionCursos = $derived.by(() =>
    pages
      .filter((p) => matchesSearch(p, query))
      .filter(isCoursePage)
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name, "pt-BR")),
  );

  const sectionTags = $derived.by(() =>
    tags.filter((t) => !query || t.name.toLowerCase().includes(query)),
  );
</script>

<aside
  class="sidebar"
  style:width={sidebarWidth ? `${sidebarWidth}px` : undefined}
>
  <header class="sidebar-head">
    <h2 class="page-title">Notes</h2>
    <button class="btn ghost sm" onclick={onCreate}>
      + Nova
    </button>
  </header>

  <div class="quick-actions">
    <button class="quick" onclick={onJournalToday}>
      <span>📅</span> <span>Journal hoje</span>
    </button>
    <a class="quick" href="/study/notes/journal">
      <span>📆</span> <span>Histórico de journal</span>
    </a>
    <a class="quick" href="/study/notes/search">
      <span>🔎</span> <span>Buscar</span>
    </a>
    <a class="quick" href="/study/notes/graph">
      <span>🕸</span> <span>Grafo</span>
    </a>
    <a class="quick" href="/study/notes/templates">
      <span>📄</span> <span>Templates</span>
    </a>
  </div>

  <input
    class="sidebar-search"
    placeholder={$t("study.library.notes_filter_pages") as string}
    value={searchValue}
    oninput={(e) => onSearchInput((e.target as HTMLInputElement).value)}
  />

  {#if sectionFavorites.length > 0}
    <section class="sidebar-section">
      <h3>★ Favoritas</h3>
      <ul class="page-list">
        {#each sectionFavorites as p (p.id)}
          <li>
            <button
              class="page-row"
              class:active={selectedPageId === p.id}
              onclick={() => onSelect(p.id)}
            >
              <span class="page-name">{p.title ?? p.name}</span>
              <span class="page-meta">
                {p.block_count} bloco{p.block_count === 1 ? "" : "s"} · {fmtDay(p.updated_at)}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if sectionRecent.length > 0}
    <section class="sidebar-section">
      <h3>Recentes</h3>
      <ul class="page-list">
        {#each sectionRecent as p (p.id)}
          <li>
            <button
              class="page-row"
              class:active={selectedPageId === p.id}
              onclick={() => onSelect(p.id)}
            >
              <span class="page-name">{p.title ?? p.name}</span>
              <span class="page-meta">{fmtDay(p.updated_at)}</span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section class="sidebar-section">
    <h3>{$t("study.library.notes_pages_section")}</h3>
    <ul class="page-list">
      {#each sectionPages.slice(0, recentLimit) as p (p.id)}
        <li>
          <button
            class="page-row"
            class:active={selectedPageId === p.id}
            onclick={() => onSelect(p.id)}
          >
            <span class="page-name">{p.title ?? p.name}</span>
            <span class="page-meta">
              {p.block_count} bloco{p.block_count === 1 ? "" : "s"} · {fmtDay(p.updated_at)}
            </span>
          </button>
        </li>
      {:else}
        <li class="empty">{$t("study.library.notes_no_pages")}</li>
      {/each}
    </ul>
    {#if sectionPages.length > recentLimit}
      <button class="btn ghost sm full" onclick={onShowMore}>
        Mostrar mais
      </button>
    {/if}
  </section>

  {#if sectionCursos.length > 0}
    <section class="sidebar-section">
      <h3>📚 Cursos</h3>
      <ul class="page-list">
        {#each sectionCursos as p (p.id)}
          <li>
            <button
              class="page-row"
              class:active={selectedPageId === p.id}
              onclick={() => onSelect(p.id)}
            >
              <span class="page-name">{p.title ?? p.name}</span>
              <span class="page-meta page-name">{p.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if sectionTags.length > 0}
    <section class="sidebar-section">
      <h3>Tags</h3>
      <div class="tag-cloud">
        {#each sectionTags as tag (tag.page_id)}
          <button class="tag-chip" onclick={() => onSelect(tag.page_id)}>
            <span>#{tag.name}</span>
            <span class="tag-count">{tag.ref_count}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <div
    class="resize-handle"
    role="separator"
    aria-label="Redimensionar sidebar"
    aria-orientation="vertical"
    onpointerdown={onResizeStart}
    onpointermove={onResizeMove}
    onpointerup={onResizeEnd}
    onpointercancel={onResizeEnd}
  ></div>
</aside>

<style>
  .sidebar {
    position: relative;
    border-right: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .sidebar-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .sidebar-head h2 {
    margin: 0;
    font-size: 16px;
  }
  .quick-actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .quick {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 13px;
    text-decoration: none;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .quick:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    color: var(--text);
  }
  .sidebar-search {
    padding: 8px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .sidebar-search:focus {
    outline: none;
    border-color: var(--accent);
  }
  .sidebar-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .sidebar-section h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary);
  }
  .page-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .page-row {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background 120ms ease;
  }
  .page-row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .page-row.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .page-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .page-meta {
    font-size: 10px;
    color: var(--tertiary);
  }
  .empty {
    color: var(--tertiary);
    font-size: 12px;
    padding: 8px 0;
  }
  .tag-cloud {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 999px;
    border: 0;
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .tag-chip:hover {
    background: color-mix(in oklab, var(--accent) 18%, transparent);
  }
  .tag-count {
    color: var(--tertiary);
    font-weight: 500;
  }
  .btn {
    padding: 6px 12px;
    border-radius: var(--border-radius);
    border: 1px solid transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms ease, border-color 150ms ease;
  }
  .btn.ghost {
    background: transparent;
    border-color: var(--input-border);
    color: var(--text);
  }
  .btn.ghost:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .btn.sm {
    padding: 4px 10px;
    font-size: 11px;
  }
  .btn.full {
    width: 100%;
  }
  .resize-handle {
    position: absolute;
    top: 0;
    right: -3px;
    bottom: 0;
    width: 6px;
    cursor: ew-resize;
    background: transparent;
    z-index: 10;
    transition: background 120ms ease;
  }
  .resize-handle:hover,
  .resize-handle:active {
    background: color-mix(in oklab, var(--accent) 30%, transparent);
  }
  @media (prefers-reduced-motion: reduce) {
    .quick,
    .page-row,
    .tag-chip,
    .btn {
      transition: none;
    }
  }
</style>
