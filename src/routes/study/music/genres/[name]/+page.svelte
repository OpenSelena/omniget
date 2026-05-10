<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { fmtDurationLong } from "$lib/study-music/format";
  import {
    musicPlayer,
    type MusicTrack,
  } from "$lib/study-music/player-store.svelte";
  import TrackRow from "$lib/study-music-components/TrackRow.svelte";

  type GenreSummary = {
    name: string;
    track_count: number;
    album_count: number;
    artist_count: number;
    total_duration_ms: number;
  };

  let genre = $state<GenreSummary | null>(null);
  let tracks = $state<MusicTrack[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const name = $derived(decodeURIComponent($page.params.name ?? ""));

  async function load() {
    loading = true;
    error = null;
    try {
      const res = await pluginInvoke<{
        genre: GenreSummary;
        tracks: MusicTrack[];
      }>("study", "study:music:genres:get", { name, limit: 500 });
      genre = res.genre;
      tracks = res.tracks ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function playAll() {
    if (tracks.length === 0) return;
    void musicPlayer.play(tracks[0], tracks);
  }

  function playShuffled() {
    if (tracks.length === 0) return;
    musicPlayer.shuffle = true;
    const idx = Math.floor(Math.random() * tracks.length);
    void musicPlayer.play(tracks[idx], tracks);
  }

  $effect(() => {
    if (name) void load();
  });

  onMount(() => {
    void load();
  });
</script>

<section class="page">
  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if genre}
    <header class="hero">
      <span class="eyebrow">{$t("study.music.eyebrow_genre")}</span>
      <h1>{genre.name}</h1>
      <p class="meta">
        {$t("study.music.tracks_count", { count: genre.track_count })} ·
        {$t("study.music.albums_count", { count: genre.album_count })} ·
        {$t("study.music.artists_count", { count: genre.artist_count })} ·
        {fmtDurationLong(genre.total_duration_ms)}
      </p>
      <div class="actions">
        <button type="button" class="play-btn" onclick={playAll}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
          {$t("study.music.play")}
        </button>
        <button type="button" class="ghost-btn" onclick={playShuffled}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M16 3h5v5 M4 20l17-17 M21 16v5h-5 M4 4l5 5 M15 15l6 6"/></svg>
          {$t("study.music.shuffle")}
        </button>
      </div>
    </header>

    <ul class="track-list">
      {#each tracks as tr, idx (tr.id)}
        <TrackRow track={tr} queue={tracks} index={idx} showNumber showAlbum />
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-bottom: 40px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 0 4px;
  }
  .eyebrow {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .hero h1 {
    margin: 0;
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.02em;
    text-transform: capitalize;
  }
  .hero .meta {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .play-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 18px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .ghost-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .track-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .muted { color: rgba(255, 255, 255, 0.5); }
  .error { color: rgb(248, 113, 113); }
</style>
