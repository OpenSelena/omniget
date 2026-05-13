<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import { playlistsStore, type Playlist } from "$lib/study-music/playlists-store.svelte";
  import {
    spotifyStore,
    type SpotifyTrack,
  } from "$lib/study-music/spotify-store.svelte";
  import {
    soundcloudStore,
    type ScTrack,
  } from "$lib/study-music/soundcloud-store.svelte";
  import { colorFromString } from "$lib/study-music/format";
  import {
    studyMusicContinueListening,
    type MusicContinueEntry,
  } from "$lib/study-bridge";
  import NavigationTitle from "$lib/study-music-components/NavigationTitle.svelte";
  import SpeedDialGridItem from "$lib/study-music-components/SpeedDialGridItem.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";

  type Album = {
    name: string;
    artist: string | null;
    track_count: number;
    total_duration_ms: number;
    year: number | null;
    cover_path: string | null;
  };
  type Artist = { name: string; album_count: number; track_count: number };

  let recentAlbums = $state<Album[]>([]);
  let topArtists = $state<Artist[]>([]);
  let continueEntries = $state<MusicContinueEntry[]>([]);

  let loadingHome = $state(true);

  let pullStartY = $state(0);
  let pullDelta = $state(0);
  let pullActive = $state(false);
  let isMobile = $state(false);

  async function loadHome() {
    loadingHome = true;

    const [continueRes, albumsRes, artistsRes] = await Promise.allSettled([
      studyMusicContinueListening({ limit: 12 }),
      pluginInvoke<{ albums: Album[] }>("study", "study:music:albums:list", {
        sort: "added",
        limit: 10,
      }),
      pluginInvoke<{ artists: Artist[] }>("study", "study:music:artists:list", {
        limit: 40,
      }),
    ]);

    if (continueRes.status === "fulfilled") {
      continueEntries = (continueRes.value.entries ?? [])
        .filter((e) => e.source !== "youtube")
        .slice(0, 8);
    }
    if (albumsRes.status === "fulfilled") recentAlbums = albumsRes.value.albums ?? [];
    if (artistsRes.status === "fulfilled") topArtists = artistsRes.value.artists ?? [];

    loadingHome = false;
  }

  function coverUrl(path: string | null | undefined): string | null {
    if (!path) return null;
    if (/^https?:\/\//i.test(path)) return path;
    if (path.startsWith("external://")) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  async function playContinue(entry: MusicContinueEntry) {
    try {
      if (entry.track_id != null) {
        const res = await pluginInvoke<{ track: MusicTrack }>(
          "study",
          "study:music:tracks:get",
          { id: entry.track_id },
        );
        if (res.track) await musicPlayer.play(res.track);
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function openAlbum(album: Album) {
    const params = new URLSearchParams({ name: album.name });
    if (album.artist) params.set("artist", album.artist);
    await goto(`/study/music/albums/by-name?${params.toString()}`);
  }

  async function openArtist(artist: Artist) {
    const params = new URLSearchParams({ name: artist.name });
    await goto(`/study/music/artist/by-name?${params.toString()}`);
  }

  function openPlaylist(p: Playlist) {
    goto(`/study/music/playlists/${p.id}`);
  }

  function handleTouchStart(e: TouchEvent) {
    if (!isMobile) return;
    const scrollEl = document.scrollingElement ?? document.documentElement;
    if (scrollEl.scrollTop > 4) return;
    pullStartY = e.touches[0]?.clientY ?? 0;
    pullActive = true;
  }
  function handleTouchMove(e: TouchEvent) {
    if (!pullActive) return;
    const y = e.touches[0]?.clientY ?? 0;
    pullDelta = Math.max(0, y - pullStartY);
  }
  async function handleTouchEnd() {
    if (!pullActive) return;
    const should = pullDelta > 100;
    pullActive = false;
    pullDelta = 0;
    if (should) await loadHome();
  }

  onMount(() => {
    void loadHome();
    void playlistsStore.load();
    void spotifyStore.refreshStatus().then(() => {
      if (
        spotifyStore.status.logged_in &&
        spotifyStore.savedTracks.length === 0 &&
        spotifyStore.recentlyPlayed.length === 0
      ) {
        void spotifyStore.loadAll();
      }
    });
    void soundcloudStore.refreshStatus().then(() => {
      if (soundcloudStore.isLoggedIn && soundcloudStore.likedTracks.length === 0) {
        void soundcloudStore.loadAll();
      }
    });
    isMobile = window.matchMedia("(max-width: 760px)").matches;
  });

  function spotifyCover(images: { url: string; width?: number | null }[] | undefined): string | null {
    return spotifyStore.pickImage(images, 300);
  }

  async function playSc(track: ScTrack, queue?: ScTrack[]) {
    try {
      await soundcloudStore.playTrack(track, queue);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function playSpotifyTrack(track: SpotifyTrack, queue?: SpotifyTrack[]) {
    try {
      await spotifyStore.playTrack(track, queue);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  const hasSpotifyContent = $derived(
    spotifyStore.status.logged_in &&
      (spotifyStore.savedTracks.length > 0 ||
        spotifyStore.playlists.length > 0 ||
        spotifyStore.recentlyPlayed.length > 0 ||
        spotifyStore.topArtists.length > 0),
  );
  const hasSoundcloudContent = $derived(
    soundcloudStore.isLoggedIn &&
      (soundcloudStore.likedTracks.length > 0 ||
        soundcloudStore.playlists.length > 0 ||
        soundcloudStore.streamFeed.length > 0),
  );
</script>

<svelte:window
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
/>

<section class="music-home">
  {#if pullActive && pullDelta > 0}
    <div class="pull-indicator" style:--p={Math.min(1, pullDelta / 100)}>
      {pullDelta > 100 ? $t("study.music.pull_to_refresh_release") : $t("study.music.pull_to_refresh_hint")}
    </div>
  {/if}

  <header class="page-head">
    <h1>{$t("study.music.nav_home")}</h1>
  </header>

  <div class="speed-dial-row">
    <SpeedDialGridItem label={$t("study.music.speed_dial_library") as string} href="/study/music/library">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 5v14"/><path d="M8 5v14"/><path d="M13 5h7v14h-7z"/></svg>
      {/snippet}
    </SpeedDialGridItem>
    <SpeedDialGridItem label={$t("study.music.speed_dial_albums") as string} href="/study/music/albums">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3"/></svg>
      {/snippet}
    </SpeedDialGridItem>
    <SpeedDialGridItem label={$t("study.music.speed_dial_artists") as string} href="/study/music/artists">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M16 11a4 4 0 1 1-8 0 4 4 0 0 1 8 0z"/><path d="M4 21v-1a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v1"/></svg>
      {/snippet}
    </SpeedDialGridItem>
    <SpeedDialGridItem label={$t("study.music.speed_dial_playlists") as string} href="/study/music/playlists">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15V6"/><path d="M3 18V9"/><path d="M3 18a3 3 0 1 0 6 0V9a3 3 0 1 0-6 0"/><path d="M15 6a3 3 0 1 0 6 0 3 3 0 1 0-6 0"/></svg>
      {/snippet}
    </SpeedDialGridItem>
    <SpeedDialGridItem label={$t("study.music.speed_dial_favorites") as string} href="/study/music/favorites">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78L12 21.23l8.84-8.84a5.5 5.5 0 0 0 0-7.78z"/></svg>
      {/snippet}
    </SpeedDialGridItem>
    <SpeedDialGridItem label={$t("study.music.speed_dial_genres") as string} href="/study/music/genres">
      {#snippet icon()}
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6h16"/><path d="M4 12h16"/><path d="M4 18h10"/></svg>
      {/snippet}
    </SpeedDialGridItem>
  </div>

  <section class="block">
    <NavigationTitle title={$t("study.music.shelf_continue") as string} seeAllHref="/study/music/history" />
    {#if loadingHome}
      <YoutubeSkeleton kind="row" count={4} />
    {:else if continueEntries.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.shelf_continue_empty") as string}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each continueEntries as entry (entry.source + (entry.external_id ?? entry.track_id ?? entry.title))}
          <button
            type="button"
            class="continue-card"
            onclick={() => playContinue(entry)}
          >
            <div class="cover">
              {#if coverUrl(entry.cover_url)}
                <img src={coverUrl(entry.cover_url)} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(entry.title)}></div>
              {/if}
              <div class="progress-track">
                <div class="progress-fill" style:width={`${Math.round(entry.progress * 100)}%`}></div>
              </div>
            </div>
            <h3 class="card-title">{entry.title}</h3>
            {#if entry.artist}<p class="card-sub">{entry.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.shelf_recent_albums") as string} seeAllHref="/study/music/albums" />
    {#if loadingHome}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if recentAlbums.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.shelf_recent_albums_empty") as string}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each recentAlbums as album (album.name + (album.artist ?? ""))}
          <button
            type="button"
            class="discover-card"
            onclick={() => openAlbum(album)}
          >
            <div class="cover">
              {#if coverUrl(album.cover_path)}
                <img src={coverUrl(album.cover_path)} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(album.name)}></div>
              {/if}
            </div>
            <h3 class="card-title">{album.name}</h3>
            {#if album.artist}<p class="card-sub">{album.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.shelf_artists") as string} seeAllHref="/study/music/artists" />
    {#if loadingHome}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if topArtists.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.shelf_artists_empty") as string}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each topArtists.slice(0, 20) as artist (artist.name)}
          <button
            type="button"
            class="artist-card"
            onclick={() => openArtist(artist)}
          >
            <div class="artist-avatar" style:background={colorFromString(artist.name)}>
              <span class="artist-initial">{artist.name.charAt(0).toUpperCase()}</span>
            </div>
            <h3 class="card-title artist-name">{artist.name}</h3>
            <p class="card-sub">
              {$t("study.music.albums_count", { count: artist.album_count })}
            </p>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  {#if playlistsStore.list.length > 0}
    <section class="block">
      <NavigationTitle title={$t("study.music.playlists_title") as string} seeAllHref="/study/music/playlists" />
      <div class="h-scroll">
        {#each playlistsStore.list.slice(0, 10) as p (p.id)}
          <button type="button" class="discover-card" onclick={() => openPlaylist(p)}>
            <div class="cover">
              {#if coverUrl(p.resolved_cover)}
                <img src={coverUrl(p.resolved_cover)} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(p.name)}></div>
              {/if}
            </div>
            <h3 class="card-title">{p.name}</h3>
            <p class="card-sub">{$t("study.music.tracks_count", { count: p.track_count })}</p>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if hasSpotifyContent && spotifyStore.recentlyPlayed.length > 0}
    <section class="block spotify-block">
      <NavigationTitle title={$t("study.music.shelf_spotify_recent") as string} seeAllHref="/study/music/spotify" />
      <div class="h-scroll">
        {#each spotifyStore.recentlyPlayed.slice(0, 12) as track (track.id + (track.played_at ?? ""))}
          <button
            type="button"
            class="discover-card"
            onclick={() => playSpotifyTrack(track, spotifyStore.recentlyPlayed)}
          >
            <div class="cover">
              {#if spotifyCover(track.album.images)}
                <img src={spotifyCover(track.album.images)} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(track.album.name)}></div>
              {/if}
            </div>
            <h3 class="card-title">{track.name}</h3>
            <p class="card-sub">{track.artists.map((a) => a.name).join(", ")}</p>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if hasSoundcloudContent && soundcloudStore.likedTracks.length > 0}
    <section class="block sc-block">
      <NavigationTitle title={$t("study.music.shelf_soundcloud_likes") as string} seeAllHref="/study/music/soundcloud" />
      <div class="h-scroll">
        {#each soundcloudStore.likedTracks.slice(0, 12) as track (track.id)}
          <button
            type="button"
            class="discover-card"
            onclick={() => playSc(track, soundcloudStore.likedTracks)}
          >
            <div class="cover">
              {#if track.artwork_url || track.user.avatar_url}
                <img src={soundcloudStore.pickArtwork(track.artwork_url ?? track.user.avatar_url)} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(track.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{track.title}</h3>
            <p class="card-sub">{track.user.username}</p>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <a class="yt-cta" href="/study/music/youtube">
    <span class="yt-cta-icon" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor"><path d="M22.54 6.42a2.78 2.78 0 0 0-1.94-2C18.88 4 12 4 12 4s-6.88 0-8.6.46a2.78 2.78 0 0 0-1.94 2A29 29 0 0 0 1 11.75a29 29 0 0 0 .46 5.33A2.78 2.78 0 0 0 3.4 19c1.72.46 8.6.46 8.6.46s6.88 0 8.6-.46a2.78 2.78 0 0 0 1.94-2 29 29 0 0 0 .46-5.25 29 29 0 0 0-.46-5.33z"/><polygon points="9.75 15.02 15.5 11.75 9.75 8.48 9.75 15.02" fill="#000"/></svg>
    </span>
    <span class="yt-cta-body">
      <span class="yt-cta-title">{$t("study.music.explore_youtube_cta")}</span>
      <span class="yt-cta-sub">{$t("study.music.explore_youtube_cta_body")}</span>
    </span>
    <span class="yt-cta-arrow" aria-hidden="true">→</span>
  </a>
</section>

<style>
  .music-home {
    display: flex;
    flex-direction: column;
    gap: 28px;
    color: rgba(255, 255, 255, 0.95);
    position: relative;
  }
  .page-head h1 {
    margin: 0;
    font-size: clamp(28px, 3.5vw, 40px);
    font-weight: 900;
    letter-spacing: -0.02em;
    color: white;
  }
  .pull-indicator {
    position: sticky;
    top: 0;
    z-index: 5;
    text-align: center;
    padding: 6px 0;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.6), transparent);
    color: rgba(255, 255, 255, 0.78);
    font-size: 12px;
    font-weight: 600;
    opacity: var(--p, 1);
  }
  .speed-dial-row {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px;
  }
  @media (max-width: 760px) {
    .speed-dial-row {
      grid-template-columns: repeat(3, 1fr);
    }
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .h-scroll {
    display: flex;
    gap: 18px;
    overflow-x: auto;
    overflow-y: visible;
    padding: 4px 2px 16px;
    scrollbar-width: none;
    scroll-behavior: smooth;
  }
  .h-scroll::-webkit-scrollbar { display: none; }

  .continue-card,
  .discover-card,
  .artist-card {
    flex: 0 0 auto;
    background: transparent;
    border: 0;
    padding: 0;
    text-align: left;
    color: inherit;
    cursor: pointer;
    font: inherit;
  }
  .continue-card { width: 160px; }
  .discover-card { width: 176px; }
  .artist-card { width: 130px; text-align: center; }

  .cover {
    position: relative;
    aspect-ratio: 1 / 1;
    border-radius: 12px;
    overflow: hidden;
    background: rgba(40, 40, 40, 0.6);
    margin-bottom: 10px;
    transition: transform 200ms ease;
  }
  .continue-card:hover .cover,
  .discover-card:hover .cover {
    transform: scale(1.02);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cover-fallback {
    width: 100%;
    height: 100%;
  }
  .progress-track {
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 6px;
    height: 3px;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: inherit;
  }
  .artist-avatar {
    width: 130px;
    height: 130px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    margin: 0 auto 10px;
    transition: transform 200ms ease;
  }
  .artist-card:hover .artist-avatar {
    transform: scale(1.04);
  }
  .artist-initial {
    font-size: 42px;
    font-weight: 900;
    color: rgba(255, 255, 255, 0.92);
    letter-spacing: -0.02em;
  }
  .artist-name {
    text-align: center;
    white-space: normal;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .card-title {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }
  .card-sub {
    margin: 2px 0 0;
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }

  .yt-cta {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 14px;
    color: rgba(255, 255, 255, 0.92);
    text-decoration: none;
    transition: background 160ms ease, border-color 160ms ease, transform 160ms ease;
    max-width: 480px;
    align-self: flex-start;
  }
  .yt-cta:hover {
    background: rgba(255, 255, 255, 0.07);
    border-color: rgba(255, 255, 255, 0.15);
    transform: translateY(-1px);
  }
  .yt-cta-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: #ff0033;
    display: grid;
    place-items: center;
    color: white;
    flex-shrink: 0;
  }
  .yt-cta-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .yt-cta-title {
    font-size: 14px;
    font-weight: 700;
    color: white;
  }
  .yt-cta-sub {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .yt-cta-arrow {
    color: rgba(255, 255, 255, 0.6);
    font-size: 18px;
    flex-shrink: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .cover, .continue-card:hover .cover, .discover-card:hover .cover,
    .artist-avatar, .artist-card:hover .artist-avatar, .yt-cta, .yt-cta:hover {
      transition: none;
      transform: none;
    }
  }
</style>
