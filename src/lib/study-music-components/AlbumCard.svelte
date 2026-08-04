<script lang="ts">
  import { goto } from "$app/navigation";
  import CoverImage from "./CoverImage.svelte";

  type Album = {
    name: string;
    artist?: string | null;
    cover_path?: string | null;
    track_count?: number;
    year?: number | null;
    first_id?: number | null;
  };

  type Props = {
    album: Album;
    href?: string;
  };

  let { album, href }: Props = $props();

  function open() {
    if (href) {
      goto(href);
      return;
    }
    const params = new URLSearchParams({ name: album.name });
    if (album.artist) params.set("artist", album.artist);
    goto(`/study/music/album/by-name?${params.toString()}`);
  }
</script>

<button type="button" class="album-card" onclick={open}>
  <div class="cover-wrap">
    <CoverImage
      src={album.cover_path}
      alt={album.name}
      fallbackSeed={album.name + (album.artist ?? "")}
      rounded="lg"
      trackId={album.first_id ?? null}
    />
    <span class="play-overlay" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
    </span>
  </div>
  <div class="meta">
    <span class="title">{album.name}</span>
    {#if album.artist}
      <span class="sub">{album.artist}</span>
    {/if}
  </div>
</button>

<style>
  .album-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: color-mix(in oklab, var(--button) 50%, transparent);
    border: 0;
    border-radius: 10px;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 150ms ease, transform 150ms ease;
  }
  .album-card:hover {
    background: color-mix(in oklab, var(--accent) 8%, var(--button));
  }
  .album-card:hover .play-overlay {
    opacity: 1;
    transform: translateY(0);
  }
  .cover-wrap {
    position: relative;
    aspect-ratio: 1 / 1;
    width: 100%;
    box-shadow: 0 4px 14px color-mix(in oklab, black 28%, transparent);
    border-radius: 10px;
    overflow: hidden;
  }
  .play-overlay {
    position: absolute;
    bottom: 8px;
    right: 8px;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent, white);
    display: grid;
    place-items: center;
    box-shadow: 0 4px 12px color-mix(in oklab, var(--accent) 40%, transparent);
    opacity: 0;
    transform: translateY(8px);
    transition: opacity 150ms ease, transform 150ms ease;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 2px;
    min-width: 0;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 12px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: reduce) {
    .album-card,
    .play-overlay {
      transition: none;
    }
    .album-card:hover .play-overlay {
      transform: none;
    }
  }
</style>
