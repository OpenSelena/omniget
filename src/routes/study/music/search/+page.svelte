<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import TrackRow from "$lib/study-music-components/TrackRow.svelte";
  import AlbumCard from "$lib/study-music-components/AlbumCard.svelte";
  import type { MusicTrack } from "$lib/study-music/player-store.svelte";

  type Album = {
    name: string;
    artist: string | null;
    cover_path: string | null;
    track_count: number;
    year: number | null;
  };

  let q = $state("");
  let tracks = $state<MusicTrack[]>([]);
  let albums = $state<Album[]>([]);
  let artists = $state<string[]>([]);
  let loading = $state(false);
  let inputRef = $state<HTMLInputElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  async function runSearch(query: string) {
    if (!query.trim()) {
      tracks = [];
      albums = [];
      artists = [];
      loading = false;
      return;
    }
    loading = true;
    try {
      const tracksRes = await pluginInvoke<{ tracks: MusicTrack[] }>(
        "study",
        "study:music:tracks:search",
        { q: query, limit: 60 },
      );
      tracks = tracksRes.tracks ?? [];

      const albumNames = new Set<string>();
      const artistNames = new Set<string>();
      const albumByKey = new Map<string, Album>();

      for (const tr of tracks) {
        if (tr.album && !albumNames.has(tr.album)) {
          albumNames.add(tr.album);
          albumByKey.set(tr.album, {
            name: tr.album,
            artist: tr.album_artist ?? tr.artist ?? null,
            cover_path: tr.cover_path,
            track_count: 0,
            year: tr.year ?? null,
          });
        }
        if (tr.artist) artistNames.add(tr.artist);
        if (tr.album_artist) artistNames.add(tr.album_artist);
      }
      for (const tr of tracks) {
        if (tr.album) {
          const a = albumByKey.get(tr.album);
          if (a) a.track_count += 1;
        }
      }
      albums = [...albumByKey.values()].slice(0, 12);
      artists = [...artistNames].slice(0, 20);
    } finally {
      loading = false;
    }
  }

  function onInput(e: Event) {
    q = (e.target as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => runSearch(q), 200);
  }

  onMount(() => {
    inputRef?.focus();
    return () => {
      if (timer) clearTimeout(timer);
    };
  });
</script>

<section class="search-page">
  <header class="head">
    <div class="search-box">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="7"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input
        bind:this={inputRef}
        type="search"
        value={q}
        oninput={onInput}
        placeholder={$t("study.music.search_placeholder")}
        autocomplete="off"
      />
      {#if q}
        <button
          type="button"
          class="clear"
          onclick={() => { q = ""; tracks = []; albums = []; artists = []; }}
          aria-label={$t("study.common.clear") as string}
        >×</button>
      {/if}
    </div>
  </header>

  {#if !q.trim()}
    <div class="empty">
      <p>{$t("study.music.search_hint")}</p>
    </div>
  {:else if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if tracks.length === 0 && albums.length === 0 && artists.length === 0}
    <div class="empty">
      <p>{$t("study.music.empty_results")}</p>
    </div>
  {:else}
    {#if tracks.length > 0}
      <section class="block">
        <h2>{$t("study.music.tab_tracks")}</h2>
        <ul class="track-list">
          {#each tracks.slice(0, 10) as track (track.id)}
            <TrackRow {track} queue={tracks} showCover showAlbum />
          {/each}
        </ul>
      </section>
    {/if}

    {#if albums.length > 0}
      <section class="block">
        <h2>{$t("study.music.tab_albums")}</h2>
        <div class="album-grid">
          {#each albums as album (album.name + (album.artist ?? ""))}
            <AlbumCard {album} href={`/study/music/album/by-name?name=${encodeURIComponent(album.name)}${album.artist ? `&artist=${encodeURIComponent(album.artist)}` : ""}`} />
          {/each}
        </div>
      </section>
    {/if}

    {#if artists.length > 0}
      <section class="block">
        <h2>{$t("study.music.tab_artists")}</h2>
        <ul class="artist-pills">
          {#each artists as name (name)}
            <li>
              <a class="artist-pill" href={`/study/music/artist/by-name?name=${encodeURIComponent(name)}`}>
                {name}
              </a>
            </li>
          {/each}
        </ul>
      </section>
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
  .block h2 { margin: 0; font-size: 16px; font-weight: 700; color: var(--secondary); }
  .track-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 16px;
  }
  .artist-pills {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .artist-pill {
    display: inline-flex;
    padding: 6px 14px;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 999px;
    color: var(--secondary);
    font-size: 12px;
    text-decoration: none;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .artist-pill:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .muted { color: var(--tertiary); font-size: 13px; }
</style>
