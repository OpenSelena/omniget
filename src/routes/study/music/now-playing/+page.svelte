<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount, tick } from "svelte";
  import { t } from "$lib/i18n";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicTheme } from "$lib/study-music/theme-store.svelte";
  import { lyricsStore } from "$lib/study-music/lyrics-store.svelte";
  import { fmtDuration } from "$lib/study-music/format";
  import CoverImage from "$lib/study-music-components/CoverImage.svelte";
  import ProgressBar from "$lib/study-music-components/ProgressBar.svelte";
  import VolumeControl from "$lib/study-music-components/VolumeControl.svelte";
  import PlayerErrorBanner from "$lib/study-music-components/PlayerErrorBanner.svelte";

  type Tab = "lyrics" | "queue" | "previous";
  let tab = $state<Tab>("lyrics");

  const track = $derived(musicPlayer.currentTrack);
  const trackId = $derived(track?.id ?? null);
  const lyricsLines = $derived(lyricsStore.lines);
  const lyricsStatus = $derived(lyricsStore.status);
  const activeLyricIdx = $derived(
    lyricsStatus === "synced" && lyricsLines.length > 0
      ? lyricsStore.activeIndex(musicPlayer.currentTime)
      : -1,
  );
  const previousTracks = $derived(
    musicPlayer.queue.slice(0, Math.max(0, musicPlayer.queueIndex)).reverse(),
  );
  const upNext = $derived(musicPlayer.queue.slice(musicPlayer.queueIndex + 1));

  const reduceMotion = $derived(musicTheme.reduceAnimationsActive);

  let lyricsContainer = $state<HTMLDivElement | null>(null);
  let lineRefs: HTMLLIElement[] = $state([]);
  let _lastScrollAt = 0;

  $effect(() => {
    if (trackId === null) {
      lyricsStore.reset();
      return;
    }
    void lyricsStore.loadFor(trackId);
  });

  $effect(() => {
    if (tab !== "lyrics") return;
    void activeLyricIdx;
    if (activeLyricIdx < 0) return;
    const now = performance.now();
    if (now - _lastScrollAt < 200) return;
    _lastScrollAt = now;
    void scrollToActive();
  });

  async function scrollToActive() {
    await tick();
    const el = lineRefs[activeLyricIdx];
    if (!el || !lyricsContainer) return;
    const cRect = lyricsContainer.getBoundingClientRect();
    const offset =
      el.offsetTop - lyricsContainer.offsetTop - cRect.height / 2 + el.offsetHeight / 2;
    lyricsContainer.scrollTo({
      top: offset,
      behavior: reduceMotion ? "instant" : "smooth",
    });
  }

  function close() {
    if (typeof window !== "undefined" && window.history.length > 1) {
      history.back();
    } else {
      goto("/study/music");
    }
  }

  function jumpTo(absoluteIdx: number) {
    if (absoluteIdx < 0 || absoluteIdx >= musicPlayer.queue.length) return;
    musicPlayer.queueIndex = absoluteIdx;
    void musicPlayer.play(musicPlayer.queue[absoluteIdx], musicPlayer.queue);
  }

  onMount(() => {
    if (!track) {
      goto("/study/music");
    }
  });

  function favoriteToggle() {
    if (!track) return;
    void musicPlayer.toggleFavorite(track.id);
  }

  const coverSrc = $derived(
    track?.cover_path ??
      track?.spotify_cover_url ??
      track?.youtube_thumbnail ??
      track?.soundcloud_artwork_url ??
      null,
  );
</script>

<svelte:head>
  <title>{track?.title ?? $t("study.music.now_playing_title")} · omniget</title>
</svelte:head>

{#if track}
  <div class="np-shell">
    <PlayerErrorBanner />
    <div class="np-backdrop" aria-hidden="true">
      {#if coverSrc}
        <CoverImage
          src={coverSrc}
          alt=""
          fallbackSeed={track.album ?? track.title}
          rounded="md"
          trackId={track.id}
        />
      {/if}
    </div>
    <div class="np-veil" aria-hidden="true"></div>

    <button
      type="button"
      class="np-close"
      onclick={close}
      aria-label={$t("study.music.now_playing_close") as string}
      title={$t("study.music.now_playing_close") as string}
    >
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <polyline points="6 9 12 15 18 9"/>
      </svg>
      <span>{$t("study.music.now_playing_close")}</span>
    </button>

    <div class="np-grid">
      <section class="np-left">
        <div class="np-cover-wrap">
          <CoverImage
            src={coverSrc}
            alt={track.title ?? track.path}
            fallbackSeed={track.album ?? track.title}
            rounded="lg"
            trackId={track.id}
          />
        </div>
        <div class="np-meta">
          <h1 class="np-title">{track.title ?? track.path}</h1>
          {#if track.artist}
            <p class="np-artist">{track.artist}</p>
          {/if}
          {#if track.album}
            <p class="np-album">{track.album}</p>
          {/if}
        </div>
        <ProgressBar
          current={musicPlayer.currentTime}
          duration={musicPlayer.duration}
          onSeek={(s) => musicPlayer.seek(s)}
        />
        <div class="np-controls">
          <button
            type="button"
            class="np-fav"
            class:on={track.favorite}
            onclick={favoriteToggle}
            aria-label={track.favorite
              ? ($t("study.music.unfavorite") as string)
              : ($t("study.music.favorite") as string)}
          >
            <svg viewBox="0 0 24 24" width="18" height="18" fill={track.favorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
            </svg>
          </button>
          <button
            type="button"
            class="np-btn"
            class:active={musicPlayer.shuffle}
            onclick={() => musicPlayer.toggleShuffle()}
            aria-label={$t("study.music.shuffle") as string}
            title={$t("study.music.shuffle") as string}
          >
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="16 3 21 3 21 8"/>
              <line x1="4" y1="20" x2="21" y2="3"/>
              <polyline points="21 16 21 21 16 21"/>
              <line x1="15" y1="15" x2="21" y2="21"/>
              <line x1="4" y1="4" x2="9" y2="9"/>
            </svg>
          </button>
          <button
            type="button"
            class="np-btn"
            onclick={() => musicPlayer.prev()}
            aria-label={$t("study.music.prev") as string}
          >
            <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M19 20L9 12l10-8v16zM5 4h2v16H5z"/></svg>
          </button>
          <button
            type="button"
            class="np-play"
            onclick={() => musicPlayer.togglePlay()}
            aria-label={musicPlayer.paused
              ? ($t("study.music.play") as string)
              : ($t("study.music.pause") as string)}
          >
            {#if musicPlayer.loading}
              <span class="np-spinner" aria-hidden="true"></span>
            {:else if musicPlayer.paused}
              <svg viewBox="0 0 24 24" width="28" height="28" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            {:else}
              <svg viewBox="0 0 24 24" width="28" height="28" fill="currentColor" aria-hidden="true"><rect x="6" y="5" width="4" height="14"/><rect x="14" y="5" width="4" height="14"/></svg>
            {/if}
          </button>
          <button
            type="button"
            class="np-btn"
            onclick={() => musicPlayer.next()}
            aria-label={$t("study.music.next") as string}
          >
            <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M5 4l10 8-10 8V4zm12 0h2v16h-2z"/></svg>
          </button>
          <button
            type="button"
            class="np-btn"
            class:active={musicPlayer.repeat !== "off"}
            onclick={() => musicPlayer.cycleRepeat()}
            aria-label={$t("study.music.repeat") as string}
            title={$t("study.music.repeat") as string}
          >
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="17 1 21 5 17 9"/>
              <path d="M3 11V9a4 4 0 0 1 4-4h14"/>
              <polyline points="7 23 3 19 7 15"/>
              <path d="M21 13v2a4 4 0 0 1-4 4H3"/>
              {#if musicPlayer.repeat === "one"}
                <text x="11" y="14" font-size="9" font-weight="bold" fill="currentColor" stroke="none">1</text>
              {/if}
            </svg>
          </button>
          <div class="np-volume">
            <VolumeControl />
          </div>
        </div>
      </section>

      <section class="np-right">
        <div class="np-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab === "lyrics"}
            class="np-tab"
            class:active={tab === "lyrics"}
            onclick={() => (tab = "lyrics")}
          >{$t("study.music.now_playing_tab_lyrics")}</button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "queue"}
            class="np-tab"
            class:active={tab === "queue"}
            onclick={() => (tab = "queue")}
          >{$t("study.music.now_playing_tab_queue")} <span class="np-tab-count">{upNext.length}</span></button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "previous"}
            class="np-tab"
            class:active={tab === "previous"}
            onclick={() => (tab = "previous")}
          >{$t("study.music.now_playing_tab_previous")} <span class="np-tab-count">{previousTracks.length}</span></button>
        </div>

        <div class="np-tab-body">
          {#if tab === "lyrics"}
            {#if lyricsStatus === "loading"}
              <div class="np-lyrics-skel" aria-hidden="true">
                <span class="np-skel-bar" style:width="62%"></span>
                <span class="np-skel-bar" style:width="48%"></span>
                <span class="np-skel-bar" style:width="74%"></span>
              </div>
            {:else if lyricsStatus === "notfound"}
              <p class="np-empty">{$t("study.music.now_playing_lyrics_empty")}</p>
            {:else if lyricsStatus === "error"}
              <p class="np-empty">{$t("study.music.now_playing_lyrics_error")}</p>
            {:else if lyricsLines.length === 0}
              <p class="np-empty">{$t("study.music.now_playing_lyrics_empty")}</p>
            {:else}
              <div bind:this={lyricsContainer} class="np-lyrics">
                <ol>
                  {#each lyricsLines as line, i (i + ":" + line.text)}
                    <li
                      bind:this={lineRefs[i]}
                      class="np-line"
                      class:active={i === activeLyricIdx}
                    >{line.text || " "}</li>
                  {/each}
                </ol>
              </div>
            {/if}
          {:else if tab === "queue"}
            {#if upNext.length === 0}
              <p class="np-empty">{$t("study.music.now_playing_queue_empty")}</p>
            {:else}
              <ol class="np-list">
                {#each upNext as qt, i (musicPlayer.queueIndex + 1 + i + ":" + qt.id)}
                  <li>
                    <button
                      type="button"
                      class="np-row"
                      onclick={() => jumpTo(musicPlayer.queueIndex + 1 + i)}
                    >
                      <span class="np-row-cover">
                        <CoverImage
                          src={qt.cover_path ?? qt.spotify_cover_url ?? qt.youtube_thumbnail ?? null}
                          alt={qt.title ?? qt.path}
                          size={36}
                          fallbackSeed={qt.album ?? qt.title}
                          rounded="sm"
                          trackId={qt.id}
                        />
                      </span>
                      <span class="np-row-meta">
                        <span class="np-row-title">{qt.title ?? qt.path}</span>
                        {#if qt.artist}<span class="np-row-artist">{qt.artist}</span>{/if}
                      </span>
                      <span class="np-row-time">{fmtDuration(qt.duration_ms ?? 0)}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          {:else if tab === "previous"}
            {#if previousTracks.length === 0}
              <p class="np-empty">{$t("study.music.now_playing_previous_empty")}</p>
            {:else}
              <ol class="np-list">
                {#each previousTracks as pt, i (musicPlayer.queueIndex - 1 - i + ":" + pt.id)}
                  <li>
                    <button
                      type="button"
                      class="np-row"
                      onclick={() => jumpTo(musicPlayer.queueIndex - 1 - i)}
                    >
                      <span class="np-row-cover">
                        <CoverImage
                          src={pt.cover_path ?? pt.spotify_cover_url ?? pt.youtube_thumbnail ?? null}
                          alt={pt.title ?? pt.path}
                          size={36}
                          fallbackSeed={pt.album ?? pt.title}
                          rounded="sm"
                          trackId={pt.id}
                        />
                      </span>
                      <span class="np-row-meta">
                        <span class="np-row-title">{pt.title ?? pt.path}</span>
                        {#if pt.artist}<span class="np-row-artist">{pt.artist}</span>{/if}
                      </span>
                      <span class="np-row-time">{fmtDuration(pt.duration_ms ?? 0)}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          {/if}
        </div>
      </section>
    </div>
  </div>
{:else}
  <div class="np-shell np-empty-shell">
    <p class="np-empty">{$t("study.music.now_playing_no_track")}</p>
    <button type="button" class="np-close" onclick={close}>{$t("study.music.now_playing_close")}</button>
  </div>
{/if}

<style>
  .np-shell {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    color: var(--secondary);
    isolation: isolate;
  }
  .np-empty-shell {
    display: grid;
    place-items: center;
    gap: 12px;
  }
  .np-backdrop {
    position: absolute;
    inset: -10%;
    z-index: -2;
    filter: blur(60px) saturate(1.4);
    opacity: 0.55;
    pointer-events: none;
  }
  .np-backdrop :global(img),
  .np-backdrop :global(.cover-fallback) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 0;
  }
  :global(.music-shell[data-reduce-animations="true"]) .np-backdrop {
    filter: saturate(1.2);
    opacity: 0.4;
  }
  .np-veil {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      radial-gradient(ellipse at 30% 0%, var(--music-accent-mid, transparent) 0%, transparent 60%),
      linear-gradient(to bottom, rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.6));
    pointer-events: none;
  }
  .np-close {
    position: absolute;
    top: 18px;
    left: 22px;
    z-index: 10;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 999px;
    color: rgba(255, 255, 255, 0.9);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    transition: background 120ms ease, border-color 120ms ease;
  }
  .np-close:hover {
    background: rgba(0, 0, 0, 0.5);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .np-grid {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    width: 100%;
    height: 100%;
    max-width: 1400px;
    margin-inline: auto;
    padding: 80px 56px 48px;
    gap: 56px;
    box-sizing: border-box;
  }

  .np-left {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 24px;
    min-width: 0;
  }
  .np-cover-wrap {
    position: relative;
    width: min(100%, 420px);
    aspect-ratio: 1;
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
    align-self: flex-start;
  }
  .np-cover-wrap :global(img),
  .np-cover-wrap :global(.cover-fallback) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
  }

  .np-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .np-title {
    font-size: 28px;
    font-weight: 800;
    line-height: 1.2;
    margin: 0;
    color: rgba(255, 255, 255, 0.98);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .np-artist {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.8);
    margin: 0;
  }
  .np-album {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.55);
    margin: 0;
  }

  .np-controls {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .np-btn {
    width: 38px;
    height: 38px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.85);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: color 120ms ease, background 120ms ease, transform 100ms ease;
  }
  .np-btn:hover { color: rgba(255, 255, 255, 1); background: rgba(255, 255, 255, 0.08); }
  .np-btn.active { color: var(--music-highlight, var(--accent)); }
  .np-btn:active { transform: scale(0.92); }
  .np-play {
    width: 60px;
    height: 60px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.96);
    color: #0f0f0f;
    cursor: pointer;
    display: grid;
    place-items: center;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    transition: transform 120ms ease, background 120ms ease;
  }
  .np-play:hover {
    transform: scale(1.05);
    background: rgba(255, 255, 255, 1);
  }
  .np-play:active { transform: scale(0.97); }
  .np-spinner {
    display: block;
    width: 24px;
    height: 24px;
    border: 2px solid rgba(0, 0, 0, 0.3);
    border-top-color: rgba(0, 0, 0, 0.85);
    border-radius: 50%;
    animation: np-spin 0.8s linear infinite;
  }
  @keyframes np-spin { to { transform: rotate(360deg); } }
  .np-fav {
    width: 38px;
    height: 38px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: color 120ms ease, background 120ms ease;
  }
  .np-fav:hover { color: rgba(255, 255, 255, 1); background: rgba(255, 255, 255, 0.08); }
  .np-fav.on { color: var(--music-highlight, var(--accent)); }
  .np-volume {
    margin-left: auto;
  }

  .np-right {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: rgba(0, 0, 0, 0.18);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 14px;
    overflow: hidden;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }
  .np-tabs {
    display: flex;
    gap: 4px;
    padding: 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .np-tab {
    flex: 1;
    padding: 9px 12px;
    background: transparent;
    border: 0;
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: color 120ms ease, background 120ms ease;
  }
  .np-tab:hover { color: rgba(255, 255, 255, 0.85); background: rgba(255, 255, 255, 0.04); }
  .np-tab.active {
    color: rgba(255, 255, 255, 1);
    background: rgba(255, 255, 255, 0.08);
  }
  .np-tab-count {
    display: inline-block;
    min-width: 18px;
    padding: 0 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    font-size: 10px;
    line-height: 16px;
    color: rgba(255, 255, 255, 0.7);
  }
  .np-tab-body {
    flex: 1;
    overflow-y: auto;
    padding: 14px 16px 18px;
    min-height: 0;
  }

  .np-empty {
    margin: 0;
    padding: 24px 8px;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
    text-align: center;
    line-height: 1.5;
  }

  .np-lyrics {
    height: 100%;
    overflow-y: auto;
  }
  .np-lyrics ol {
    list-style: none;
    margin: 0;
    padding: 18px 4px 50vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .np-line {
    font-size: 17px;
    font-weight: 600;
    line-height: 1.4;
    color: rgba(255, 255, 255, 0.36);
    transition: color 220ms ease, transform 220ms ease;
  }
  .np-line.active {
    color: rgba(255, 255, 255, 1);
    transform: scale(1.02);
    transform-origin: left center;
  }

  .np-lyrics-skel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 4px;
  }
  .np-skel-bar {
    height: 18px;
    background: linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.06) 0%,
      rgba(255, 255, 255, 0.12) 50%,
      rgba(255, 255, 255, 0.06) 100%
    );
    background-size: 200% 100%;
    border-radius: 6px;
    animation: np-shimmer 1.4s ease-in-out infinite;
  }
  @keyframes np-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .np-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .np-row {
    display: grid;
    grid-template-columns: 36px 1fr auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 8px;
    background: transparent;
    border: 0;
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 12.5px;
    cursor: pointer;
    text-align: left;
    transition: background 100ms ease;
  }
  .np-row:hover { background: rgba(255, 255, 255, 0.06); }
  .np-row-cover {
    width: 36px;
    height: 36px;
    border-radius: 5px;
    overflow: hidden;
  }
  .np-row-cover :global(img),
  .np-row-cover :global(.cover-fallback) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .np-row-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .np-row-title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: rgba(255, 255, 255, 0.95);
  }
  .np-row-artist {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .np-row-time {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 960px) {
    .np-grid {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
      padding: 76px 24px 24px;
      gap: 24px;
      overflow-y: auto;
      height: auto;
      min-height: 100%;
    }
    .np-cover-wrap {
      width: min(70vw, 320px);
      margin-inline: auto;
    }
    .np-meta { text-align: center; }
    .np-title { -webkit-line-clamp: unset; line-clamp: unset; }
    .np-controls { justify-content: center; flex-wrap: wrap; }
    .np-volume { margin-left: 0; width: 100%; display: flex; justify-content: center; }
  }
</style>
