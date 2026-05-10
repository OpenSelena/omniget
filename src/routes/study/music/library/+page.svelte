<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import TrackRow from "$lib/study-music-components/TrackRow.svelte";
  import type { MusicTrack } from "$lib/study-music/player-store.svelte";
  import {
    spotifyStore,
    spotifyTrackToMusicTrack,
    type SpotifyTrack,
  } from "$lib/study-music/spotify-store.svelte";
  import {
    soundcloudStore,
    scTrackToMusicTrack,
    type ScTrack,
  } from "$lib/study-music/soundcloud-store.svelte";

  let tracks = $state<MusicTrack[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let q = $state("");
  let timer: ReturnType<typeof setTimeout> | null = null;

  function buildSpotifyPool(): SpotifyTrack[] {
    const pool = new Map<string, SpotifyTrack>();
    for (const t of spotifyStore.savedTracks) pool.set(t.id, t);
    for (const t of spotifyStore.recentlyPlayed) if (!pool.has(t.id)) pool.set(t.id, t);
    for (const t of spotifyStore.topTracks) if (!pool.has(t.id)) pool.set(t.id, t);
    return [...pool.values()].filter((sp) => !spotifyStore.localMatches.has(sp.id));
  }

  function spotifyToMusic(pool: SpotifyTrack[]): MusicTrack[] {
    return pool.map(spotifyTrackToMusicTrack);
  }

  type UnifiedTrackRow = {
    source: "local" | "youtube";
    id: number;
    path: string;
    title: string | null;
    artist: string | null;
    album: string | null;
    duration_ms: number | null;
    cover_path: string | null;
    youtube_thumbnail?: string | null;
    video_id?: string;
    play_count: number;
    favorite: boolean;
  };

  function unifiedToTrack(u: UnifiedTrackRow): MusicTrack {
    return {
      id: u.id,
      path: u.path,
      title: u.title,
      artist: u.artist,
      album: u.album,
      duration_ms: u.duration_ms,
      cover_path: u.cover_path ?? u.youtube_thumbnail ?? null,
      play_count: u.play_count,
      favorite: u.favorite,
      source: u.source,
      youtube_video_id: u.source === "youtube" ? u.video_id : undefined,
      youtube_thumbnail: u.youtube_thumbnail ?? undefined,
      youtube_url: u.source === "youtube" ? u.path : undefined,
    };
  }

  async function load() {
    loading = true;
    try {
      if (
        spotifyStore.status.logged_in &&
        spotifyStore.savedTracks.length === 0 &&
        spotifyStore.recentlyPlayed.length === 0
      ) {
        try {
          await spotifyStore.loadAll();
        } catch {
          /* ignore */
        }
      }
      if (
        soundcloudStore.isLoggedIn &&
        soundcloudStore.likedTracks.length === 0
      ) {
        try {
          await soundcloudStore.loadAll();
        } catch {
          /* ignore */
        }
      }

      const trimmed = q.trim();
      const sortKey = trimmed ? "title" : "added";
      const res = await pluginInvoke<{ tracks: UnifiedTrackRow[]; total: number }>(
        "study",
        "study:music:unified:tracks",
        { limit: 500, sort: sortKey },
      );
      let unified = (res.tracks ?? []).map(unifiedToTrack);

      if (trimmed) {
        const term = trimmed.toLowerCase();
        unified = unified.filter(
          (t) =>
            (t.title ?? "").toLowerCase().includes(term) ||
            (t.artist ?? "").toLowerCase().includes(term) ||
            (t.album ?? "").toLowerCase().includes(term),
        );
      }

      const spotifyPool = buildSpotifyPool();
      const matchesText = (s: string | null | undefined) =>
        !!s && s.toLowerCase().includes(trimmed.toLowerCase());
      const filteredSpotify =
        trimmed.length > 0
          ? spotifyPool.filter(
              (t) =>
                matchesText(t.name) ||
                t.artists.some((a) => matchesText(a.name)) ||
                matchesText(t.album.name),
            )
          : spotifyPool;
      const spotifyTracks = spotifyToMusic(filteredSpotify);

      const scPool: ScTrack[] = soundcloudStore.likedTracks;
      const scFiltered =
        trimmed.length > 0
          ? scPool.filter(
              (t) =>
                matchesText(t.title) ||
                matchesText(t.user.username),
            )
          : scPool;
      const scTracks = scFiltered.map(scTrackToMusicTrack);

      tracks = [...unified, ...spotifyTracks, ...scTracks];
      total = tracks.length;
    } finally {
      loading = false;
    }
  }

  function onInput(e: Event) {
    q = (e.target as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => load(), 200);
  }

  onMount(() => {
    void load();
    return () => {
      if (timer) clearTimeout(timer);
    };
  });
</script>

<section class="library-page">
  <header class="page-head">
    <h1>{$t("study.music.library_title")}</h1>
  </header>

  <div class="search-box">
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="7"/>
      <line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      type="search"
      value={q}
      oninput={onInput}
      placeholder={$t("study.music.search_placeholder")}
      autocomplete="off"
    />
  </div>

  {#if loading && tracks.length === 0}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if tracks.length === 0}
    <div class="empty">
      <p>{$t("study.music.empty_results")}</p>
    </div>
  {:else}
    <p class="result-count">{tracks.length} de {total} faixa(s)</p>
    <ul class="track-list">
      {#each tracks as track (track.id)}
        <TrackRow {track} queue={tracks} showCover showAlbum />
      {/each}
    </ul>
  {/if}
</section>

<style>
  .library-page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    color: rgba(255, 255, 255, 0.95);
  }
  .page-head h1 {
    margin: 0;
    font-size: clamp(28px, 3.5vw, 40px);
    font-weight: 900;
    letter-spacing: -0.02em;
    color: rgba(255, 255, 255, 0.95);
  }
  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 999px;
    width: min(100%, 480px);
    transition: border-color 200ms ease;
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box svg { color: rgba(255, 255, 255, 0.5); flex-shrink: 0; }
  .search-box input {
    flex: 1;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    font-size: 13px;
    outline: none;
    min-width: 0;
  }
  .result-count {
    margin: 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
  }
  .track-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .empty {
    padding: 64px 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 13px; }
</style>
