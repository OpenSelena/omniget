<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { colorFromString } from "$lib/study-music/format";
  import { spotifyStore } from "$lib/study-music/spotify-store.svelte";
  import { soundcloudStore } from "$lib/study-music/soundcloud-store.svelte";

  type Artist = { name: string; album_count: number; track_count: number };
  type Album = { artist: string | null; cover_path: string | null };

  let artists = $state<Artist[]>([]);
  let covers = $state<Map<string, string>>(new Map());
  let loading = $state(true);
  let q = $state("");

  async function load() {
    loading = true;
    try {
      const res = await pluginInvoke<{ artists: Artist[] }>(
        "study",
        "study:music:artists:list",
        { limit: 1000 },
      );
      artists = res.artists ?? [];
      void resolveCovers();
    } finally {
      loading = false;
    }
  }

  async function resolveCovers() {
    const next = new Map(covers);
    for (const a of artists.slice(0, 60)) {
      if (next.has(a.name)) continue;
      try {
        const res = await pluginInvoke<{ albums: Album[] }>(
          "study",
          "study:music:albums:list",
          { artist: a.name, limit: 1 },
        );
        const cover = res.albums?.[0]?.cover_path;
        if (cover) next.set(a.name, cover);
      } catch {
        /* ignore */
      }
    }
    covers = next;
    void resolveMusicBrainz();
  }

  async function resolveMusicBrainz() {
    const next = new Map(covers);
    for (const a of artists.slice(0, 30)) {
      if (next.has(a.name)) continue;
      try {
        const res = await pluginInvoke<{ found: boolean; image_path: string | null }>(
          "study",
          "study:music:musicbrainz:artist",
          { name: a.name },
        );
        if (res.found && res.image_path) {
          next.set(a.name, res.image_path);
        }
      } catch {
        /* ignore */
      }
    }
    covers = next;
  }

  function coverUrl(path: string | null | undefined): string | null {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  function openArtist(name: string) {
    goto(`/study/music/artist/by-name?name=${encodeURIComponent(name)}`);
  }

  onMount(async () => {
    void load();
    if (
      spotifyStore.status.logged_in &&
      spotifyStore.topArtists.length === 0
    ) {
      try {
        await spotifyStore.loadAll();
      } catch {
        /* ignore */
      }
    }
    if (
      soundcloudStore.isLoggedIn &&
      soundcloudStore.followings.length === 0
    ) {
      try {
        await soundcloudStore.loadAll();
      } catch {
        /* ignore */
      }
    }
  });

  const filteredScArtists = $derived.by(() => {
    const term = q.trim().toLowerCase();
    const all = soundcloudStore.followings;
    if (!term) return all;
    return all.filter((u) => u.username.toLowerCase().includes(term));
  });

  function openSpotifyArtist(id: string) {
    goto(`/study/music/spotify/artist/${id}`);
  }

  function spotifyCover(images: { url: string; width?: number | null }[]): string | null {
    return spotifyStore.pickImage(images, 300);
  }

  const filtered = $derived.by(() => {
    const term = q.trim().toLowerCase();
    if (!term) return artists;
    return artists.filter((a) => a.name.toLowerCase().includes(term));
  });

  const filteredSpotifyArtists = $derived.by(() => {
    const term = q.trim().toLowerCase();
    const all = spotifyStore.allArtists;
    if (!term) return all;
    return all.filter((a) => a.name.toLowerCase().includes(term));
  });
</script>

<section class="artists-page">
  <header class="page-head">
    <h1>{$t("study.music.nav_artists")}</h1>
    <div class="head-actions">
      <input
        type="search"
        class="search"
        placeholder={$t("study.music.search_placeholder")}
        bind:value={q}
      />
    </div>
  </header>

  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if filtered.length === 0}
    <div class="empty">
      <p>{$t("study.music.artists_empty")}</p>
    </div>
  {:else}
    <p class="result-count">{filtered.length} artista(s)</p>
    <div class="artist-grid">
      {#each filtered as a (a.name)}
        <div
          class="artist-card"
          role="button"
          tabindex="0"
          onclick={() => openArtist(a.name)}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openArtist(a.name); } }}
        >
          <div class="artist-circle">
            {#if covers.get(a.name)}
              <img src={coverUrl(covers.get(a.name))} alt="" loading="lazy" />
            {:else}
              <div class="artist-fallback" style:background={colorFromString(a.name)}>
                {a.name.slice(0, 1).toUpperCase()}
              </div>
            {/if}
          </div>
          <h3 class="artist-name">{a.name}</h3>
          <p class="artist-meta">
            {a.album_count} álbum(ns) · {a.track_count} faixa(s)
          </p>
        </div>
      {/each}
    </div>
  {/if}

  {#if filteredScArtists.length > 0}
    <section class="spotify-block">
      <header class="block-head">
        <h2><span style="color: #ff5500">●</span> Seguindo no SoundCloud ({filteredScArtists.length})</h2>
      </header>
      <div class="artist-grid">
        {#each filteredScArtists as u (u.id)}
          <a class="artist-card" href={`/study/music/soundcloud/user/${u.id}`}>
            <div class="artist-circle">
              {#if u.avatar_url}
                <img src={soundcloudStore.pickArtwork(u.avatar_url)} alt="" loading="lazy" />
              {:else}
                <div class="artist-fallback" style:background={colorFromString(u.username)}>{u.username.slice(0,1).toUpperCase()}</div>
              {/if}
            </div>
            <h3 class="artist-name">{u.username}</h3>
            {#if u.followers_count}<p class="artist-meta">{u.followers_count.toLocaleString("pt-BR")} seguidores</p>{/if}
          </a>
        {/each}
      </div>
    </section>
  {/if}

  {#if filteredSpotifyArtists.length > 0}
    <section class="spotify-block">
      <header class="block-head">
        <h2>
          <span class="spotify-mark" aria-hidden="true">
            <svg viewBox="0 0 168 168" width="14" height="14"><circle cx="84" cy="84" r="84" fill="#1db954"/><path fill="#000" d="M119.6 110.6c-1.5 2.5-4.7 3.3-7.2 1.8-19.7-12-44.5-14.7-73.7-8-2.8.6-5.6-1.1-6.3-3.9-.6-2.8 1.1-5.6 3.9-6.3 31.9-7.3 59.4-4.2 81.5 9.2 2.5 1.5 3.3 4.7 1.8 7.2zm9.5-21.2c-1.9 3.1-5.9 4.1-9 2.2-22.6-13.9-57-17.9-83.8-9.8-3.5 1.1-7.1-.9-8.2-4.3-1.1-3.5.9-7.1 4.3-8.2 30.6-9.3 68.5-4.8 94.5 11.1 3.1 1.9 4.1 5.9 2.2 9zm.8-22c-27-16-71.6-17.5-97.4-9.7-4.1 1.2-8.4-1.1-9.6-5.2-1.2-4.1 1.1-8.4 5.2-9.6 29.6-9 78.7-7.2 109.8 11.3 3.7 2.2 4.9 7 2.7 10.7-2.2 3.7-7 4.9-10.7 2.5z"/></svg>
          </span>
          Artistas do Spotify ({filteredSpotifyArtists.length})
        </h2>
      </header>
      <div class="artist-grid">
        {#each filteredSpotifyArtists as a (a.id)}
          <button
            type="button"
            class="artist-card"
            onclick={() => openSpotifyArtist(a.id)}
          >
            <div class="artist-circle">
              {#if spotifyCover(a.images)}
                <img src={spotifyCover(a.images)} alt="" loading="lazy" />
              {:else}
                <div class="artist-fallback" style:background={colorFromString(a.name)}>
                  {a.name.slice(0, 1).toUpperCase()}
                </div>
              {/if}
            </div>
            <h3 class="artist-name">{a.name}</h3>
            {#if a.genres.length > 0}
              <p class="artist-meta">{a.genres.slice(0, 2).join(", ")}</p>
            {/if}
          </button>
        {/each}
      </div>
    </section>
  {/if}
</section>

<style>
  .artists-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
    color: rgba(255, 255, 255, 0.95);
  }
  .spotify-block { display: flex; flex-direction: column; gap: 16px; margin-top: 24px; padding-top: 24px; border-top: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent); }
  .block-head { display: flex; align-items: center; gap: 12px; }
  .block-head h2 { margin: 0; font-size: 18px; font-weight: 800; color: var(--secondary); display: inline-flex; align-items: center; gap: 10px; }
  .spotify-mark { display: inline-flex; align-items: center; }
  .spotify-block .artist-card { background: transparent; border: 0; padding: 0; cursor: pointer; color: inherit; text-align: center; }
  .page-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .page-head h1 {
    margin: 0;
    font-size: clamp(28px, 3.5vw, 40px);
    font-weight: 900;
    letter-spacing: -0.02em;
    color: rgba(255, 255, 255, 0.95);
  }
  .head-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .search {
    padding: 8px 14px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    font-size: 13px;
    min-width: 240px;
    outline: none;
  }
  .search:focus { border-color: var(--accent); }
  .result-count {
    margin: 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
  }
  .empty {
    padding: 64px 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
  }
  .artist-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 24px;
  }
  .artist-card {
    cursor: pointer;
    background: transparent;
    border: 0;
    color: inherit;
    padding: 0;
    text-align: center;
  }
  .artist-circle {
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    background: rgba(40, 40, 40, 0.8);
    overflow: hidden;
    margin: 0 auto 14px;
    position: relative;
    transition: transform 300ms ease;
  }
  .artist-card:hover .artist-circle {
    transform: scale(1.04);
  }
  .artist-circle img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 700ms ease;
  }
  .artist-card:hover .artist-circle img {
    transform: scale(1.1);
  }
  .artist-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: white;
    font-size: clamp(28px, 4vw, 56px);
    font-weight: 800;
  }
  .artist-name {
    margin: 0;
    color: white;
    font-weight: 700;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 4px;
  }
  .artist-meta {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    padding: 0 4px;
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 13px; }
  @media (prefers-reduced-motion: reduce) {
    .artist-circle, .artist-circle img { transition: none; }
    .artist-card:hover .artist-circle, .artist-card:hover .artist-circle img { transform: none; }
  }
</style>
