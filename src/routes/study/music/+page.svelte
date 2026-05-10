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
  import AlbumPill from "$lib/study-music-components/AlbumPill.svelte";

  type RootEntry = { path: string; enabled: boolean; exists: boolean };
  type Album = {
    name: string;
    artist: string | null;
    track_count: number;
    total_duration_ms: number;
    year: number | null;
    cover_path: string | null;
  };
  type Artist = { name: string; album_count: number; track_count: number };

  let roots = $state<RootEntry[]>([]);
  let allAlbums = $state<Album[]>([]);
  let topArtists = $state<Artist[]>([]);
  let artistCovers = $state<Map<string, string>>(new Map());
  let loading = $state(true);
  let scanning = $state(false);
  let scanMsg = $state("");
  let heroAlbum = $state<Album | null>(null);
  let heroFavorite = $state(false);

  async function loadAll() {
    loading = true;
    try {
      const [rootsRes, albumsRes, artistsRes] = await Promise.allSettled([
        pluginInvoke<{ roots: RootEntry[] }>("study", "study:music:roots:list"),
        pluginInvoke<{ albums: Album[] }>(
          "study",
          "study:music:albums:list",
          { limit: 60 },
        ),
        pluginInvoke<{ artists: Artist[] }>(
          "study",
          "study:music:artists:list",
          { limit: 40 },
        ),
      ]);

      if (rootsRes.status === "fulfilled") roots = rootsRes.value.roots ?? [];
      if (albumsRes.status === "fulfilled") {
        allAlbums = albumsRes.value.albums ?? [];
        const withCover = allAlbums.filter((a) => a.cover_path);
        const pool = withCover.length > 0 ? withCover : allAlbums;
        if (pool.length > 0) {
          heroAlbum = pool[Math.floor(Math.random() * pool.length)];
        }
      }
      if (artistsRes.status === "fulfilled") {
        topArtists = artistsRes.value.artists ?? [];
      }

      void resolveArtistCovers();
    } finally {
      loading = false;
    }
  }

  async function resolveArtistCovers() {
    const next = new Map(artistCovers);
    const work = topArtists.slice(0, 20).filter((a) => !next.has(a.name));
    for (const a of work) {
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
    artistCovers = next;
  }

  async function addRoot() {
    try {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({ directory: true, multiple: false });
      if (typeof picked !== "string" || !picked) return;
      await pluginInvoke("study", "study:music:roots:add", { path: picked });
      showToast("success", $t("study.music.root_added", { path: picked }) as string);
      await loadAll();
      await runScan();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  async function runScan() {
    if (scanning) return;
    scanning = true;
    scanMsg = "";
    try {
      const res = await pluginInvoke<{ added: number; updated: number; removed: number }>(
        "study",
        "study:music:scan",
      );
      scanMsg = $t("study.music.scan_done", res) as string;
      await loadAll();
    } catch (e) {
      scanMsg = e instanceof Error ? e.message : String(e);
    } finally {
      scanning = false;
      setTimeout(() => (scanMsg = ""), 6000);
    }
  }

  async function playHero() {
    if (!heroAlbum) return;
    try {
      const args: { name: string; artist?: string } = { name: heroAlbum.name };
      if (heroAlbum.artist) args.artist = heroAlbum.artist;
      const res = await pluginInvoke<{ tracks: MusicTrack[] }>(
        "study",
        "study:music:albums:get",
        args,
      );
      if (res.tracks && res.tracks.length > 0) {
        await musicPlayer.play(res.tracks[0], res.tracks);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  function scrollContainer(id: string, dir: 1 | -1) {
    const el = document.getElementById(id);
    if (!el) return;
    el.scrollBy({ left: dir * 360, behavior: "smooth" });
  }

  function openAlbum(album: Album) {
    const params = new URLSearchParams({ name: album.name });
    if (album.artist) params.set("artist", album.artist);
    goto(`/study/music/album/by-name?${params.toString()}`);
  }

  function openArtist(name: string) {
    goto(`/study/music/artist/by-name?name=${encodeURIComponent(name)}`);
  }

  function openPlaylist(p: Playlist) {
    goto(`/study/music/playlists/${p.id}`);
  }

  function coverUrl(path: string | null | undefined): string | null {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  onMount(async () => {
    void loadAll();
    void playlistsStore.load();
    await spotifyStore.refreshStatus();
    if (
      spotifyStore.status.logged_in &&
      spotifyStore.savedTracks.length === 0 &&
      spotifyStore.recentlyPlayed.length === 0
    ) {
      void spotifyStore.loadAll();
    }
    await soundcloudStore.refreshStatus();
    if (
      soundcloudStore.isLoggedIn &&
      soundcloudStore.likedTracks.length === 0
    ) {
      void soundcloudStore.loadAll();
    }
  });

  async function playSc(track: ScTrack, queue?: ScTrack[]) {
    try {
      await soundcloudStore.playTrack(track, queue);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  async function playSpotifyTrack(track: SpotifyTrack, queue?: SpotifyTrack[]) {
    try {
      const mode = await spotifyStore.playTrack(track, queue);
      if (mode === "youtube") {
        showToast("info", "Tocando via YouTube (modo Free)");
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  function spotifyCover(images: { url: string; width?: number | null }[] | undefined): string | null {
    return spotifyStore.pickImage(images, 300);
  }

  const enabledRoots = $derived(roots.filter((r) => r.enabled));
  const noRoots = $derived(roots.length === 0);
  const hasContent = $derived(allAlbums.length > 0 || topArtists.length > 0);
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

  const recentAlbums = $derived(allAlbums.slice(0, 12));
  const listenNow = $derived.by(() => {
    if (!heroAlbum) return allAlbums.slice(0, 8);
    const heroKey = heroAlbum.name + "|" + (heroAlbum.artist ?? "");
    return allAlbums
      .filter((a) => a.name + "|" + (a.artist ?? "") !== heroKey)
      .slice(0, 8);
  });
</script>

<section class="music-home">
  <header class="page-head">
    <h1>{$t("study.music.nav_home")}</h1>
    <div class="head-actions">
      <button type="button" class="ghost-btn" onclick={addRoot}>
        + {$t("study.music.add_root")}
      </button>
      <button
        type="button"
        class="ghost-btn"
        onclick={runScan}
        disabled={scanning || enabledRoots.length === 0}
      >
        {scanning ? $t("study.music.scanning") : $t("study.music.scan_now")}
      </button>
    </div>
  </header>

  {#if scanMsg}
    <p class="scan-msg">{scanMsg}</p>
  {/if}

  {#if loading && !hasSpotifyContent && !hasSoundcloudContent}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if noRoots && !hasSpotifyContent && !hasSoundcloudContent}
    <div class="empty-music">
      <h2>{$t("study.music.empty_no_roots_title")}</h2>
      <p>{$t("study.music.empty_no_roots_body")}</p>
      <button type="button" class="cta-big" onclick={addRoot}>
        {$t("study.music.add_root_cta")}
      </button>
    </div>
  {:else if !hasContent && !hasSpotifyContent && !hasSoundcloudContent}
    <div class="empty-music">
      <h2>{$t("study.music.empty_no_tracks_title")}</h2>
      <p>{$t("study.music.empty_no_tracks_body")}</p>
      <button type="button" class="cta-big" onclick={runScan} disabled={scanning}>
        {scanning ? $t("study.music.scanning") : $t("study.music.scan_now")}
      </button>
    </div>
  {:else if hasContent}
    {#if heroAlbum}
      <section class="hero">
        <div class="hero-bg" style:background-image={coverUrl(heroAlbum.cover_path) ? `url('${coverUrl(heroAlbum.cover_path)}')` : "none"}></div>
        <div class="hero-overlay"></div>
        <div class="hero-content">
          <span class="hero-eyebrow">
            <svg viewBox="0 0 24 24" width="9" height="9" fill="currentColor" aria-hidden="true">
              <path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z M12.5 7H11v6l5.25 3.15.75-1.23-4.5-2.67z"/>
            </svg>
            {$t("study.music.jump_back_in")}
          </span>
          <h1 class="hero-title">{heroAlbum.name}</h1>
          {#if heroAlbum.artist}
            <p class="hero-artist">{$t("study.music.by")} {heroAlbum.artist}</p>
          {/if}
          <div class="hero-actions">
            <button type="button" class="hero-play" onclick={playHero}>
              <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
              <span>{$t("study.music.start_listening")}</span>
            </button>
            <button
              type="button"
              class="hero-fav"
              class:on={heroFavorite}
              onclick={() => (heroFavorite = !heroFavorite)}
              aria-label={$t("study.music.favorite") as string}
            >
              <svg viewBox="0 0 24 24" width="13" height="13" fill={heroFavorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
              </svg>
            </button>
          </div>
        </div>
      </section>
    {/if}

    {#if listenNow.length > 0}
      <section class="block">
        <header class="block-head">
          <h2>{$t("study.music.shelf_listen_now")}</h2>
        </header>
        <div class="album-pills-grid">
          {#each listenNow as album (album.name + (album.artist ?? ""))}
            <AlbumPill {album} />
          {/each}
        </div>
      </section>
    {/if}

    {#if topArtists.length > 0}
      <section class="block">
        <header class="block-head">
          <h2>{$t("study.music.shelf_top_artists")}</h2>
          <div class="scroll-arrows">
            <button type="button" class="arrow" onclick={() => scrollContainer("artists-scroll", -1)} aria-label="Anterior">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="15 18 9 12 15 6"/></svg>
            </button>
            <button type="button" class="arrow" onclick={() => scrollContainer("artists-scroll", 1)} aria-label="Próximo">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>
        </header>
        <div id="artists-scroll" class="h-scroll">
          {#each topArtists.slice(0, 16) as a (a.name)}
            <div
              class="artist-card"
              role="button"
              tabindex="0"
              onclick={() => openArtist(a.name)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openArtist(a.name); } }}
            >
              <div class="artist-circle">
                {#if artistCovers.get(a.name)}
                  <img src={coverUrl(artistCovers.get(a.name))} alt="" loading="lazy" />
                {:else}
                  <div class="artist-fallback" style:background={colorFromString(a.name)}>
                    {a.name.slice(0, 1).toUpperCase()}
                  </div>
                {/if}
              </div>
              <h3 class="artist-name">{a.name}</h3>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if recentAlbums.length > 0}
      <section class="block">
        <header class="block-head">
          <h2>{$t("study.music.shelf_new_releases")}</h2>
          <div class="scroll-arrows">
            <button type="button" class="arrow" onclick={() => scrollContainer("albums-scroll", -1)} aria-label="Anterior">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="15 18 9 12 15 6"/></svg>
            </button>
            <button type="button" class="arrow" onclick={() => scrollContainer("albums-scroll", 1)} aria-label="Próximo">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>
        </header>
        <div id="albums-scroll" class="h-scroll">
          {#each recentAlbums as album (album.name + (album.artist ?? ""))}
            <div
              class="album-card"
              role="button"
              tabindex="0"
              onclick={() => openAlbum(album)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openAlbum(album); } }}
            >
              <div class="album-card-cover">
                {#if coverUrl(album.cover_path)}
                  <img src={coverUrl(album.cover_path)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(album.name + (album.artist ?? ""))}>
                    <svg viewBox="0 0 24 24" width="40%" height="40%" fill="currentColor" aria-hidden="true"><circle cx="12" cy="12" r="10" opacity="0.2"/><circle cx="12" cy="12" r="3"/></svg>
                  </div>
                {/if}
                <div class="album-card-overlay"></div>
                <button
                  type="button"
                  class="album-card-play"
                  onclick={(e) => { e.stopPropagation(); openAlbum(album); }}
                  aria-label="Tocar"
                >
                  <svg viewBox="0 0 24 24" width="11" height="11" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
                </button>
              </div>
              <h3 class="album-card-title">{album.name}</h3>
              {#if album.artist}
                <p class="album-card-sub">{album.artist}</p>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if playlistsStore.list.length > 0}
      <section class="block">
        <header class="block-head">
          <h2>{$t("study.music.playlists_title")}</h2>
          <div class="scroll-arrows">
            <button type="button" class="arrow" onclick={() => scrollContainer("playlists-scroll", -1)} aria-label="Anterior">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="15 18 9 12 15 6"/></svg>
            </button>
            <button type="button" class="arrow" onclick={() => scrollContainer("playlists-scroll", 1)} aria-label="Próximo">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>
        </header>
        <div id="playlists-scroll" class="h-scroll">
          {#each playlistsStore.list as p (p.id)}
            <div
              class="album-card"
              role="button"
              tabindex="0"
              onclick={() => openPlaylist(p)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openPlaylist(p); } }}
            >
              <div class="album-card-cover">
                {#if coverUrl(p.resolved_cover)}
                  <img src={coverUrl(p.resolved_cover)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(p.name)}>
                    <svg viewBox="0 0 24 24" width="40%" height="40%" fill="currentColor" aria-hidden="true"><path d="M9 18V5l12-2v13" opacity="0.4"/><circle cx="6" cy="18" r="3" opacity="0.4"/><circle cx="18" cy="16" r="3" opacity="0.4"/></svg>
                  </div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{p.name}</h3>
              <p class="album-card-sub">{p.track_count} faixa(s)</p>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}

  {#if spotifyStore.status.logged_in}
    {#if spotifyStore.recentlyPlayed.length > 0}
      <section class="block spotify-block">
        <header class="block-head">
          <h2>
            <span class="spotify-mark" aria-hidden="true">
              <svg viewBox="0 0 168 168" width="14" height="14"><circle cx="84" cy="84" r="84" fill="#1db954"/><path fill="#000" d="M119.6 110.6c-1.5 2.5-4.7 3.3-7.2 1.8-19.7-12-44.5-14.7-73.7-8-2.8.6-5.6-1.1-6.3-3.9-.6-2.8 1.1-5.6 3.9-6.3 31.9-7.3 59.4-4.2 81.5 9.2 2.5 1.5 3.3 4.7 1.8 7.2zm9.5-21.2c-1.9 3.1-5.9 4.1-9 2.2-22.6-13.9-57-17.9-83.8-9.8-3.5 1.1-7.1-.9-8.2-4.3-1.1-3.5.9-7.1 4.3-8.2 30.6-9.3 68.5-4.8 94.5 11.1 3.1 1.9 4.1 5.9 2.2 9zm.8-22c-27-16-71.6-17.5-97.4-9.7-4.1 1.2-8.4-1.1-9.6-5.2-1.2-4.1 1.1-8.4 5.2-9.6 29.6-9 78.7-7.2 109.8 11.3 3.7 2.2 4.9 7 2.7 10.7-2.2 3.7-7 4.9-10.7 2.5z"/></svg>
            </span>
            Tocadas recentemente
          </h2>
        </header>
        <div class="h-scroll">
          {#each spotifyStore.recentlyPlayed.slice(0, 12) as track (track.id + (track.played_at ?? ""))}
            <button
              type="button"
              class="album-card"
              onclick={() => playSpotifyTrack(track, spotifyStore.recentlyPlayed)}
            >
              <div class="album-card-cover">
                {#if spotifyCover(track.album.images)}
                  <img src={spotifyCover(track.album.images)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(track.album.name)}></div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{track.name}</h3>
              <p class="album-card-sub">{track.artists.map((a) => a.name).join(", ")}</p>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if spotifyStore.topArtists.length > 0}
      <section class="block spotify-block">
        <header class="block-head">
          <h2>
            <span class="spotify-mark" aria-hidden="true">
              <svg viewBox="0 0 168 168" width="14" height="14"><circle cx="84" cy="84" r="84" fill="#1db954"/><path fill="#000" d="M119.6 110.6c-1.5 2.5-4.7 3.3-7.2 1.8-19.7-12-44.5-14.7-73.7-8-2.8.6-5.6-1.1-6.3-3.9-.6-2.8 1.1-5.6 3.9-6.3 31.9-7.3 59.4-4.2 81.5 9.2 2.5 1.5 3.3 4.7 1.8 7.2zm9.5-21.2c-1.9 3.1-5.9 4.1-9 2.2-22.6-13.9-57-17.9-83.8-9.8-3.5 1.1-7.1-.9-8.2-4.3-1.1-3.5.9-7.1 4.3-8.2 30.6-9.3 68.5-4.8 94.5 11.1 3.1 1.9 4.1 5.9 2.2 9zm.8-22c-27-16-71.6-17.5-97.4-9.7-4.1 1.2-8.4-1.1-9.6-5.2-1.2-4.1 1.1-8.4 5.2-9.6 29.6-9 78.7-7.2 109.8 11.3 3.7 2.2 4.9 7 2.7 10.7-2.2 3.7-7 4.9-10.7 2.5z"/></svg>
            </span>
            Seus artistas
          </h2>
        </header>
        <div class="h-scroll">
          {#each spotifyStore.topArtists.slice(0, 16) as a (a.id)}
            <a
              class="artist-card"
              href={`/study/music/spotify/artist/${a.id}`}
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
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if spotifyStore.savedTracks.length > 0}
      <section class="block spotify-block">
        <header class="block-head">
          <h2>
            <span class="spotify-mark" aria-hidden="true">
              <svg viewBox="0 0 168 168" width="14" height="14"><circle cx="84" cy="84" r="84" fill="#1db954"/><path fill="#000" d="M119.6 110.6c-1.5 2.5-4.7 3.3-7.2 1.8-19.7-12-44.5-14.7-73.7-8-2.8.6-5.6-1.1-6.3-3.9-.6-2.8 1.1-5.6 3.9-6.3 31.9-7.3 59.4-4.2 81.5 9.2 2.5 1.5 3.3 4.7 1.8 7.2zm9.5-21.2c-1.9 3.1-5.9 4.1-9 2.2-22.6-13.9-57-17.9-83.8-9.8-3.5 1.1-7.1-.9-8.2-4.3-1.1-3.5.9-7.1 4.3-8.2 30.6-9.3 68.5-4.8 94.5 11.1 3.1 1.9 4.1 5.9 2.2 9zm.8-22c-27-16-71.6-17.5-97.4-9.7-4.1 1.2-8.4-1.1-9.6-5.2-1.2-4.1 1.1-8.4 5.2-9.6 29.6-9 78.7-7.2 109.8 11.3 3.7 2.2 4.9 7 2.7 10.7-2.2 3.7-7 4.9-10.7 2.5z"/></svg>
            </span>
            Suas curtidas
          </h2>
        </header>
        <div class="h-scroll">
          {#each spotifyStore.savedTracks.slice(0, 16) as track (track.id)}
            <button
              type="button"
              class="album-card"
              onclick={() => playSpotifyTrack(track, spotifyStore.savedTracks)}
            >
              <div class="album-card-cover">
                {#if spotifyCover(track.album.images)}
                  <img src={spotifyCover(track.album.images)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(track.album.name)}></div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{track.name}</h3>
              <p class="album-card-sub">{track.artists.map((a) => a.name).join(", ")}</p>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if spotifyStore.playlists.length > 0}
      <section class="block spotify-block">
        <header class="block-head">
          <h2>
            <span class="spotify-mark" aria-hidden="true">
              <svg viewBox="0 0 168 168" width="14" height="14"><circle cx="84" cy="84" r="84" fill="#1db954"/><path fill="#000" d="M119.6 110.6c-1.5 2.5-4.7 3.3-7.2 1.8-19.7-12-44.5-14.7-73.7-8-2.8.6-5.6-1.1-6.3-3.9-.6-2.8 1.1-5.6 3.9-6.3 31.9-7.3 59.4-4.2 81.5 9.2 2.5 1.5 3.3 4.7 1.8 7.2zm9.5-21.2c-1.9 3.1-5.9 4.1-9 2.2-22.6-13.9-57-17.9-83.8-9.8-3.5 1.1-7.1-.9-8.2-4.3-1.1-3.5.9-7.1 4.3-8.2 30.6-9.3 68.5-4.8 94.5 11.1 3.1 1.9 4.1 5.9 2.2 9zm.8-22c-27-16-71.6-17.5-97.4-9.7-4.1 1.2-8.4-1.1-9.6-5.2-1.2-4.1 1.1-8.4 5.2-9.6 29.6-9 78.7-7.2 109.8 11.3 3.7 2.2 4.9 7 2.7 10.7-2.2 3.7-7 4.9-10.7 2.5z"/></svg>
            </span>
            Suas playlists Spotify
          </h2>
        </header>
        <div class="h-scroll">
          {#each spotifyStore.playlists as p (p.id)}
            <a
              class="album-card"
              href={`/study/music/spotify/playlist/${p.id}`}
            >
              <div class="album-card-cover">
                {#if spotifyCover(p.images)}
                  <img src={spotifyCover(p.images)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(p.name)}>
                    <svg viewBox="0 0 24 24" width="40%" height="40%" fill="currentColor" aria-hidden="true"><path d="M9 18V5l12-2v13" opacity="0.4"/><circle cx="6" cy="18" r="3" opacity="0.4"/><circle cx="18" cy="16" r="3" opacity="0.4"/></svg>
                  </div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{p.name}</h3>
              <p class="album-card-sub">{p.tracks_total} faixa(s){p.owner_name ? ` · ${p.owner_name}` : ""}</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {/if}

  {#if soundcloudStore.isLoggedIn}
    {#if soundcloudStore.likedTracks.length > 0}
      <section class="block sc-block">
        <header class="block-head">
          <h2><span class="sc-mark">●</span> Suas curtidas no SoundCloud</h2>
        </header>
        <div class="h-scroll">
          {#each soundcloudStore.likedTracks.slice(0, 16) as track (track.id)}
            <button
              type="button"
              class="album-card"
              onclick={() => playSc(track, soundcloudStore.likedTracks)}
            >
              <div class="album-card-cover">
                {#if track.artwork_url || track.user.avatar_url}
                  <img src={soundcloudStore.pickArtwork(track.artwork_url ?? track.user.avatar_url)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(track.title)}></div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{track.title}</h3>
              <p class="album-card-sub">{track.user.username}</p>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if soundcloudStore.streamFeed.length > 0}
      <section class="block sc-block">
        <header class="block-head">
          <h2><span class="sc-mark">●</span> Stream do SoundCloud</h2>
        </header>
        <div class="h-scroll">
          {#each soundcloudStore.streamFeed.slice(0, 16) as track (track.id)}
            <button
              type="button"
              class="album-card"
              onclick={() => playSc(track, soundcloudStore.streamFeed)}
            >
              <div class="album-card-cover">
                {#if track.artwork_url || track.user.avatar_url}
                  <img src={soundcloudStore.pickArtwork(track.artwork_url ?? track.user.avatar_url)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(track.title)}></div>
                {/if}
                <div class="album-card-overlay"></div>
              </div>
              <h3 class="album-card-title">{track.title}</h3>
              <p class="album-card-sub">{track.user.username}</p>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if soundcloudStore.playlists.length > 0}
      <section class="block sc-block">
        <header class="block-head">
          <h2><span class="sc-mark">●</span> Suas playlists SoundCloud</h2>
        </header>
        <div class="h-scroll">
          {#each soundcloudStore.playlists as p (p.id)}
            <a
              class="album-card"
              href={`/study/music/soundcloud/playlist/${p.id}`}
            >
              <div class="album-card-cover">
                {#if p.artwork_url}
                  <img src={soundcloudStore.pickArtwork(p.artwork_url)} alt="" loading="lazy" />
                {:else}
                  <div class="album-card-fallback" style:background={colorFromString(p.title)}></div>
                {/if}
              </div>
              <h3 class="album-card-title">{p.title}</h3>
              <p class="album-card-sub">{p.track_count} faixa(s) · {p.user.username}</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</section>

<style>
  .music-home {
    display: flex;
    flex-direction: column;
    gap: 32px;
    color: rgba(255, 255, 255, 0.95);
  }
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
  .ghost-btn {
    padding: 7px 16px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 999px;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 200ms ease, color 200ms ease;
  }
  .ghost-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }
  .ghost-btn:disabled { opacity: 0.5; cursor: default; }
  .scan-msg {
    margin: -20px 0 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
  }
  .empty-music {
    text-align: center;
    padding: 64px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: rgba(255, 255, 255, 0.85);
  }
  .empty-music h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 800;
  }
  .empty-music p {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 14px;
    max-width: 50ch;
    line-height: 1.5;
  }
  .cta-big {
    margin-top: 8px;
    padding: 12px 28px;
    background: var(--accent);
    color: var(--on-accent, white);
    border: 0;
    border-radius: 999px;
    font-family: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 14px color-mix(in oklab, var(--accent) 30%, transparent);
  }

  .hero {
    position: relative;
    height: 350px;
    border-radius: 24px;
    overflow: hidden;
    margin-bottom: 16px;
  }
  .hero-bg {
    position: absolute;
    inset: 0;
    background-size: cover;
    background-position: center;
    background-color: rgba(40, 40, 40, 0.6);
  }
  .hero-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to right,
      rgba(0, 0, 0, 0.9) 0%,
      rgba(0, 0, 0, 0.4) 50%,
      transparent 100%
    );
  }
  .hero-content {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 32px;
    color: white;
  }
  @media (min-width: 768px) {
    .hero-content { padding: 48px; }
  }
  .hero-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--accent);
    margin-bottom: 12px;
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  }
  .hero-title {
    margin: 0 0 16px;
    font-size: clamp(30px, 4vw, 56px);
    line-height: 1.05;
    font-weight: 900;
    color: white;
    max-width: 36ch;
    overflow-wrap: anywhere;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
  }
  .hero-artist {
    margin: 0 0 32px;
    font-size: 16px;
    color: rgba(255, 255, 255, 0.6);
    font-weight: 500;
    max-width: 30ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .hero-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .hero-play {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 12px 32px;
    background: white;
    color: black;
    border: 0;
    border-radius: 999px;
    font-family: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: transform 200ms ease, background 200ms ease;
  }
  .hero-play:hover {
    background: rgba(255, 255, 255, 0.9);
    transform: scale(1.05);
  }
  .hero-play:active {
    transform: scale(0.95);
  }
  .hero-fav {
    width: 44px;
    height: 44px;
    padding: 0;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 50%;
    color: white;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: background 200ms ease;
  }
  .hero-fav:hover {
    background: rgba(255, 255, 255, 0.2);
  }
  .hero-fav.on {
    color: var(--accent);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .block-head h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: rgba(255, 255, 255, 0.95);
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .spotify-mark {
    display: inline-flex;
    align-items: center;
  }
  .spotify-block .album-card,
  .spotify-block .artist-card {
    text-decoration: none;
  }
  .sc-mark { color: #ff5500; font-size: 14px; }
  .sc-block .album-card { text-decoration: none; }
  .scroll-arrows {
    display: flex;
    gap: 8px;
  }
  .arrow {
    width: 32px;
    height: 32px;
    padding: 0;
    background: rgba(255, 255, 255, 0.05);
    border: 0;
    border-radius: 50%;
    color: white;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: background 200ms ease;
  }
  .arrow:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .album-pills-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 16px;
  }

  .h-scroll {
    display: flex;
    gap: 24px;
    overflow-x: auto;
    overflow-y: visible;
    padding: 8px 4px 24px;
    scroll-behavior: smooth;
    scrollbar-width: none;
  }
  .h-scroll::-webkit-scrollbar { display: none; }

  .artist-card {
    flex: 0 0 144px;
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
    margin: 0 auto 16px;
    transition: transform 700ms ease;
    position: relative;
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
    font-size: 48px;
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

  .album-card {
    flex: 0 0 176px;
    cursor: pointer;
    background: transparent;
    border: 0;
    color: inherit;
    padding: 0;
    text-align: left;
  }
  .album-card-cover {
    position: relative;
    aspect-ratio: 1 / 1;
    border-radius: 16px;
    background: rgba(40, 40, 40, 0.8);
    overflow: hidden;
    margin-bottom: 16px;
    transition: all 300ms ease;
  }
  .album-card-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 500ms ease;
  }
  .album-card:hover .album-card-cover img {
    transform: scale(1.05);
  }
  .album-card-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: white;
  }
  .album-card-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0);
    transition: background 300ms ease;
    border-radius: inherit;
  }
  .album-card:hover .album-card-overlay {
    background: rgba(0, 0, 0, 0.2);
  }
  .album-card-play {
    position: absolute;
    right: 12px;
    bottom: 12px;
    width: 40px;
    height: 40px;
    padding: 0;
    background: white;
    color: black;
    border: 0;
    border-radius: 50%;
    cursor: pointer;
    display: grid;
    place-items: center;
    opacity: 0;
    transform: translateY(8px);
    transition: opacity 300ms ease, transform 300ms ease;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.3);
  }
  .album-card:hover .album-card-play {
    opacity: 1;
    transform: translateY(0);
  }
  .album-card-title {
    margin: 0;
    color: white;
    font-weight: 700;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 4px;
  }
  .album-card-sub {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 4px;
  }

  .muted {
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }

  @media (prefers-reduced-motion: reduce) {
    .hero-play, .hero-fav, .arrow, .album-card-cover img,
    .album-card-overlay, .album-card-play, .artist-circle, .artist-circle img {
      transition: none;
    }
    .hero-play:hover, .artist-card:hover .artist-circle,
    .artist-card:hover .artist-circle img,
    .album-card:hover .album-card-cover img {
      transform: none;
    }
  }
</style>
