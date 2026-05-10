<script lang="ts">
  import { onMount, tick } from "svelte";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { lyricsStore } from "$lib/study-music/lyrics-store.svelte";
  import { t } from "$lib/i18n";

  let containerEl = $state<HTMLDivElement | null>(null);
  let lineRefs: HTMLLIElement[] = [];

  const trackId = $derived(musicPlayer.currentTrack?.id ?? null);
  const activeIdx = $derived(
    lyricsStore.lines.length > 0
      ? lyricsStore.activeIndex(musicPlayer.currentTime)
      : -1,
  );

  $effect(() => {
    if (!musicUI.lyricsOpen) return;
    if (trackId === null) {
      lyricsStore.reset();
      return;
    }
    void lyricsStore.loadFor(trackId);
  });

  $effect(() => {
    if (!musicUI.lyricsOpen) return;
    void activeIdx;
    void tick().then(() => {
      const el = lineRefs[activeIdx];
      if (el && containerEl) {
        const rect = el.getBoundingClientRect();
        const cRect = containerEl.getBoundingClientRect();
        const offset =
          el.offsetTop - containerEl.offsetTop - cRect.height / 2 + rect.height / 2;
        containerEl.scrollTo({ top: offset, behavior: "smooth" });
      }
    });
  });

  function close() {
    musicUI.closeLyrics();
  }

  function refresh() {
    if (trackId !== null) {
      void lyricsStore.loadFor(trackId, true);
    }
  }

  onMount(() => {
    if (typeof document === "undefined") return;
    const handler = (e: KeyboardEvent) => {
      if (!musicUI.lyricsOpen) return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      )
        return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  });
</script>

{#if musicUI.lyricsOpen}
  <aside class="lyrics-panel" role="complementary">
    <header class="head">
      <h3>{$t("study.music.lyrics_title")}</h3>
      <div class="actions">
        <button
          type="button"
          class="ico"
          onclick={refresh}
          title={$t("study.music.lyrics_refresh") as string}
          aria-label={$t("study.music.lyrics_refresh") as string}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="23 4 23 10 17 10"/>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
          </svg>
        </button>
        <button
          type="button"
          class="ico"
          onclick={close}
          aria-label={$t("study.common.close") as string}
        >×</button>
      </div>
    </header>

    {#if !musicPlayer.currentTrack}
      <div class="empty">
        <p>{$t("study.music.lyrics_no_track")}</p>
      </div>
    {:else if lyricsStore.loading}
      <div class="loading">
        <span class="spinner"></span>
        <p>{$t("study.music.lyrics_loading")}</p>
      </div>
    {:else if lyricsStore.notFound}
      <div class="empty">
        <p>{$t("study.music.lyrics_not_found")}</p>
        <p class="muted">{$t("study.music.lyrics_lrclib_hint")}</p>
      </div>
    {:else if lyricsStore.lines.length > 0}
      <div bind:this={containerEl} class="scroll synced">
        <ul class="lines">
          {#each lyricsStore.lines as line, i (i)}
            <li
              bind:this={lineRefs[i]}
              class="line"
              class:active={i === activeIdx}
              class:past={i < activeIdx}
            >
              {line.text || "♪"}
            </li>
          {/each}
        </ul>
      </div>
    {:else if lyricsStore.plain}
      <div class="scroll plain">
        {#each lyricsStore.plain.split(/\r?\n/) as line, i (i)}
          <p class="plain-line">{line || " "}</p>
        {/each}
      </div>
    {/if}

    {#if lyricsStore.source}
      <footer class="foot">
        <span>{$t("study.music.lyrics_source", { source: lyricsStore.source })}</span>
      </footer>
    {/if}
  </aside>
{/if}

<style>
  .lyrics-panel {
    position: fixed;
    top: 14px;
    right: 14px;
    bottom: 96px;
    width: min(420px, 92vw);
    z-index: 95;
    background: rgb(20, 20, 20);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .head h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.85);
  }
  .actions { display: inline-flex; gap: 4px; }
  .ico {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }
  .ico:hover { color: white; background: rgba(255, 255, 255, 0.08); }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 22px 32px;
    scrollbar-width: thin;
  }
  .lines {
    list-style: none;
    margin: 0;
    padding: 30vh 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .line {
    font-size: 18px;
    line-height: 1.4;
    color: rgba(255, 255, 255, 0.4);
    transition: color 220ms ease, transform 220ms ease, font-size 220ms ease;
    transform-origin: left;
  }
  .line.past {
    color: rgba(255, 255, 255, 0.25);
  }
  .line.active {
    color: white;
    font-weight: 700;
    font-size: 22px;
    transform: scale(1.02);
  }
  .plain-line {
    margin: 0 0 10px;
    font-size: 14px;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.85);
  }
  .empty,
  .loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
    justify-content: center;
    padding: 24px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
    text-align: center;
  }
  .muted { color: rgba(255, 255, 255, 0.35); font-size: 11px; }
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid rgba(255, 255, 255, 0.15);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .foot {
    padding: 8px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.35);
    font-size: 10px;
    text-align: right;
  }
</style>
