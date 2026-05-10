<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import type { MusicTrack } from "$lib/study-music/player-store.svelte";

  type Album = {
    name: string;
    artist: string | null;
    cover_path: string | null;
    track_count: number;
    year: number | null;
  };

  type Props = {
    album: Album | null;
  };

  let { album }: Props = $props();

  const coverUrl = $derived.by(() => {
    if (!album?.cover_path) return null;
    try {
      return convertFileSrc(album.cover_path);
    } catch {
      return album.cover_path;
    }
  });

  let busy = $state(false);

  async function startListening() {
    if (!album || busy) return;
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

  function openAlbum() {
    if (!album) return;
    const params = new URLSearchParams({ name: album.name });
    if (album.artist) params.set("artist", album.artist);
    goto(`/study/music/album/by-name?${params.toString()}`);
  }
</script>

{#if album}
  <section
    class="hero"
    style:background-image={coverUrl ? `url('${coverUrl}')` : "none"}
  >
    <div class="overlay">
      <div class="content">
        <span class="eyebrow">
          <svg viewBox="0 0 24 24" width="11" height="11" fill="currentColor" aria-hidden="true">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
          </svg>
          {$t("study.music.featured_album")}
        </span>
        <h1 class="title">{album.name}</h1>
        {#if album.artist}
          <p class="artist">{$t("study.music.by")} {album.artist}</p>
        {/if}
        <div class="actions">
          <button
            type="button"
            class="play-cta"
            onclick={startListening}
            disabled={busy}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            <span>{$t("study.music.start_listening")}</span>
          </button>
          <button
            type="button"
            class="open-cta"
            onclick={openAlbum}
            aria-label={$t("study.music.open_album") as string}
            title={$t("study.music.open_album") as string}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <line x1="5" y1="12" x2="19" y2="12"/>
              <polyline points="12 5 19 12 12 19"/>
            </svg>
          </button>
        </div>
      </div>
    </div>
  </section>
{/if}

<style>
  .hero {
    position: relative;
    width: 100%;
    aspect-ratio: 21 / 9;
    max-height: 380px;
    border-radius: 12px;
    overflow: hidden;
    background-size: cover;
    background-position: center 30%;
    background-color: color-mix(in oklab, var(--accent) 8%, var(--button));
    box-shadow: 0 8px 24px color-mix(in oklab, black 28%, transparent);
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, black 70%, transparent) 0%,
      color-mix(in oklab, black 35%, transparent) 50%,
      transparent 80%
    );
    padding: 32px 36px;
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 60%;
    color: white;
  }
  .eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  }
  .title {
    margin: 0;
    font-size: clamp(36px, 6vw, 80px);
    line-height: 0.95;
    font-weight: 900;
    letter-spacing: -0.02em;
    color: white;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
    overflow-wrap: anywhere;
  }
  .artist {
    margin: 0;
    font-size: 14px;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
  }
  .play-cta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    background: var(--accent);
    color: var(--on-accent, white);
    border: 0;
    border-radius: 999px;
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 14px color-mix(in oklab, var(--accent) 40%, transparent);
    transition: transform 120ms ease;
  }
  .play-cta:hover { transform: scale(1.04); }
  .play-cta:disabled { opacity: 0.6; cursor: default; }
  .open-cta {
    width: 38px;
    height: 38px;
    padding: 0;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 50%;
    color: white;
    cursor: pointer;
    display: grid;
    place-items: center;
    backdrop-filter: blur(4px);
    transition: background 120ms ease;
  }
  .open-cta:hover {
    background: rgba(255, 255, 255, 0.22);
  }
  @media (max-width: 800px) {
    .overlay { padding: 18px; }
    .content { max-width: 100%; }
  }
</style>
