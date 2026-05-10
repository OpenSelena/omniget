<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { fmtDuration } from "$lib/study-music/format";
  import {
    musicPlayer,
    type MusicTrack,
  } from "$lib/study-music/player-store.svelte";
  import { t } from "$lib/i18n";

  type YtTrack = {
    video_id: string;
    url: string;
    title: string | null;
    artist: string | null;
    uploader: string | null;
    duration_ms: number | null;
    thumbnail: string | null;
    last_played_at: number | null;
    play_count: number;
    favorite: boolean;
  };

  type YtPlaylist = {
    id: string;
    url: string;
    title: string | null;
    uploader: string | null;
    thumbnail: string | null;
    track_count: number;
    kind: string;
  };

  let playlist = $state<YtPlaylist | null>(null);
  let tracks = $state<YtTrack[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const playlistId = $derived(decodeURIComponent($page.params.id ?? ""));

  function ytToMusicTrack(t: YtTrack): MusicTrack {
    const numericId =
      Math.abs(
        t.video_id
          .split("")
          .reduce((acc, c) => (acc * 31 + c.charCodeAt(0)) | 0, 0),
      ) || 1;
    return {
      id: numericId,
      path: t.url,
      title: t.title,
      artist: t.artist ?? t.uploader,
      album: playlist?.title ?? null,
      duration_ms: t.duration_ms,
      cover_path: null,
      favorite: t.favorite,
      play_count: t.play_count,
      last_played_at: t.last_played_at,
      source: "youtube",
      youtube_url: t.url,
      youtube_video_id: t.video_id,
      youtube_thumbnail: t.thumbnail ?? undefined,
    };
  }

  async function load() {
    loading = true;
    error = null;
    try {
      const res = await pluginInvoke<{
        playlist: YtPlaylist;
        tracks: YtTrack[];
      }>("study", "study:music:youtube:playlist_get", { id: playlistId });
      playlist = res.playlist;
      tracks = res.tracks ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function playAll() {
    if (tracks.length === 0) return;
    const queue = tracks.map(ytToMusicTrack);
    void musicPlayer.play(queue[0], queue);
  }

  function playOne(t: YtTrack) {
    const queue = tracks.map(ytToMusicTrack);
    const target = queue.find((q) => q.youtube_video_id === t.video_id);
    if (target) void musicPlayer.play(target, queue);
  }

  async function toggleFavorite(t: YtTrack) {
    try {
      const res = await pluginInvoke<{ favorite: boolean }>(
        "study",
        "study:music:youtube:track_toggle_favorite",
        { video_id: t.video_id },
      );
      tracks = tracks.map((x) =>
        x.video_id === t.video_id ? { ...x, favorite: res.favorite } : x,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  $effect(() => {
    if (playlistId) void load();
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
  {:else if playlist}
    <header class="hero">
      <div class="hero-cover">
        {#if playlist.thumbnail}
          <img src={playlist.thumbnail} alt="" />
        {:else}
          <div class="fallback"></div>
        {/if}
      </div>
      <div class="hero-meta">
        <span class="eyebrow">{$t("study.music.yt_eyebrow")}</span>
        <h1>{playlist.title ?? playlist.id}</h1>
        {#if playlist.uploader}
          <p class="hero-sub">{playlist.uploader}</p>
        {/if}
        <p class="hero-stats">{tracks.length} faixas</p>
        <div class="hero-actions">
          <button type="button" class="cta" onclick={playAll}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
            {$t("study.music.play")}
          </button>
        </div>
      </div>
    </header>

    <ul class="track-list">
      {#each tracks as track, i (track.video_id)}
        <li class="row">
          <button type="button" class="row-btn" onclick={() => playOne(track)}>
            <span class="num">{i + 1}</span>
            <span class="thumb-wrap">
              {#if track.thumbnail}
                <img src={track.thumbnail} alt="" loading="lazy" />
              {/if}
            </span>
            <span class="info">
              <span class="title">{track.title ?? track.video_id}</span>
              {#if track.artist || track.uploader}
                <span class="artist">{track.artist ?? track.uploader}</span>
              {/if}
            </span>
            <span class="dur">{fmtDuration(track.duration_ms)}</span>
          </button>
          <button
            type="button"
            class="fav"
            class:on={track.favorite}
            onclick={(e) => { e.stopPropagation(); toggleFavorite(track); }}
            aria-label={$t("study.music.favorite") as string}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill={track.favorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
            </svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding-bottom: 40px;
  }
  .hero {
    display: flex;
    gap: 24px;
    align-items: flex-end;
  }
  .hero-cover {
    width: 200px;
    height: 200px;
    flex-shrink: 0;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.5);
    background: rgba(40, 40, 40, 0.6);
  }
  .hero-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .hero-cover .fallback {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, #ff0000, #8b0000);
  }
  .hero-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .eyebrow {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .hero-meta h1 {
    margin: 0;
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .hero-sub {
    margin: 0;
    color: rgba(255, 255, 255, 0.65);
    font-size: 14px;
  }
  .hero-stats {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
  }
  .hero-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .cta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
  .track-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .row:hover { background: rgba(255, 255, 255, 0.04); }
  .row-btn {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 4px;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .num {
    flex-shrink: 0;
    width: 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .thumb-wrap {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    background: rgba(40, 40, 40, 0.6);
    border-radius: 4px;
    overflow: hidden;
  }
  .thumb-wrap img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 2px;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dur {
    flex-shrink: 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
    min-width: 48px;
    text-align: right;
  }
  .fav {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .fav:hover { color: var(--accent); background: rgba(255, 255, 255, 0.08); }
  .fav.on { color: var(--accent); }
  .muted { color: rgba(255, 255, 255, 0.5); }
  .error { color: rgb(248, 113, 113); }
</style>
