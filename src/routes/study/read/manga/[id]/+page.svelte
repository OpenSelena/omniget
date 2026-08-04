<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";

  type Series = {
    id: number;
    title: string;
    author: string | null;
    description: string | null;
    status: string | null;
    reading_direction: string;
    cover_path: string | null;
    entry_count: number;
    read_count: number;
    progress_pct: number;
    last_entry_id: number | null;
  };

  type Entry = {
    id: number;
    number: number | null;
    label: string | null;
    volume: number | null;
    scanlator: string | null;
    archive_format: string;
    archive_path: string | null;
    page_count: number | null;
    is_read: boolean;
    last_page_read: number;
    last_read_at: number | null;
    sort_index: number;
  };

  const seriesId = $derived(Number($page.params.id));

  let series = $state<Series | null>(null);
  let entries = $state<Entry[]>([]);
  let loading = $state(true);
  let error = $state("");

  async function load() {
    loading = true;
    try {
      series = await pluginInvoke<Series>("study", "study:read:series:get", {
        seriesId,
      });
      entries = await pluginInvoke<Entry[]>("study", "study:read:series:entries", {
        seriesId,
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function openEntry(e: Entry) {
    goto(`/study/read/entry/${e.id}`);
  }

  async function toggleRead(e: Entry, ev: MouseEvent) {
    ev.stopPropagation();
    ev.preventDefault();
    try {
      await pluginInvoke("study", "study:read:series:mark_entry", {
        entryId: e.id,
        isRead: !e.is_read,
      });
      entries = entries.map((x) => (x.id === e.id ? { ...x, is_read: !x.is_read } : x));
      if (series) {
        const readCount = entries.filter((x) => x.is_read).length;
        series = {
          ...series,
          read_count: readCount,
          progress_pct: series.entry_count > 0 ? readCount / series.entry_count : 0,
        };
      }
    } catch (err) {
      console.error(err);
    }
  }

  function continueReading() {
    if (!series) return;
    if (series.last_entry_id) {
      goto(`/study/read/entry/${series.last_entry_id}`);
      return;
    }
    const firstUnread = entries.find((e) => !e.is_read);
    if (firstUnread) {
      goto(`/study/read/entry/${firstUnread.id}`);
      return;
    }
    if (entries.length > 0) {
      goto(`/study/read/entry/${entries[0].id}`);
    }
  }

  function entryLabel(e: Entry): string {
    if (e.number != null) {
      const label = e.label ?? "";
      return label ? `${e.number} · ${label}` : `${$t("study.read.chapter")} ${e.number}`;
    }
    return e.label ?? `#${e.sort_index + 1}`;
  }

  onMount(load);
</script>

<section class="detail">
  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if series}
    <header class="head">
      <button type="button" class="back-btn" onclick={() => goto("/study/read/manga")}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
          <path d="M15 18l-6-6 6-6"></path>
        </svg>
        <span>{$t("study.read.back_to_manga")}</span>
      </button>
    </header>

    <div class="hero">
      <div class="cover">
        {#if series.cover_path}
          <img src={convertFileSrc(series.cover_path)} alt={series.title} />
        {:else}
          <div class="cover-placeholder">
            <svg viewBox="0 0 64 64" width="44" height="56" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="8" y="6" width="48" height="52" rx="2"></rect>
            </svg>
          </div>
        {/if}
      </div>
      <div class="info">
        <h1>{series.title}</h1>
        {#if series.author}
          <p class="author">{series.author}</p>
        {/if}
        {#if series.description}
          <p class="description">{series.description}</p>
        {/if}
        <div class="stats">
          <span class="mono">{$t("study.read.series_count_chapters", { current: series.read_count, total: series.entry_count })}</span>
          <span>·</span>
          <span class="mono">{Math.round(series.progress_pct * 100)}%</span>
          {#if series.reading_direction === "rtl"}
            <span>·</span>
            <span class="tag">RTL</span>
          {/if}
          {#if series.reading_direction === "vertical"}
            <span>·</span>
            <span class="tag">Webtoon</span>
          {/if}
        </div>
        <button type="button" class="cta" onclick={continueReading} disabled={entries.length === 0}>
          {series.read_count === 0
            ? $t("study.read.series_start_reading")
            : $t("study.read.series_continue_reading")}
        </button>
      </div>
    </div>

    <div class="entries">
      <h2>{$t("study.read.series_chapters")}</h2>
      <ul class="entry-list">
        {#each entries as e (e.id)}
          <li>
            <button type="button" class="entry-row" class:read={e.is_read} onclick={() => openEntry(e)}>
              <span
                class="read-mark"
                class:active={e.is_read}
                role="button"
                tabindex="0"
                aria-label={e.is_read ? $t("study.read.mark_unread") : $t("study.read.mark_read")}
                onclick={(ev) => toggleRead(e, ev as MouseEvent)}
                onkeydown={(ev) => {
                  if (ev.key === "Enter" || ev.key === " ") {
                    ev.preventDefault();
                    ev.stopPropagation();
                    toggleRead(e, ev as unknown as MouseEvent);
                  }
                }}
              >
                {#if e.is_read}
                  <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M20 6L9 17l-5-5"></path>
                  </svg>
                {/if}
              </span>
              <span class="entry-label">{entryLabel(e)}</span>
              {#if e.page_count != null}
                <span class="entry-pages mono">{e.page_count}p</span>
              {/if}
              {#if e.last_page_read > 0 && !e.is_read}
                <span class="entry-progress mono">
                  {e.last_page_read}/{e.page_count ?? "?"}
                </span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 900px;
    margin-inline: auto;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 1rem;
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
  }
  .back-btn:hover {
    color: var(--secondary);
    background: var(--sidebar-highlight);
  }
  .hero {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: calc(var(--padding) * 1.5);
    align-items: start;
  }
  .cover {
    aspect-ratio: 2 / 3;
    background: var(--input-bg);
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-placeholder {
    color: var(--tertiary);
  }
  .info h1 {
    margin: 0 0 8px;
    font-size: 24px;
    font-weight: 500;
    color: var(--secondary);
    letter-spacing: -0.5px;
  }
  .author {
    margin: 0 0 12px;
    font-size: 14px;
    color: var(--tertiary);
  }
  .description {
    margin: 0 0 16px;
    font-size: 14px;
    color: var(--secondary);
    line-height: 1.5;
    max-width: 60ch;
  }
  .stats {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 16px;
    font-size: 12px;
    color: var(--tertiary);
  }
  .tag {
    padding: 1px 6px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
  .cta {
    padding: 10px 24px;
    background: var(--cta, var(--accent));
    color: var(--on-cta, white);
    border: 1px solid transparent;
    border-radius: var(--border-radius);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
  }
  .cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .entries h2 {
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
    margin: 0 0 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .entry-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .entry-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    color: var(--secondary);
    border: 1px solid transparent;
    border-radius: var(--border-radius);
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    transition: background 150ms ease;
  }
  .entry-row:hover {
    background: var(--sidebar-highlight);
    border-color: var(--input-border);
  }
  .entry-row.read {
    color: var(--tertiary);
  }
  .read-mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: 1px solid var(--input-border);
    border-radius: 50%;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 120ms ease;
  }
  .read-mark.active {
    background: var(--accent);
    color: var(--on-accent, white);
    border-color: var(--accent);
  }
  .entry-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-pages,
  .entry-progress {
    font-size: 11px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
  }
  .entry-progress {
    color: var(--accent);
  }
  .mono {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--tertiary);
  }
  .error {
    color: var(--error);
  }
</style>
