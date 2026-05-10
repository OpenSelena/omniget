<script lang="ts">
  import { fmtDurationLong } from "$lib/study-music/format";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import { t } from "$lib/i18n";
  import CoverImage from "./CoverImage.svelte";

  type Album = {
    name: string;
    artist?: string | null;
    track_count?: number;
    total_duration_ms?: number;
    year?: number | null;
    cover_path?: string | null;
    first_id?: number | null;
  };

  type Props = {
    album: Album;
    tracks: MusicTrack[];
    eyebrow?: string;
  };

  let { album, tracks, eyebrow }: Props = $props();

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
</script>

<header class="album-hero">
  <div class="cover-block">
    <CoverImage
      src={album.cover_path}
      alt={album.name}
      fallbackSeed={album.name + (album.artist ?? "")}
      rounded="lg"
      trackId={album.first_id ?? (tracks[0]?.id ?? null)}
    />
  </div>
  <div class="meta">
    <span class="eyebrow">{eyebrow ?? $t("study.music.eyebrow_album")}</span>
    <h1 class="title">{album.name}</h1>
    <div class="info">
      {#if album.artist}
        <span class="artist">{album.artist}</span>
        <span class="dot" aria-hidden="true">·</span>
      {/if}
      {#if album.year}
        <span>{album.year}</span>
        <span class="dot" aria-hidden="true">·</span>
      {/if}
      <span>{$t("study.music.tracks_count", { count: album.track_count ?? tracks.length })}</span>
      {#if album.total_duration_ms}
        <span class="dot" aria-hidden="true">·</span>
        <span>{fmtDurationLong(album.total_duration_ms)}</span>
      {/if}
    </div>
    <div class="actions">
      <button type="button" class="play-big" onclick={playAll}>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
        <span>{$t("study.music.play")}</span>
      </button>
      <button
        type="button"
        class="ghost-big"
        onclick={playShuffled}
        title={$t("study.music.shuffle") as string}
        aria-label={$t("study.music.shuffle") as string}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <polyline points="16 3 21 3 21 8"/>
          <line x1="4" y1="20" x2="21" y2="3"/>
          <polyline points="21 16 21 21 16 21"/>
          <line x1="15" y1="15" x2="21" y2="21"/>
          <line x1="4" y1="4" x2="9" y2="9"/>
        </svg>
      </button>
    </div>
  </div>
</header>

<style>
  .album-hero {
    display: flex;
    align-items: flex-end;
    gap: 28px;
    padding: 24px 4px 28px;
  }
  .cover-block {
    flex-shrink: 0;
    width: clamp(160px, 22vw, 240px);
    aspect-ratio: 1 / 1;
    box-shadow: 0 12px 40px color-mix(in oklab, black 35%, transparent);
    border-radius: 10px;
    overflow: hidden;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .eyebrow {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--tertiary);
  }
  .title {
    margin: 0;
    font-size: clamp(28px, 4.5vw, 56px);
    line-height: 1.05;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--secondary);
    overflow-wrap: anywhere;
  }
  .info {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    color: var(--tertiary);
    font-size: 13px;
  }
  .info .artist {
    color: var(--secondary);
    font-weight: 600;
  }
  .info .dot {
    opacity: 0.6;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 6px;
  }
  .play-big {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 14px color-mix(in oklab, var(--accent) 35%, transparent);
    transition: transform 120ms ease, background 120ms ease;
  }
  .play-big:hover {
    transform: scale(1.04);
  }
  .play-big:active {
    transform: scale(0.98);
  }
  .ghost-big {
    width: 40px;
    height: 40px;
    padding: 0;
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 50%;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .ghost-big:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  @media (max-width: 720px) {
    .album-hero {
      flex-direction: column;
      align-items: flex-start;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .play-big, .ghost-big { transition: none; }
    .play-big:hover { transform: none; }
  }
</style>
