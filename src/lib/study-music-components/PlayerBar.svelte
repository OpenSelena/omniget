<script lang="ts">
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { t } from "$lib/i18n";
  import CoverImage from "./CoverImage.svelte";
  import ProgressBar from "./ProgressBar.svelte";
  import VolumeControl from "./VolumeControl.svelte";

  const t_ = $derived(musicPlayer.currentTrack);

  function favoriteToggle() {
    if (!t_) return;
    void musicPlayer.toggleFavorite(t_.id);
  }
</script>

{#if t_}
  <div class="player-bar" role="region" aria-label={$t("study.music.player_aria")}>
    <div class="left">
      <div class="cover-mini">
        <CoverImage
          src={t_.cover_path}
          alt={t_.title ?? t_.path}
          fallbackSeed={t_.album ?? t_.title}
          rounded="md"
          trackId={t_.id}
        />
      </div>
      <div class="info">
        <span class="title">{t_.title ?? t_.path}</span>
        {#if t_.artist}
          <span class="artist">{t_.artist}</span>
        {/if}
      </div>
      <button
        type="button"
        class="fav"
        class:on={t_.favorite}
        onclick={favoriteToggle}
        aria-label={t_.favorite
          ? ($t("study.music.unfavorite") as string)
          : ($t("study.music.favorite") as string)}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill={t_.favorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
        </svg>
      </button>
    </div>

    <div class="center">
      <div class="controls">
        <button
          type="button"
          class="ctrl-btn"
          class:active={musicPlayer.shuffle}
          onclick={() => musicPlayer.toggleShuffle()}
          aria-label={$t("study.music.shuffle") as string}
          title={$t("study.music.shuffle") as string}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="16 3 21 3 21 8"/>
            <line x1="4" y1="20" x2="21" y2="3"/>
            <polyline points="21 16 21 21 16 21"/>
            <line x1="15" y1="15" x2="21" y2="21"/>
            <line x1="4" y1="4" x2="9" y2="9"/>
          </svg>
        </button>
        <button
          type="button"
          class="ctrl-btn"
          onclick={() => musicPlayer.prev()}
          aria-label={$t("study.music.prev") as string}
          title={$t("study.music.prev") as string}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M19 20L9 12l10-8v16zM5 4h2v16H5z"/></svg>
        </button>
        <button
          type="button"
          class="play-btn"
          onclick={() => musicPlayer.togglePlay()}
          aria-label={musicPlayer.paused
            ? ($t("study.music.play") as string)
            : ($t("study.music.pause") as string)}
        >
          {#if musicPlayer.loading}
            <span class="spinner" aria-hidden="true"></span>
          {:else if musicPlayer.paused}
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
          {:else}
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><rect x="6" y="5" width="4" height="14"/><rect x="14" y="5" width="4" height="14"/></svg>
          {/if}
        </button>
        <button
          type="button"
          class="ctrl-btn"
          onclick={() => musicPlayer.next()}
          aria-label={$t("study.music.next") as string}
          title={$t("study.music.next") as string}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M5 4l10 8-10 8V4zm12 0h2v16h-2z"/></svg>
        </button>
        <button
          type="button"
          class="ctrl-btn"
          class:active={musicPlayer.repeat !== "off"}
          onclick={() => musicPlayer.cycleRepeat()}
          aria-label={$t("study.music.repeat") as string}
          title={$t("study.music.repeat") as string}
        >
          {#if musicPlayer.repeat === "one"}
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="17 1 21 5 17 9"/>
              <path d="M3 11V9a4 4 0 0 1 4-4h14"/>
              <polyline points="7 23 3 19 7 15"/>
              <path d="M21 13v2a4 4 0 0 1-4 4H3"/>
              <text x="11" y="14" font-size="9" font-weight="bold" fill="currentColor" stroke="none">1</text>
            </svg>
          {:else}
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="17 1 21 5 17 9"/>
              <path d="M3 11V9a4 4 0 0 1 4-4h14"/>
              <polyline points="7 23 3 19 7 15"/>
              <path d="M21 13v2a4 4 0 0 1-4 4H3"/>
            </svg>
          {/if}
        </button>
      </div>
      <ProgressBar
        current={musicPlayer.currentTime}
        duration={musicPlayer.duration}
        onSeek={(s) => musicPlayer.seek(s)}
      />
    </div>

    <div class="right">
      <button
        type="button"
        class="ctrl-btn"
        class:active={musicUI.rightbarTab === "info"}
        onclick={() => musicUI.toggleInfo()}
        aria-label={$t("study.music.tab_info") as string}
        title={$t("study.music.tab_info") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="9"/>
          <line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
      </button>
      <button
        type="button"
        class="ctrl-btn"
        class:active={musicUI.lyricsOpen}
        onclick={() => musicUI.toggleLyrics()}
        aria-label={$t("study.music.lyrics_title") as string}
        title={$t("study.music.lyrics_title") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M9 18V5l12-2v13"/>
          <circle cx="6" cy="18" r="3"/>
          <circle cx="18" cy="16" r="3"/>
          <line x1="3" y1="9" x2="9" y2="9"/>
          <line x1="3" y1="13" x2="7" y2="13"/>
        </svg>
      </button>
      <button
        type="button"
        class="ctrl-btn"
        class:active={musicPlayer.eqEnabled}
        onclick={() => musicUI.toggleEqualizer()}
        aria-label={$t("study.music.eq_title") as string}
        title={$t("study.music.eq_title") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <line x1="4" y1="21" x2="4" y2="14"/>
          <line x1="4" y1="10" x2="4" y2="3"/>
          <line x1="12" y1="21" x2="12" y2="12"/>
          <line x1="12" y1="8" x2="12" y2="3"/>
          <line x1="20" y1="21" x2="20" y2="16"/>
          <line x1="20" y1="12" x2="20" y2="3"/>
          <line x1="1" y1="14" x2="7" y2="14"/>
          <line x1="9" y1="8" x2="15" y2="8"/>
          <line x1="17" y1="16" x2="23" y2="16"/>
        </svg>
      </button>
      <button
        type="button"
        class="ctrl-btn"
        class:active={musicUI.queueOpen}
        onclick={() => musicUI.toggleQueue()}
        aria-label={$t("study.music.queue_toggle") as string}
        title={$t("study.music.queue_toggle") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <line x1="8" y1="6" x2="21" y2="6"/>
          <line x1="8" y1="12" x2="21" y2="12"/>
          <line x1="8" y1="18" x2="21" y2="18"/>
          <line x1="3" y1="6" x2="3.01" y2="6"/>
          <line x1="3" y1="12" x2="3.01" y2="12"/>
          <line x1="3" y1="18" x2="3.01" y2="18"/>
        </svg>
      </button>
      <VolumeControl />
    </div>
  </div>
{/if}

<style>
  .player-bar {
    display: grid;
    grid-template-columns: minmax(0, 28%) minmax(0, 44%) minmax(0, 28%);
    align-items: center;
    gap: 16px;
    width: 100%;
    height: 80px;
    padding: 0 18px;
    background: color-mix(in oklab, var(--primary) 90%, var(--button-elevated));
    border-top: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
  .left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .cover-mini {
    flex-shrink: 0;
    width: 56px;
    height: 56px;
    box-shadow: 0 2px 8px color-mix(in oklab, black 20%, transparent);
    border-radius: 6px;
    overflow: hidden;
  }
  .info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    flex: 1;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 11px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fav {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--tertiary);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: color 120ms ease, background 120ms ease;
  }
  .fav:hover { color: var(--secondary); background: color-mix(in oklab, var(--accent) 8%, transparent); }
  .fav.on { color: var(--accent); }

  .center {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
  }
  .ctrl-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--tertiary);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: color 120ms ease, background 120ms ease, transform 100ms ease;
  }
  .ctrl-btn:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .ctrl-btn.active {
    color: var(--accent);
  }
  .ctrl-btn:active {
    transform: scale(0.92);
  }
  .play-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--secondary);
    color: var(--primary);
    cursor: pointer;
    display: grid;
    place-items: center;
    box-shadow: 0 2px 8px color-mix(in oklab, var(--secondary) 20%, transparent);
    transition: transform 100ms ease, background 120ms ease;
  }
  .play-btn:hover {
    transform: scale(1.05);
    background: var(--accent);
    color: var(--on-accent, white);
  }
  .play-btn:active {
    transform: scale(0.96);
  }
  .spinner {
    display: block;
    width: 16px;
    height: 16px;
    border: 2px solid color-mix(in oklab, currentColor 30%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
  }

  @media (max-width: 760px) {
    .player-bar {
      grid-template-columns: 1fr auto;
      height: auto;
      padding: 8px 12px;
    }
    .right { display: none; }
  }
  @media (prefers-reduced-motion: reduce) {
    .play-btn, .ctrl-btn, .fav, .spinner {
      transition: none;
      animation: none;
    }
  }
</style>
