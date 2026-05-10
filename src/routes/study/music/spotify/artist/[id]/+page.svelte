<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    spotifyStore,
    type SpotifyArtist,
    type SpotifyTrack,
    type SpotifySavedAlbum,
  } from "$lib/study-music/spotify-store.svelte";
  import { colorFromString } from "$lib/study-music/format";

  let artist = $state<SpotifyArtist | null>(null);
  let topTracks = $state<SpotifyTrack[]>([]);
  let albums = $state<SpotifySavedAlbum[]>([]);
  let isFollowing = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let followToggle = $state(false);

  const artistId = $derived($page.params.id ?? "");

  onMount(async () => {
    if (!artistId) {
      goto("/study/music/spotify");
      return;
    }
    if (!spotifyStore.status.logged_in) {
      goto("/study/music/spotify");
      return;
    }
    try {
      const [info, top, albs, follow] = await Promise.allSettled([
        spotifyStore.getArtist(artistId),
        spotifyStore.getArtistTopTracks(artistId),
        spotifyStore.getArtistAlbums(artistId),
        spotifyStore.checkFollowArtists([artistId]),
      ]);

      if (info.status === "fulfilled" && info.value) {
        const a: any = info.value;
        artist = {
          id: a.id,
          name: a.name ?? "",
          uri: a.uri ?? "",
          images: a.images ?? [],
          genres: a.genres ?? [],
          followers: a.followers?.total,
          popularity: a.popularity,
        };
      } else {
        const fromCache = spotifyStore.allArtists.find((x) => x.id === artistId);
        if (fromCache) {
          artist = fromCache;
        }
      }
      if (top.status === "fulfilled") topTracks = top.value;
      if (albs.status === "fulfilled") albums = albs.value;
      if (follow.status === "fulfilled") isFollowing = follow.value[0] === true;

      const failed = [info, top, albs, follow].filter(
        (x) => x.status === "rejected",
      );
      if (
        failed.length > 0 &&
        topTracks.length === 0 &&
        albums.length === 0
      ) {
        const reason = (failed[0] as PromiseRejectedResult).reason;
        const msg = reason instanceof Error ? reason.message : String(reason);
        if (msg.includes("403") || msg.toLowerCase().includes("forbidden")) {
          error =
            "O Spotify bloqueou o acesso a este artista para apps em Development mode. Pode tocar via clicar em uma faixa dele em outra tela.";
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  async function playFromIndex(idx: number) {
    if (topTracks.length === 0) return;
    const reordered = [...topTracks.slice(idx), ...topTracks.slice(0, idx)];
    try {
      const mode = await spotifyStore.playTrack(reordered[0], reordered);
      if (mode === "youtube") {
        showToast("info", "Tocando via YouTube (modo Free)");
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function toggleFollow() {
    if (followToggle) return;
    followToggle = true;
    try {
      isFollowing = await spotifyStore.toggleFollowArtist(artistId);
      showToast("success", isFollowing ? "Seguindo" : "Deixou de seguir");
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      followToggle = false;
    }
  }

  function fmtDuration(ms: number): string {
    const total = Math.floor(ms / 1000);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

<section class="artist-page">
  <button type="button" class="back-btn" onclick={() => history.back()}>
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <polyline points="15 18 9 12 15 6"/>
    </svg>
    Voltar
  </button>

  {#if artist}
    <header class="hero">
      <div class="hero-circle">
        {#if spotifyStore.pickImage(artist.images, 400)}
          <img src={spotifyStore.pickImage(artist.images, 400)} alt="" />
        {:else}
          <div class="hero-fallback" style:background={colorFromString(artist.name)}>
            {artist.name.slice(0, 1).toUpperCase()}
          </div>
        {/if}
      </div>
      <div class="hero-info">
        <span class="eyebrow">Artista</span>
        <h1>{artist.name}</h1>
        {#if artist.followers}
          <p class="meta">{artist.followers.toLocaleString("pt-BR")} seguidores</p>
        {/if}
        {#if artist.genres.length > 0}
          <p class="genres">{artist.genres.slice(0, 3).join(" · ")}</p>
        {/if}
        <div class="actions">
          <button type="button" class="play-btn" onclick={() => playFromIndex(0)} disabled={topTracks.length === 0}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            Tocar
          </button>
          <button
            type="button"
            class="follow-btn"
            class:on={isFollowing}
            onclick={toggleFollow}
            disabled={followToggle}
          >
            {isFollowing ? "Seguindo" : "Seguir"}
          </button>
        </div>
      </div>
    </header>
  {/if}

  {#if loading}
    <p class="muted">Carregando…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else}
    {#if topTracks.length > 0}
      <section class="block">
        <header class="block-head"><h2>Populares</h2></header>
        <div class="track-list">
          {#each topTracks.slice(0, 10) as track, i (track.id + i)}
            <button type="button" class="track-row" onclick={() => playFromIndex(i)}>
              <span class="track-num">{i + 1}</span>
              <div class="track-cover">
                {#if spotifyStore.pickImage(track.album.images, 80)}
                  <img src={spotifyStore.pickImage(track.album.images, 80)} alt="" loading="lazy" />
                {/if}
              </div>
              <div class="track-meta">
                <span class="track-title">{track.name}</span>
                <span class="track-album">{track.album.name}</span>
              </div>
              <span class="track-dur">{fmtDuration(track.duration_ms)}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if albums.length > 0}
      <section class="block">
        <header class="block-head"><h2>Discografia</h2></header>
        <div class="album-grid">
          {#each albums as a (a.id)}
            <a class="album-card" href={`/study/music/spotify/album/${a.id}`}>
              <div class="album-cover">
                {#if spotifyStore.pickImage(a.images, 300)}
                  <img src={spotifyStore.pickImage(a.images, 300)} alt="" loading="lazy" />
                {:else}
                  <div class="album-fallback" style:background={colorFromString(a.name)}></div>
                {/if}
              </div>
              <h3 class="album-title">{a.name}</h3>
              <p class="album-sub">{a.release_date ? a.release_date.slice(0, 4) : ""}{a.total_tracks ? ` · ${a.total_tracks} faixas` : ""}</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</section>

<style>
  .artist-page { display: flex; flex-direction: column; gap: 24px; color: rgba(255, 255, 255, 0.95); }
  .back-btn {
    align-self: flex-start; display: inline-flex; align-items: center; gap: 8px;
    padding: 6px 12px 6px 8px; background: rgba(255,255,255,0.05); border: 0;
    border-radius: 999px; color: rgba(255,255,255,0.85);
    font-family: inherit; font-size: 12px; font-weight: 600; cursor: pointer;
  }
  .back-btn:hover { background: rgba(255,255,255,0.1); }

  .hero { display: flex; gap: 32px; align-items: flex-end; padding: 24px 0; }
  .hero-circle { width: 220px; height: 220px; border-radius: 50%; overflow: hidden; flex-shrink: 0; box-shadow: 0 4px 60px rgba(0,0,0,0.5); }
  .hero-circle img { width: 100%; height: 100%; object-fit: cover; }
  .hero-fallback { width: 100%; height: 100%; display: grid; place-items: center; color: white; font-size: 64px; font-weight: 800; }
  .hero-info { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .eyebrow { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: rgba(255,255,255,0.6); }
  .hero-info h1 { margin: 0; font-size: clamp(28px, 4vw, 56px); font-weight: 900; letter-spacing: -0.02em; line-height: 1.05; }
  .meta, .genres { margin: 0; color: rgba(255,255,255,0.5); font-size: 13px; }
  .actions { display: flex; align-items: center; gap: 12px; margin-top: 8px; }
  .play-btn { display: inline-flex; align-items: center; gap: 8px; padding: 12px 32px; background: #1db954; color: #000; border: 0; border-radius: 999px; font-family: inherit; font-size: 14px; font-weight: 700; cursor: pointer; transition: transform 200ms ease, background 200ms ease; }
  .play-btn:hover { background: #1ed760; transform: scale(1.04); }
  .play-btn:disabled { opacity: 0.5; cursor: default; transform: none; }
  .follow-btn { padding: 10px 20px; background: transparent; border: 1px solid rgba(255,255,255,0.4); border-radius: 999px; color: white; font-family: inherit; font-size: 13px; font-weight: 700; cursor: pointer; transition: border-color 200ms ease, background 200ms ease; }
  .follow-btn:hover { border-color: white; }
  .follow-btn.on { background: rgba(29,185,84,0.15); border-color: #1db954; color: #1db954; }
  .follow-btn:disabled { opacity: 0.5; cursor: default; }

  .block { display: flex; flex-direction: column; gap: 16px; }
  .block-head h2 { margin: 0; font-size: 22px; font-weight: 800; }

  .track-list { display: flex; flex-direction: column; gap: 2px; }
  .track-row { display: grid; grid-template-columns: 40px 56px 1fr 60px; gap: 12px; align-items: center; padding: 8px 12px; background: transparent; border: 0; border-radius: 6px; color: inherit; cursor: pointer; text-align: left; transition: background 200ms ease; }
  .track-row:hover { background: rgba(255,255,255,0.06); }
  .track-num { color: rgba(255,255,255,0.5); font-size: 13px; text-align: center; }
  .track-cover { width: 44px; height: 44px; border-radius: 4px; overflow: hidden; background: rgba(40,40,40,0.8); }
  .track-cover img { width: 100%; height: 100%; object-fit: cover; }
  .track-meta { display: flex; flex-direction: column; min-width: 0; }
  .track-title { font-size: 14px; font-weight: 600; color: white; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track-album { font-size: 12px; color: rgba(255,255,255,0.55); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track-dur { font-size: 13px; color: rgba(255,255,255,0.55); text-align: right; font-variant-numeric: tabular-nums; }

  .album-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 24px; }
  .album-card { background: transparent; border: 0; padding: 0; cursor: pointer; color: inherit; text-align: left; text-decoration: none; }
  .album-cover { aspect-ratio: 1/1; border-radius: 12px; overflow: hidden; margin-bottom: 12px; background: rgba(40,40,40,0.8); transition: transform 300ms ease; }
  .album-cover img { width: 100%; height: 100%; object-fit: cover; transition: transform 500ms ease; }
  .album-card:hover .album-cover img { transform: scale(1.05); }
  .album-fallback { width: 100%; height: 100%; }
  .album-title { margin: 0; font-size: 14px; font-weight: 700; color: white; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0 4px; }
  .album-sub { margin: 4px 0 0; font-size: 12px; color: rgba(255,255,255,0.5); padding: 0 4px; }

  .muted { color: rgba(255,255,255,0.5); font-size: 13px; }
  .error { color: #e22134; font-size: 13px; }
  @media (prefers-reduced-motion: reduce) { .play-btn { transition: none; } .play-btn:hover { transform: none; } .album-card:hover .album-cover img { transform: none; } }
</style>
