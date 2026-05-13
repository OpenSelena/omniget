<script lang="ts">
  import { onMount } from "svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    studyMusicHistoryList,
    studyMusicHistoryClear,
    studyMusicHistoryRemove,
    type MusicHistoryEntry,
  } from "$lib/study-bridge";
  import { playYoutubeVideoId } from "$lib/study-music/youtube-play-helper";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";
  import YoutubeError from "$lib/study-music-youtube-components/YoutubeError.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import { t } from "$lib/i18n";
  import { goto } from "$app/navigation";

  let entries = $state<MusicHistoryEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      const res = await studyMusicHistoryList({
        limit: 300,
        source: "youtube",
      });
      entries = res.entries ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function fmtAgo(tsMs: number): string {
    const seconds = Math.floor((Date.now() - tsMs) / 1000);
    if (seconds < 60) return $t("study.music.history_now") as string;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes} min`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} h`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days} d`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months} m`;
    const years = Math.floor(months / 12);
    return `${years} a`;
  }

  async function play(entry: MusicHistoryEntry) {
    try {
      if (entry.external_id) {
        await playYoutubeVideoId(
          entry.external_id,
          entry.title,
          entry.artist,
          entry.cover_url,
        );
        return;
      }
      showToast("info", $t("study.music.history_unplayable") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function remove(id: number) {
    try {
      await studyMusicHistoryRemove({ id });
      entries = entries.filter((e) => e.id !== id);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function clearAll() {
    if (!window.confirm($t("study.music.history_clear_confirm_yt") as string)) return;
    try {
      const res = await studyMusicHistoryClear({ source: "youtube" });
      showToast("success", `${res.removed} ${$t("study.music.history_entries_removed") as string}`);
      entries = [];
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="history-page">
  <header class="head">
    <h1>{$t("study.music.history_title_yt")}</h1>
    {#if entries.length > 0}
      <button class="clear-btn" onclick={clearAll} type="button">{$t("study.music.history_clear")}</button>
    {/if}
  </header>

  {#if loading}
    <div class="skel-list">
      <YoutubeSkeleton kind="row" count={6} />
    </div>
  {:else if error}
    <YoutubeError error={error} onRetry={() => { error = null; void load(); }} />
  {:else if entries.length === 0}
    <EmptyPlaceholder
      title={$t("study.music.history_empty_title_yt") as string}
      ctaLabel={$t("study.music.history_empty_cta_yt") as string}
      onCta={() => goto("/study/music/youtube/search")}
    />
  {:else}
    <ul class="list">
      {#each entries as entry (entry.id)}
        <li class="row">
          <button class="thumb" onclick={() => play(entry)} type="button" aria-label={$t("study.music.play") as string}>
            {#if entry.cover_url}
              <img src={entry.cover_url} alt="" loading="lazy" />
            {/if}
          </button>
          <div class="info">
            <button class="title-btn" onclick={() => play(entry)} type="button">
              {entry.title}
            </button>
            <p class="meta">
              {#if entry.artist}
                <span>{entry.artist}</span>
                <span> · </span>
              {/if}
              <span>{fmtAgo(entry.played_at)}</span>
            </p>
          </div>
          <button class="remove" onclick={() => remove(entry.id)} aria-label={$t("study.music.history_remove_aria") as string} type="button">×</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .history-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px 20px 32px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .head h1 { margin: 0; font-size: 22px; font-weight: 700; color: var(--primary); }
  .clear-btn {
    padding: 6px 14px;
    background: transparent;
    color: var(--tertiary);
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 999px;
    font-size: 12px;
    cursor: pointer;
  }
  .clear-btn:hover { color: #d33; border-color: #d33; }
  .list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .row {
    display: grid;
    grid-template-columns: 120px 1fr auto;
    align-items: center;
    gap: 14px;
    padding: 8px;
    border-radius: 10px;
  }
  .row:hover { background: color-mix(in oklab, var(--button) 30%, transparent); }
  .thumb {
    position: relative;
    width: 120px;
    aspect-ratio: 16 / 9;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    border: 0;
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    padding: 0;
  }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .info { min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .title-btn {
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--primary);
    font-size: 14px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .title-btn:hover { color: var(--accent); }
  .meta { margin: 0; font-size: 12px; color: var(--tertiary); }
  .remove {
    width: 30px;
    height: 30px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--tertiary);
    font-size: 18px;
    cursor: pointer;
  }
  .remove:hover { color: #d33; background: color-mix(in oklab, var(--button) 40%, transparent); }
  .skel-list { display: flex; flex-direction: column; gap: 8px; }
</style>
