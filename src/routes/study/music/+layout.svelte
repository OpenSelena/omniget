<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicTheme } from "$lib/study-music/theme-store.svelte";
  import { downloadStore } from "$lib/study-music/download-store.svelte";
  import MusicSidebar from "$lib/study-music-components/MusicSidebar.svelte";
  import PlayerBar from "$lib/study-music-components/PlayerBar.svelte";
  import DownloadsDock from "$lib/study-music-components/DownloadsDock.svelte";

  let { children } = $props();

  onMount(() => {
    downloadStore.start();
  });

  onDestroy(() => {
    downloadStore.stop();
  });

  const isFullscreen = $derived($page.url.pathname.startsWith("/study/music/now-playing"));

  const paletteVars = $derived.by(() => {
    if (!musicTheme.useDominant) return null;
    const palette = musicPlayer.palette;
    const dominant = palette?.[0] ?? musicPlayer.dominantColor;
    if (!dominant) return null;
    const accent = palette?.[1] ?? dominant;
    const highlight = palette?.[2] ?? accent;
    return {
      dominant: `rgb(${dominant.r}, ${dominant.g}, ${dominant.b})`,
      accent: `rgb(${accent.r}, ${accent.g}, ${accent.b})`,
      accentSoft: `rgba(${accent.r}, ${accent.g}, ${accent.b}, 0.18)`,
      accentMid: `rgba(${accent.r}, ${accent.g}, ${accent.b}, 0.28)`,
      highlight: `rgb(${highlight.r}, ${highlight.g}, ${highlight.b})`,
      bgTint: `rgba(${dominant.r}, ${dominant.g}, ${dominant.b}, 0.10)`,
      scrubberGradient: `linear-gradient(to right, rgb(${accent.r}, ${accent.g}, ${accent.b}), rgb(${highlight.r}, ${highlight.g}, ${highlight.b}))`,
    };
  });

  const themeAccentOverride = $derived(musicTheme.effectiveAccent);
  const reduceAnimationsAttr = $derived(musicTheme.reduceAnimationsActive ? "true" : null);

  function exitFullscreen() {
    if (typeof window === "undefined") {
      goto("/study/library");
      return;
    }
    let target = "/study/library";
    try {
      const stored = sessionStorage.getItem("study.music.return_url");
      if (stored && !stored.startsWith("/study/music")) {
        target = stored;
      }
    } catch {
      /* ignore */
    }
    goto(target);
  }

  $effect(() => {
    if (typeof document === "undefined") return;
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }
      if (musicUI.addToPlaylistTrack !== null) return;
      if (musicUI.rightbarTab !== null) return;
      if (musicUI.rootsOpen) return;
      if (musicUI.equalizerOpen) return;
      if (musicUI.lastfmOpen) return;
      if (musicUI.themeOpen) return;
      if (musicUI.discordOpen) return;
      if (musicUI.youtubeOpen) return;
      if (musicUI.contextMenu.open) return;
      if (musicUI.selectedCount() > 0) return;
      e.preventDefault();
      if ($page.url.pathname.startsWith("/study/music/now-playing")) {
        goto("/study/music");
        return;
      }
      if ($page.url.pathname !== "/study/music") {
        goto("/study/music");
      } else {
        exitFullscreen();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });
</script>

<div
  class="music-shell"
  class:fullscreen={isFullscreen}
  data-reduce-animations={reduceAnimationsAttr}
  style:--music-accent={paletteVars?.accent ?? 'transparent'}
  style:--music-accent-mid={paletteVars?.accentMid ?? 'transparent'}
  style:--music-accent-soft={paletteVars?.accentSoft ?? 'transparent'}
  style:--music-highlight={paletteVars?.highlight ?? 'transparent'}
  style:--music-bg-tint={paletteVars?.bgTint ?? 'transparent'}
  style:--music-scrubber-gradient={paletteVars?.scrubberGradient ?? null}
  style:--accent={themeAccentOverride ?? null}
>
  {#if !isFullscreen}
    <MusicSidebar />
  {/if}
  <main class="music-main" class:fullscreen={isFullscreen}>
    {#if !isFullscreen}
      <button
        type="button"
        class="exit-btn"
        onclick={exitFullscreen}
        aria-label={$t("study.music.exit") as string}
        title={$t("study.music.exit") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
        <span class="esc-hint">ESC</span>
      </button>
    {/if}
    <div class="music-content" class:fullscreen={isFullscreen}>
      {@render children()}
    </div>
  </main>
  {#if !isFullscreen}
    <div class="music-player">
      <PlayerBar />
    </div>
  {/if}
  <DownloadsDock />
</div>

<style>
  .music-shell {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    grid-template-columns: 240px 1fr;
    grid-template-rows: 1fr 80px;
    grid-template-areas:
      "side main"
      "player player";
    background:
      radial-gradient(ellipse at 30% 0%, var(--music-accent-mid, transparent) 0%, transparent 60%),
      radial-gradient(ellipse at 80% 100%, var(--music-accent-soft, transparent) 0%, transparent 70%),
      var(--primary);
    color: var(--secondary);
    transition: background 600ms ease;
  }
  .music-shell.fullscreen {
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
    grid-template-areas: "main";
  }
  .music-shell > :global(aside.music-sidebar) {
    grid-area: side;
  }
  .music-main {
    grid-area: main;
    position: relative;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
  }
  .music-main.fullscreen {
    overflow: hidden;
  }
  .music-content {
    padding: 24px 32px 32px;
    max-width: 1600px;
    margin-inline: auto;
  }
  .music-content.fullscreen {
    padding: 0;
    max-width: none;
    margin: 0;
    height: 100%;
  }
  .music-player {
    grid-area: player;
    border-top: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
  }
  .exit-btn {
    position: absolute;
    top: 14px;
    right: 18px;
    z-index: 5;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 999px;
    color: var(--tertiary);
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .exit-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, var(--button));
  }
  .esc-hint {
    font-family: ui-monospace, monospace;
    font-size: 10px;
    padding: 2px 6px;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 4px;
    color: var(--secondary);
  }
  @media (max-width: 760px) {
    .music-shell {
      grid-template-columns: 1fr;
      grid-template-areas:
        "main"
        "player";
    }
    .music-shell > :global(aside.music-sidebar) {
      display: none;
    }
  }
  :global(.music-shell[data-reduce-animations="true"] *),
  :global(.music-shell[data-reduce-animations="true"] *::before),
  :global(.music-shell[data-reduce-animations="true"] *::after) {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
  }
</style>
