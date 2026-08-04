<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    spotifyStore,
    type SpotifyArtist,
    type SpotifyPlaylist,
    type SpotifyTrack,
  } from "$lib/study-music/spotify-store.svelte";
  import { colorFromString } from "$lib/study-music/format";

  type Tab = "track" | "playlist" | "artist" | "album";
  type SearchAlbum = {
    id: string;
    name: string;
    uri: string;
    images: { url: string; width?: number | null; height?: number | null }[];
    artists: { name: string }[];
    release_date?: string;
  };

  let query = $state("");
  let tab = $state<Tab>("track");
  let inputRef = $state<HTMLInputElement | null>(null);

  let tracks = $state<SpotifyTrack[]>([]);
  let playlists = $state<SpotifyPlaylist[]>([]);
  let artists = $state<SpotifyArtist[]>([]);
  let albums = $state<SearchAlbum[]>([]);
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    if (!spotifyStore.status.logged_in) {
      goto("/study/music/spotify");
      return;
    }
    queueMicrotask(() => inputRef?.focus());
  });

  function onInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!query.trim()) {
      tracks = [];
      playlists = [];
      artists = [];
      albums = [];
      return;
    }
    debounceTimer = setTimeout(() => doSearch(), 300);
  }

  async function doSearch() {
    const q = query.trim();
    if (!q) return;
    loading = true;
    try {
      const local = spotifyStore.searchLocal(q);
      tracks = local.tracks;
      artists = local.artists;
      playlists = local.playlists;
      const albumsMap = new Map<string, SearchAlbum>();
      for (const t of local.tracks) {
        const a = t.album;
        if (!a?.id || albumsMap.has(a.id)) continue;
        if (!a.name.toLowerCase().includes(q.toLowerCase())) continue;
        albumsMap.set(a.id, {
          id: a.id,
          name: a.name,
          uri: a.uri,
          images: a.images ?? [],
          artists: t.artists.map((x) => ({ name: x.name })),
          release_date: a.release_date,
        });
      }
      albums = [...albumsMap.values()];
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      loading = false;
    }
  }

  async function playTrack(track: SpotifyTrack, queue?: SpotifyTrack[]) {
    try {
      const mode = await spotifyStore.playTrack(track, queue);
      if (mode === "youtube") {
        showToast("info", "Tocando via YouTube (modo Free)");
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function fmtDuration(ms: number): string {
    const total = Math.floor(ms / 1000);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

<section class="search-page">
  <header class="head">
    <button type="button" class="back-btn" onclick={() => goto("/study/music/spotify")}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <polyline points="15 18 9 12 15 6"/>
      </svg>
      Spotify
    </button>
  </header>

  <div class="search-bar">
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="8"/>
      <line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      bind:this={inputRef}
      bind:value={query}
      oninput={onInput}
      type="search"
      placeholder="Buscar nas suas curtidas, recents e top tracks…"
      autocomplete="off"
    />
    {#if loading}
      <span class="spinner"></span>
    {/if}
  </div>

  {#if tracks.length || playlists.length || artists.length || albums.length}
    <div class="tabs" role="tablist">
      <button
        type="button"
        class="tab"
        class:on={tab === "track"}
        onclick={() => (tab = "track")}
        role="tab"
        aria-selected={tab === "track"}
      >
        Faixas {tracks.length > 0 ? `(${tracks.length})` : ""}
      </button>
      <button
        type="button"
        class="tab"
        class:on={tab === "playlist"}
        onclick={() => (tab = "playlist")}
        role="tab"
        aria-selected={tab === "playlist"}
      >
        Playlists {playlists.length > 0 ? `(${playlists.length})` : ""}
      </button>
      <button
        type="button"
        class="tab"
        class:on={tab === "artist"}
        onclick={() => (tab = "artist")}
        role="tab"
        aria-selected={tab === "artist"}
      >
        Artistas {artists.length > 0 ? `(${artists.length})` : ""}
      </button>
      <button
        type="button"
        class="tab"
        class:on={tab === "album"}
        onclick={() => (tab = "album")}
        role="tab"
        aria-selected={tab === "album"}
      >
        Álbuns {albums.length > 0 ? `(${albums.length})` : ""}
      </button>
    </div>
  {/if}

  {#if tab === "track" && tracks.length > 0}
    <div class="track-list">
      {#each tracks as t, i (t.id + i)}
        <button type="button" class="track-row" onclick={() => playTrack(t, tracks)}>
          <div class="track-cover">
            {#if spotifyStore.pickImage(t.album.images, 80)}
              <img src={spotifyStore.pickImage(t.album.images, 80)} alt="" loading="lazy" />
            {/if}
          </div>
          <div class="track-meta">
            <span class="track-title">{t.name}</span>
            <span class="track-artists">{t.artists.map((a) => a.name).join(", ")}</span>
          </div>
          <span class="track-album">{t.album.name}</span>
          <span class="track-dur">{fmtDuration(t.duration_ms)}</span>
        </button>
      {/each}
    </div>
  {:else if tab === "playlist" && playlists.length > 0}
    <div class="grid">
      {#each playlists as p (p.id)}
        <button
          type="button"
          class="card"
          onclick={() => goto(`/study/music/spotify/playlist/${p.id}`)}
        >
          <div class="cover">
            {#if spotifyStore.pickImage(p.images, 300)}
              <img src={spotifyStore.pickImage(p.images, 300)} alt="" loading="lazy" />
            {:else}
              <div class="cover-fallback" style:background={colorFromString(p.name)}></div>
            {/if}
          </div>
          <h3>{p.name}</h3>
          <p>{p.owner_name ?? ""}</p>
        </button>
      {/each}
    </div>
  {:else if tab === "artist" && artists.length > 0}
    <div class="grid">
      {#each artists as a (a.id)}
        <button
          type="button"
          class="card artist"
          onclick={() => goto(`/study/music/spotify/artist/${a.id}`)}
        >
          <div class="cover round">
            {#if spotifyStore.pickImage(a.images, 300)}
              <img src={spotifyStore.pickImage(a.images, 300)} alt="" loading="lazy" />
            {:else}
              <div class="cover-fallback round" style:background={colorFromString(a.name)}>
                {a.name.slice(0, 1).toUpperCase()}
              </div>
            {/if}
          </div>
          <h3>{a.name}</h3>
          <p>Artista</p>
        </button>
      {/each}
    </div>
  {:else if tab === "album" && albums.length > 0}
    <div class="grid">
      {#each albums as a (a.id)}
        <button
          type="button"
          class="card"
          onclick={() => goto(`/study/music/spotify/album/${a.id}`)}
        >
          <div class="cover">
            {#if spotifyStore.pickImage(a.images, 300)}
              <img src={spotifyStore.pickImage(a.images, 300)} alt="" loading="lazy" />
            {:else}
              <div class="cover-fallback" style:background={colorFromString(a.name)}></div>
            {/if}
          </div>
          <h3>{a.name}</h3>
          <p>{a.artists.map((x) => x.name).join(", ")}</p>
        </button>
      {/each}
    </div>
  {:else if query.trim() && !loading}
    <p class="muted">Nada encontrado para "{query}"</p>
  {/if}
</section>

<style>
  .search-page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    color: rgba(255, 255, 255, 0.95);
  }
  .head {
    display: flex;
    gap: 8px;
  }
  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px 6px 8px;
    background: rgba(255, 255, 255, 0.05);
    border: 0;
    border-radius: 999px;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .back-btn:hover { background: rgba(255, 255, 255, 0.1); }

  .search-bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 999px;
    padding: 12px 20px;
    color: rgba(255, 255, 255, 0.6);
  }
  .search-bar input {
    flex: 1;
    background: transparent;
    border: 0;
    outline: 0;
    color: white;
    font-family: inherit;
    font-size: 15px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: #1db954;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .tabs {
    display: flex;
    gap: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 0;
  }
  .tab {
    padding: 8px 16px;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: color 200ms ease, border-color 200ms ease;
  }
  .tab:hover { color: white; }
  .tab.on { color: white; border-bottom-color: #1db954; }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 24px;
  }
  .card {
    background: transparent;
    border: 0;
    color: inherit;
    cursor: pointer;
    text-align: left;
    padding: 0;
  }
  .cover {
    aspect-ratio: 1 / 1;
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 12px;
    background: rgba(40, 40, 40, 0.8);
  }
  .cover.round { border-radius: 50%; }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 500ms ease;
  }
  .card:hover .cover img { transform: scale(1.05); }
  .cover-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: white;
    font-size: 32px;
    font-weight: 800;
  }
  .cover-fallback.round { border-radius: 50%; }
  .card h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 4px;
  }
  .card p {
    margin: 4px 0 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 4px;
  }
  .card.artist { text-align: center; }
  .card.artist h3, .card.artist p { padding: 0; }

  .track-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .track-row {
    display: grid;
    grid-template-columns: 56px 1fr 1fr 60px;
    gap: 12px;
    align-items: center;
    padding: 8px 12px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: inherit;
    cursor: pointer;
    text-align: left;
    transition: background 200ms ease;
  }
  .track-row:hover { background: rgba(255, 255, 255, 0.06); }
  .track-cover {
    width: 44px;
    height: 44px;
    border-radius: 4px;
    overflow: hidden;
    background: rgba(40, 40, 40, 0.8);
  }
  .track-cover img { width: 100%; height: 100%; object-fit: cover; }
  .track-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .track-title {
    font-size: 14px;
    font-weight: 600;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-artists {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-album {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-dur {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.55);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .muted {
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
    text-align: center;
    padding: 32px 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .card .cover img, .spinner, .tab { transition: none; animation: none; }
    .card:hover .cover img { transform: none; }
  }
</style>
