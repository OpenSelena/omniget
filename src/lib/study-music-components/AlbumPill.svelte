<script lang="ts">
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import CoverImage from "./CoverImage.svelte";

  type Album = {
    name: string;
    artist?: string | null;
    cover_path?: string | null;
    year?: number | null;
    first_id?: number | null;
  };

  type Props = {
    album: Album;
    href?: string;
  };

  let { album, href }: Props = $props();
  let busy = $state(false);

  function open() {
    if (href) {
      goto(href);
      return;
    }
    const params = new URLSearchParams({ name: album.name });
    if (album.artist) params.set("artist", album.artist);
    goto(`/study/music/album/by-name?${params.toString()}`);
  }

  async function playAlbum(e: MouseEvent) {
    e.stopPropagation();
    if (busy) return;
    busy = true;
    try {
      const args: { name: string; artist?: string } = { name: album.name };
      if (album.artist) args.artist = album.artist;
      const res = await pluginInvoke<{ tracks: MusicTrack[] }>(
        "study",
        "study:music:albums:get",
        args,
      );
      if (res.tracks && res.tracks.length > 0) {
        await musicPlayer.play(res.tracks[0], res.tracks);
      }
    } catch {
      /* ignore */
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="album-pill"
  role="button"
  tabindex="0"
  onclick={open}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      open();
    }
  }}
>
  <div class="cover">
    <CoverImage
      src={album.cover_path}
      alt={album.name}
      fallbackSeed={album.name + (album.artist ?? "")}
      rounded="sm"
      trackId={album.first_id ?? null}
    />
    <div class="cover-overlay" aria-hidden="true"></div>
  </div>
  <div class="meta">
    <h3 class="title">{album.name}</h3>
    {#if album.artist}
      <p class="sub">{album.artist}</p>
    {/if}
  </div>
  <div class="play-wrap">
    <button
      type="button"
      class="play-btn"
      onclick={playAlbum}
      aria-label="Tocar"
      disabled={busy}
    >
      <svg viewBox="0 0 24 24" width="11" height="11" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
    </button>
  </div>
</div>

<style>
  .album-pill {
    display: flex;
    align-items: center;
    gap: 0;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 16px;
    cursor: pointer;
    overflow: hidden;
    padding-right: 16px;
    transition: background 300ms ease, border-color 300ms ease;
  }
  .album-pill:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .album-pill:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .cover {
    position: relative;
    flex-shrink: 0;
    width: 80px;
    height: 80px;
    background: rgba(40, 40, 40, 0.5);
    overflow: hidden;
  }
  .cover :global(.cover) {
    border-radius: 0;
  }
  .cover :global(img) {
    transition: transform 500ms ease;
  }
  .album-pill:hover .cover :global(img) {
    transform: scale(1.05);
  }
  .cover-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0);
    transition: background 300ms ease;
  }
  .album-pill:hover .cover-overlay {
    background: rgba(0, 0, 0, 0.2);
  }
  .meta {
    flex: 1;
    min-width: 0;
    padding: 16px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  .title {
    margin: 0;
    color: rgba(255, 255, 255, 0.95);
    font-weight: 700;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.5);
    font-weight: 600;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .play-wrap {
    opacity: 0;
    transition: opacity 300ms ease;
  }
  .album-pill:hover .play-wrap {
    opacity: 1;
  }
  .play-btn {
    width: 32px;
    height: 32px;
    padding: 0;
    background: rgba(255, 255, 255, 0.1);
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.85);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: background 200ms ease;
  }
  .play-btn:hover {
    background: rgba(255, 255, 255, 0.2);
  }
  @media (max-width: 640px) {
    .cover { width: 64px; height: 64px; }
    .meta { padding: 12px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .album-pill, .cover :global(img), .cover-overlay, .play-wrap, .play-btn {
      transition: none;
    }
  }
</style>
