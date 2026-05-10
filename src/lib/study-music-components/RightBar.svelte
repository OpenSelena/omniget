<script lang="ts">
  import { tick } from "svelte";
  import { musicUI, type RightBarTab } from "$lib/study-music/ui-store.svelte";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import { lyricsStore } from "$lib/study-music/lyrics-store.svelte";
  import { fmtDuration, fmtDurationLong, trackDisplayTitle } from "$lib/study-music/format";
  import { t } from "$lib/i18n";
  import CoverImage from "./CoverImage.svelte";

  let lyricsContainer = $state<HTMLDivElement | null>(null);
  let lineRefs: HTMLLIElement[] = [];

  const currentTrack = $derived(musicPlayer.currentTrack);
  const activeIdx = $derived(
    lyricsStore.lines.length > 0
      ? lyricsStore.activeIndex(musicPlayer.currentTime)
      : -1,
  );

  $effect(() => {
    if (musicUI.rightbarTab !== "lyrics") return;
    const id = currentTrack?.id ?? null;
    if (id === null) {
      lyricsStore.reset();
      return;
    }
    void lyricsStore.loadFor(id);
  });

  $effect(() => {
    if (musicUI.rightbarTab !== "lyrics") return;
    void activeIdx;
    void tick().then(() => {
      const el = lineRefs[activeIdx];
      if (el && lyricsContainer) {
        const rect = el.getBoundingClientRect();
        const cRect = lyricsContainer.getBoundingClientRect();
        const offset =
          el.offsetTop - lyricsContainer.offsetTop - cRect.height / 2 + rect.height / 2;
        lyricsContainer.scrollTo({ top: offset, behavior: "smooth" });
      }
    });
  });

  function jumpTo(idx: number) {
    if (idx < 0 || idx >= musicPlayer.queue.length) return;
    musicPlayer.queueIndex = idx;
    void musicPlayer.play(musicPlayer.queue[idx], musicPlayer.queue);
  }

  function setTab(tab: RightBarTab) {
    musicUI.rightbarTab = tab;
  }

  function close() {
    musicUI.rightbarTab = null;
  }

  function refreshLyrics() {
    if (currentTrack) void lyricsStore.loadFor(currentTrack.id, true);
  }

  $effect(() => {
    if (typeof document === "undefined") return;
    function onKey(e: KeyboardEvent) {
      if (musicUI.rightbarTab === null) return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });
</script>

{#if musicUI.rightbarTab !== null}
  <aside class="right-bar" role="complementary">
    <header class="head">
      <nav class="tabs" role="tablist">
        <button
          type="button"
          role="tab"
          class="tab"
          class:active={musicUI.rightbarTab === "queue"}
          aria-selected={musicUI.rightbarTab === "queue"}
          onclick={() => setTab("queue")}
        >
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/>
            <line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>
          </svg>
          <span>{$t("study.music.tab_queue")}</span>
          {#if musicPlayer.queue.length > 0}
            <span class="badge">{musicPlayer.queue.length}</span>
          {/if}
        </button>
        <button
          type="button"
          role="tab"
          class="tab"
          class:active={musicUI.rightbarTab === "lyrics"}
          aria-selected={musicUI.rightbarTab === "lyrics"}
          onclick={() => setTab("lyrics")}
        >
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M9 18V5l12-2v13"/>
            <line x1="3" y1="9" x2="9" y2="9"/>
            <line x1="3" y1="13" x2="7" y2="13"/>
          </svg>
          <span>{$t("study.music.tab_lyrics")}</span>
        </button>
        <button
          type="button"
          role="tab"
          class="tab"
          class:active={musicUI.rightbarTab === "info"}
          aria-selected={musicUI.rightbarTab === "info"}
          onclick={() => setTab("info")}
        >
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="9"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>{$t("study.music.tab_info")}</span>
        </button>
      </nav>
      <button type="button" class="close" onclick={close} aria-label={$t("study.common.close") as string}>×</button>
    </header>

    <div class="body">
      {#if musicUI.rightbarTab === "queue"}
        {#if musicPlayer.queue.length === 0}
          <p class="empty">{$t("study.music.queue_empty")}</p>
        {:else}
          <div class="queue-bar">
            <span class="muted">{$t("study.music.tracks_count", { count: musicPlayer.queue.length })}</span>
            <button type="button" class="link" onclick={() => musicPlayer.clearQueue()}>
              {$t("study.music.queue_clear")}
            </button>
          </div>
          <ol class="queue-list">
            {#each musicPlayer.queue as track, i (i + ":" + track.id)}
              {@const current = i === musicPlayer.queueIndex}
              <li class="q-row" class:current>
                <button type="button" class="q-btn" onclick={() => jumpTo(i)}>
                  <CoverImage
                    src={track.cover_path}
                    alt={track.title ?? track.path}
                    size={32}
                    fallbackSeed={track.album ?? track.title}
                    rounded="sm"
                    trackId={track.id}
                  />
                  <span class="q-info">
                    <span class="q-title">{trackDisplayTitle(track)}</span>
                    {#if track.artist}<span class="q-artist">{track.artist}</span>{/if}
                  </span>
                  <span class="q-dur">{fmtDuration(track.duration_ms)}</span>
                </button>
              </li>
            {/each}
          </ol>
        {/if}
      {:else if musicUI.rightbarTab === "lyrics"}
        {#if !currentTrack}
          <p class="empty">{$t("study.music.lyrics_no_track")}</p>
        {:else if lyricsStore.loading}
          <div class="loading"><span class="spinner"></span><p>{$t("study.music.lyrics_loading")}</p></div>
        {:else if lyricsStore.notFound}
          <div class="empty">
            <p>{$t("study.music.lyrics_not_found")}</p>
            <p class="muted small">{$t("study.music.lyrics_lrclib_hint")}</p>
            <button type="button" class="link" onclick={refreshLyrics}>
              {$t("study.music.lyrics_refresh")}
            </button>
          </div>
        {:else if lyricsStore.lines.length > 0}
          <div bind:this={lyricsContainer} class="lyrics synced">
            <ul class="lines">
              {#each lyricsStore.lines as line, i (i)}
                <li
                  bind:this={lineRefs[i]}
                  class="line"
                  class:active={i === activeIdx}
                  class:past={i < activeIdx}
                >{line.text || "♪"}</li>
              {/each}
            </ul>
          </div>
        {:else if lyricsStore.plain}
          <div class="lyrics plain">
            {#each lyricsStore.plain.split(/\r?\n/) as line, i (i)}
              <p class="plain-line">{line || " "}</p>
            {/each}
          </div>
        {/if}
      {:else if musicUI.rightbarTab === "info"}
        {#if !currentTrack}
          <p class="empty">{$t("study.music.info_no_track")}</p>
        {:else}
          <div class="info-block">
            <div class="info-cover">
              <CoverImage
                src={currentTrack.cover_path}
                alt={currentTrack.title ?? ""}
                fallbackSeed={currentTrack.album ?? currentTrack.title}
                rounded="lg"
                trackId={currentTrack.id}
              />
            </div>
            <h2 class="info-title">{trackDisplayTitle(currentTrack)}</h2>
            {#if currentTrack.artist}
              <p class="info-artist">{currentTrack.artist}</p>
            {/if}
            <dl class="info-meta">
              {#if currentTrack.album}
                <dt>{$t("study.music.field_album")}</dt>
                <dd>{currentTrack.album}</dd>
              {/if}
              {#if currentTrack.year}
                <dt>{$t("study.music.field_year")}</dt>
                <dd>{currentTrack.year}</dd>
              {/if}
              {#if currentTrack.genre}
                <dt>{$t("study.music.field_genre")}</dt>
                <dd>{currentTrack.genre}</dd>
              {/if}
              {#if currentTrack.duration_ms}
                <dt>{$t("study.music.field_duration")}</dt>
                <dd>{fmtDurationLong(currentTrack.duration_ms)}</dd>
              {/if}
              {#if currentTrack.bitrate}
                <dt>{$t("study.music.field_bitrate")}</dt>
                <dd>{currentTrack.bitrate} kbps</dd>
              {/if}
              {#if currentTrack.sample_rate}
                <dt>{$t("study.music.field_samplerate")}</dt>
                <dd>{Math.round(currentTrack.sample_rate / 1000)} kHz</dd>
              {/if}
              {#if currentTrack.play_count !== undefined && currentTrack.play_count > 0}
                <dt>{$t("study.music.field_plays")}</dt>
                <dd>{currentTrack.play_count}</dd>
              {/if}
            </dl>
            <p class="info-path" title={currentTrack.path}>{currentTrack.path}</p>
          </div>
        {/if}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .right-bar {
    position: fixed;
    top: 14px;
    right: 14px;
    bottom: 96px;
    width: min(380px, 92vw);
    z-index: 95;
    background: rgba(20, 20, 20, 0.96);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    backdrop-filter: blur(8px);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 10px 10px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .tabs {
    display: flex;
    gap: 2px;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 11px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }
  .tab:hover { color: rgba(255, 255, 255, 0.95); }
  .tab.active {
    background: rgba(255, 255, 255, 0.08);
    color: white;
  }
  .badge {
    padding: 0 6px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.9);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .close {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .close:hover { color: white; background: rgba(255, 255, 255, 0.08); }
  .body {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .queue-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 11px; }
  .small { font-size: 11px; }
  .link {
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.65);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    padding: 4px 0;
  }
  .link:hover { color: white; text-decoration: underline; }
  .queue-list {
    flex: 1;
    overflow-y: auto;
    list-style: none;
    margin: 0;
    padding: 6px 6px 24px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .q-row.current .q-title { color: var(--accent); }
  .q-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
  }
  .q-btn:hover { background: rgba(255, 255, 255, 0.05); }
  .q-row.current .q-btn { background: color-mix(in oklab, var(--accent) 14%, transparent); }
  .q-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .q-title {
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .q-artist {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .q-dur {
    flex-shrink: 0;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 32px 16px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
    text-align: center;
  }
  .loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid rgba(255, 255, 255, 0.15);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .lyrics {
    flex: 1;
    overflow-y: auto;
    padding: 16px 22px 32px;
    scrollbar-width: thin;
  }
  .lyrics .lines {
    list-style: none;
    margin: 0;
    padding: 30vh 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .line {
    font-size: 16px;
    line-height: 1.4;
    color: rgba(255, 255, 255, 0.4);
    transition: color 220ms ease, transform 220ms ease, font-size 220ms ease;
  }
  .line.past { color: rgba(255, 255, 255, 0.25); }
  .line.active {
    color: white;
    font-weight: 700;
    font-size: 19px;
    transform: scale(1.02);
    transform-origin: left;
  }
  .plain-line {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.85);
  }
  .info-block {
    flex: 1;
    overflow-y: auto;
    padding: 18px 18px 24px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .info-cover {
    width: 100%;
    aspect-ratio: 1 / 1;
    margin-bottom: 4px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    border-radius: 10px;
    overflow: hidden;
  }
  .info-title {
    margin: 8px 0 0;
    font-size: 18px;
    font-weight: 800;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .info-artist {
    margin: 0;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
  }
  .info-meta {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 12px 0 0;
    padding-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 12px;
  }
  .info-meta dt {
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.06em;
    align-self: center;
  }
  .info-meta dd {
    margin: 0;
    color: rgba(255, 255, 255, 0.95);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .info-path {
    margin: 16px 0 0;
    padding: 8px;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 6px;
    font-family: ui-monospace, monospace;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.45);
    word-break: break-all;
  }
</style>
