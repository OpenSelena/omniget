<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { fmtDurationLong, colorFromString } from "$lib/study-music/format";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import AlbumCard from "$lib/study-music-components/AlbumCard.svelte";
  import TrackRow from "$lib/study-music-components/TrackRow.svelte";

  type Album = {
    name: string;
    artist: string | null;
    cover_path: string | null;
    track_count: number;
    total_duration_ms: number;
    year: number | null;
  };

  const artistName = $derived($page.url.searchParams.get("name") ?? "");

  let albums = $state<Album[]>([]);
  let topTracks = $state<MusicTrack[]>([]);
  let loading = $state(true);

  async function load() {
    if (!artistName) {
      loading = false;
      return;
    }
    loading = true;
    try {
      const [albumsRes, tracksRes] = await Promise.allSettled([
        pluginInvoke<{ albums: Album[] }>("study", "study:music:albums:list", {
          artist: artistName,
          limit: 50,
        }),
        pluginInvoke<{ tracks: MusicTrack[] }>("study", "study:music:tracks:list", {
          artist: artistName,
          sort: "most_played",
          limit: 8,
        }),
      ]);
      if (albumsRes.status === "fulfilled") albums = albumsRes.value.albums ?? [];
      if (tracksRes.status === "fulfilled") topTracks = tracksRes.value.tracks ?? [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  $effect(() => {
    void artistName;
    void load();
  });

  function playAll() {
    const all: MusicTrack[] = [...topTracks];
    if (all.length > 0) musicPlayer.play(all[0], all);
  }

  const heroColor = $derived(colorFromString(artistName));
  const totalTracks = $derived(albums.reduce((sum, a) => sum + a.track_count, 0));
</script>

<section class="artist-page">
  {#if !artistName}
    <p class="muted">Artista não especificado.</p>
  {:else if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else}
    <header class="hero" style:--hero-color={heroColor}>
      <div class="initial">{artistName.slice(0, 1).toUpperCase()}</div>
      <div class="meta">
        <span class="eyebrow">{$t("study.music.eyebrow_artist")}</span>
        <h1>{artistName}</h1>
        <div class="info">
          <span>{albums.length} álbum(ns)</span>
          <span class="dot" aria-hidden="true">·</span>
          <span>{totalTracks} faixa(s)</span>
        </div>
        {#if topTracks.length > 0}
          <button type="button" class="play-big" onclick={playAll}>
            <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            <span>{$t("study.music.play")}</span>
          </button>
        {/if}
      </div>
    </header>

    {#if topTracks.length > 0}
      <section class="block">
        <h2>{$t("study.music.top_tracks")}</h2>
        <ul class="track-list">
          {#each topTracks as track, i (track.id)}
            <TrackRow {track} queue={topTracks} index={i} showNumber showCover showAlbum />
          {/each}
        </ul>
      </section>
    {/if}

    {#if albums.length > 0}
      <section class="block">
        <h2>{$t("study.music.discography")}</h2>
        <div class="album-grid">
          {#each albums as album (album.name + (album.artist ?? ""))}
            <AlbumCard {album} href={`/study/music/album/by-name?name=${encodeURIComponent(album.name)}&artist=${encodeURIComponent(artistName)}`} />
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</section>

<style>
  .artist-page { display: flex; flex-direction: column; gap: 24px; }
  .hero {
    display: flex;
    align-items: flex-end;
    gap: 24px;
    padding: 32px 4px 20px;
    border-radius: 12px;
    background: linear-gradient(180deg, var(--hero-color) 0%, transparent 90%);
  }
  .initial {
    width: clamp(120px, 18vw, 200px);
    height: clamp(120px, 18vw, 200px);
    border-radius: 50%;
    background: color-mix(in oklab, var(--hero-color, var(--accent)) 60%, var(--button));
    color: white;
    font-size: clamp(48px, 7vw, 88px);
    font-weight: 800;
    display: grid;
    place-items: center;
    box-shadow: 0 12px 32px color-mix(in oklab, black 30%, transparent);
    flex-shrink: 0;
  }
  .meta { display: flex; flex-direction: column; gap: 12px; min-width: 0; flex: 1; }
  .eyebrow { font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--secondary); }
  .meta h1 { margin: 0; font-size: clamp(28px, 4.5vw, 56px); font-weight: 800; letter-spacing: -0.02em; color: var(--secondary); }
  .info { display: flex; gap: 6px; align-items: center; color: var(--tertiary); font-size: 13px; }
  .info .dot { opacity: 0.6; }
  .play-big {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    background: var(--accent);
    color: var(--on-accent, white);
    border: 0;
    border-radius: 999px;
    font-family: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 14px color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .play-big:hover { transform: scale(1.04); }
  .play-big:active { transform: scale(0.98); }
  .block { display: flex; flex-direction: column; gap: 10px; }
  .block h2 { margin: 0; font-size: 18px; font-weight: 700; color: var(--secondary); }
  .track-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 16px;
  }
  .muted { color: var(--tertiary); font-size: 13px; }
  @media (max-width: 720px) {
    .hero { flex-direction: column; align-items: flex-start; }
  }
</style>
