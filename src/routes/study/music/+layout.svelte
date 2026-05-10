<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicTheme } from "$lib/study-music/theme-store.svelte";
  import MusicSidebar from "$lib/study-music-components/MusicSidebar.svelte";
  import PlayerBar from "$lib/study-music-components/PlayerBar.svelte";

  let { children } = $props();

  const accentBg = $derived.by(() => {
    if (!musicTheme.useDominant) return null;
    const c = musicPlayer.dominantColor;
    if (!c) return null;
    return {
      strong: `rgb(${c.r}, ${c.g}, ${c.b})`,
      mid: `rgba(${c.r}, ${c.g}, ${c.b}, 0.18)`,
      soft: `rgba(${c.r}, ${c.g}, ${c.b}, 0.08)`,
    };
  });

  const themeAccentOverride = $derived(musicTheme.effectiveAccent);

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
      // Drill-down: se está em subpage de /study/music, volta pra home da aba.
      // Se já está na home (/study/music), exit pra fora do music.
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
  style:--music-accent={accentBg ? accentBg.strong : 'transparent'}
  style:--music-accent-mid={accentBg ? accentBg.mid : 'transparent'}
  style:--music-accent-soft={accentBg ? accentBg.soft : 'transparent'}
  style:--accent={themeAccentOverride ?? null}
>
  <MusicSidebar />
  <main class="music-main">
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
    <div class="music-content">
      {@render children()}
    </div>
  </main>
  <div class="music-player">
    <PlayerBar />
  </div>
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
  .music-content {
    padding: 24px 32px 32px;
    max-width: 1600px;
    margin-inline: auto;
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
</style>
