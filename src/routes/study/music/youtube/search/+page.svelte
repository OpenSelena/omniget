<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import YoutubeShelf from "$lib/study-music-youtube-components/YoutubeShelf.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";
  import YoutubeError from "$lib/study-music-youtube-components/YoutubeError.svelte";
  import ChipsRow from "$lib/study-music-components/ChipsRow.svelte";
  import {
    studyYoutubeSearch,
    studyYoutubeTrending,
    type YoutubeSearchItem,
    type YoutubeSearchVideoItem,
    type YoutubeSearchResultGroup,
    type YoutubeSearchFilter,
  } from "$lib/study-bridge";
  import { playYoutubeVideoItem } from "$lib/study-music/youtube-play-helper";
  import { showToast } from "$lib/stores/toast-store.svelte";

  let q = $state("");
  let loading = $state(false);
  let inputRef = $state<HTMLInputElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  let ytGroups = $state<YoutubeSearchResultGroup[]>([]);
  let ytError = $state<string | null>(null);

  let trending = $state<YoutubeSearchItem[]>([]);
  let trendingLoading = $state(false);

  type SortKey = "relevance" | "recent" | "views";
  let activeFilter = $state<YoutubeSearchFilter>("all");
  let activeSort = $state<SortKey>("relevance");
  const HISTORY_KEY = "study.music.search.history.v1";
  const HISTORY_MAX = 10;
  let history = $state<string[]>([]);

  function loadHistory() {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          history = parsed.filter((s) => typeof s === "string").slice(0, HISTORY_MAX);
        }
      }
    } catch {
      history = [];
    }
  }

  function saveHistory() {
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
    } catch {}
  }

  function pushHistory(query: string) {
    const trimmed = query.trim();
    if (!trimmed) return;
    history = [trimmed, ...history.filter((h) => h !== trimmed)].slice(0, HISTORY_MAX);
    saveHistory();
  }

  function removeFromHistory(query: string) {
    history = history.filter((h) => h !== query);
    saveHistory();
  }

  function clearHistory() {
    history = [];
    saveHistory();
  }

  function applyHistory(query: string) {
    q = query;
    if (timer) clearTimeout(timer);
    void runSearch(q);
  }

  function applySort(items: YoutubeSearchItem[]): YoutubeSearchItem[] {
    if (activeSort === "relevance") return items;
    return [...items].sort((a, b) => {
      if (a.kind !== "video" || b.kind !== "video") return 0;
      if (activeSort === "views") {
        const av = parseViewCount(a.view_count_text);
        const bv = parseViewCount(b.view_count_text);
        return bv - av;
      }
      if (activeSort === "recent") {
        const ap = parsePublishedRank(a.published_time_text);
        const bp = parsePublishedRank(b.published_time_text);
        return ap - bp;
      }
      return 0;
    });
  }

  function parseViewCount(text: string | null | undefined): number {
    if (!text) return 0;
    const cleaned = text.replace(/[^\d.,KMBkmb]/g, "").trim();
    const m = cleaned.match(/^([\d.,]+)([KMBkmb])?/);
    if (!m) return 0;
    const n = parseFloat((m[1] ?? "0").replace(/,/g, ""));
    const suffix = (m[2] ?? "").toLowerCase();
    const mult = suffix === "k" ? 1e3 : suffix === "m" ? 1e6 : suffix === "b" ? 1e9 : 1;
    return n * mult;
  }

  function parsePublishedRank(text: string | null | undefined): number {
    if (!text) return Number.MAX_SAFE_INTEGER;
    const m = text.match(/(\d+)\s*(second|minute|hour|day|week|month|year)/i);
    if (!m) return Number.MAX_SAFE_INTEGER;
    const n = parseInt(m[1] ?? "0", 10);
    const unit = (m[2] ?? "").toLowerCase();
    const mult =
      unit === "second" ? 1 :
      unit === "minute" ? 60 :
      unit === "hour" ? 3600 :
      unit === "day" ? 86400 :
      unit === "week" ? 604800 :
      unit === "month" ? 2629800 :
      31557600;
    return n * mult;
  }

  async function loadTrending() {
    trendingLoading = true;
    try {
      const res = await studyYoutubeTrending({ category: "now" });
      const flat: YoutubeSearchItem[] = [];
      for (const it of res.items ?? []) {
        if (it.kind === "shelf") {
          for (const s of it.items) flat.push(s);
        } else {
          flat.push(it);
        }
      }
      trending = flat.slice(0, 24);
    } catch (e) {
      console.warn("[trending] failed", e);
    } finally {
      trendingLoading = false;
    }
  }

  function flattenVideoItems(groups: YoutubeSearchResultGroup[]): YoutubeSearchVideoItem[] {
    const out: YoutubeSearchVideoItem[] = [];
    for (const g of groups) {
      for (const item of g.items) {
        if (item.kind === "video") out.push(item);
        else if (item.kind === "shelf") {
          for (const s of item.items) if (s.kind === "video") out.push(s);
        }
      }
    }
    return out;
  }

  function openYoutubePlaylist(playlistId: string) {
    const route = playlistId.startsWith("OLAK") ? "album" : "playlist";
    void goto(`/study/music/youtube/${route}/${encodeURIComponent(playlistId)}`);
  }

  async function playFromYouTube(videoId: string) {
    const queue = flattenVideoItems(ytGroups);
    const item = queue.find((v) => v.video_id === videoId);
    if (!item) return;
    try {
      await playYoutubeVideoItem(item, queue);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function runSearch(query: string) {
    if (!query.trim()) {
      ytGroups = [];
      ytError = null;
      loading = false;
      return;
    }
    loading = true;
    ytError = null;
    try {
      const ytRes = await studyYoutubeSearch({ query, filter: activeFilter });
      const raw = ytRes.groups ?? [];
      if (activeSort !== "relevance") {
        ytGroups = raw.map((g) => ({ ...g, items: applySort(g.items) }));
      } else {
        ytGroups = raw;
      }
    } catch (e) {
      ytError = e instanceof Error ? e.message : String(e);
      ytGroups = [];
    } finally {
      loading = false;
    }
  }

  function onInput(e: Event) {
    q = (e.target as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => runSearch(q), 200);
  }

  function onSearchSubmit(e: Event) {
    e.preventDefault();
    const trimmed = q.trim();
    if (!trimmed) return;
    if (timer) clearTimeout(timer);
    pushHistory(trimmed);
    void runSearch(trimmed);
  }

  function setFilter(id: string) {
    activeFilter = id as YoutubeSearchFilter;
    if (activeFilter === "all" && activeSort !== "relevance") {
      activeSort = "relevance";
    }
    if (q.trim()) void runSearch(q);
  }

  function setSort(key: SortKey) {
    activeSort = key;
    if (q.trim()) void runSearch(q);
  }

  onMount(() => {
    inputRef?.focus();
    void loadTrending();
    loadHistory();
    return () => {
      if (timer) clearTimeout(timer);
    };
  });
</script>

<section class="search-page">
  <header class="head">
    <form class="search-box" onsubmit={onSearchSubmit}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="7"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input
        bind:this={inputRef}
        type="search"
        value={q}
        oninput={onInput}
        placeholder={$t("study.music.youtube.search_placeholder")}
        autocomplete="off"
      />
      {#if q}
        <button
          type="button"
          class="clear"
          onclick={() => { q = ""; ytGroups = []; }}
          aria-label={$t("study.common.clear") as string}
        >×</button>
      {/if}
    </form>

    <div class="filters">
      <ChipsRow
        ariaLabel={$t("study.music.youtube.search_filter_aria") as string}
        chips={[
          { id: "all", label: $t("study.music.youtube.search_filter_all") as string },
          { id: "music", label: $t("study.music.youtube.search_filter_music") as string },
          { id: "videos", label: $t("study.music.youtube.search_filter_videos") as string },
          { id: "channels", label: $t("study.music.youtube.search_filter_channels") as string },
          { id: "playlists", label: $t("study.music.youtube.search_filter_playlists") as string },
        ]}
        activeId={activeFilter}
        onSelect={setFilter}
      />
      {#if activeFilter !== "all"}
        <label class="sort-label">
          <span class="sr-only">{$t("study.music.youtube.search_sort_aria")}</span>
          <select class="sort-select" value={activeSort} onchange={(e) => setSort((e.currentTarget as HTMLSelectElement).value as SortKey)}>
            <option value="relevance">{$t("study.music.youtube.search_sort_relevance")}</option>
            <option value="recent">{$t("study.music.youtube.search_sort_recent")}</option>
            <option value="views">{$t("study.music.youtube.search_sort_views")}</option>
          </select>
        </label>
      {/if}
    </div>

    {#if history.length > 0 && !q.trim()}
      <div class="history">
        <div class="history-head">
          <span class="history-label">{$t("study.music.youtube.search_history_label")}</span>
          <button type="button" class="history-clear" onclick={clearHistory}>
            {$t("study.music.youtube.search_history_clear")}
          </button>
        </div>
        <ul class="history-list">
          {#each history as item (item)}
            <li class="history-pill">
              <button type="button" class="history-text" onclick={() => applyHistory(item)}>
                {item}
              </button>
              <button
                type="button"
                class="history-x"
                aria-label={$t("study.music.youtube.search_history_remove_aria", { query: item }) as string}
                onclick={() => removeFromHistory(item)}
              >×</button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </header>

  {#if !q.trim()}
    {#if trendingLoading}
      <section class="block">
        <h3 class="block-title">{$t("study.music.youtube.shelf_trending")}</h3>
        <div class="grid-skel">
          <YoutubeSkeleton kind="card" count={3} />
        </div>
      </section>
    {:else if trending.length > 0}
      <YoutubeShelf
        title={$t("study.music.youtube.shelf_trending")}
        items={trending}
        onPlay={playFromYouTube}
        onPlaylistOpen={openYoutubePlaylist}
        minCard={200}
      />
    {:else}
      <div class="empty">
        <p>{$t("study.music.youtube.search_hint")}</p>
      </div>
    {/if}
  {:else if loading}
    <section class="block">
      <div class="grid-skel">
        <YoutubeSkeleton kind="card" count={4} />
      </div>
    </section>
  {:else if ytGroups.length === 0 && !ytError}
    <div class="empty">
      <p>{$t("study.music.empty_results")}</p>
    </div>
  {:else}
    {#if ytError}
      <YoutubeError
        error={ytError}
        onRetry={() => { ytError = null; void runSearch(q); }}
      />
    {:else if ytGroups.length > 0}
      {#each ytGroups as group, gi (gi)}
        <YoutubeShelf
          title={group.title ?? ($t("study.music.youtube.shelf_group") as string)}
          items={group.items}
          onPlay={playFromYouTube}
          onPlaylistOpen={openYoutubePlaylist}
          minCard={210}
        />
      {/each}
    {/if}
  {/if}
</section>

<style>
  .search-page { display: flex; flex-direction: column; gap: 20px; }
  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 999px;
    width: min(100%, 480px);
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box svg { color: var(--tertiary); flex-shrink: 0; }
  .search-box input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--secondary);
    font-family: inherit;
    font-size: 14px;
    outline: none;
    min-width: 0;
  }
  .clear {
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--tertiary);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }
  .clear:hover { color: var(--secondary); }
  .empty { padding: 48px 24px; text-align: center; color: var(--tertiary); font-size: 14px; }
  .block { display: flex; flex-direction: column; gap: 10px; }
  .block-title { margin: 0; font-size: 16px; font-weight: 700; color: var(--secondary); }
  .grid-skel {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 18px 14px;
  }
  .head { display: flex; flex-direction: column; gap: 12px; }
  .filters {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .sort-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tertiary);
    font-size: 12px;
  }
  .sort-select {
    background: color-mix(in oklab, var(--button) 60%, transparent);
    color: var(--secondary);
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 999px;
    padding: 6px 10px;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .sort-select:hover { border-color: var(--accent); }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .history {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .history-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .history-label {
    color: var(--tertiary);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .history-clear {
    background: transparent;
    border: 0;
    color: var(--tertiary);
    font: inherit;
    font-size: 11px;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
  }
  .history-clear:hover { color: var(--secondary); }
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .history-pill {
    display: inline-flex;
    align-items: center;
    background: color-mix(in oklab, var(--button) 50%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    border-radius: 999px;
    overflow: hidden;
  }
  .history-pill:hover { border-color: var(--accent); }
  .history-text {
    background: transparent;
    border: 0;
    padding: 6px 4px 6px 12px;
    color: var(--secondary);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .history-x {
    background: transparent;
    border: 0;
    padding: 4px 10px 4px 4px;
    color: var(--tertiary);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
    line-height: 1;
  }
  .history-x:hover { color: var(--secondary); }
</style>
