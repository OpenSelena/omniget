<script lang="ts">
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { fmtDuration } from "$lib/study-music/format";
  import { t } from "$lib/i18n";
  import CoverImage from "./CoverImage.svelte";

  function jumpTo(idx: number) {
    if (idx < 0 || idx >= musicPlayer.queue.length) return;
    musicPlayer.queueIndex = idx;
    void musicPlayer.play(musicPlayer.queue[idx], musicPlayer.queue);
  }

  function close() {
    musicUI.closeQueue();
  }

  $effect(() => {
    if (!musicUI.queueOpen || typeof document === "undefined") return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });
</script>

{#if musicUI.queueOpen}
  <div class="queue-panel" role="dialog" aria-label={$t("study.music.queue_aria") as string} aria-modal="false" tabindex="-1">
    <header class="head">
      <h3>{$t("study.music.queue_title")}</h3>
      <span class="count">{musicPlayer.queue.length}</span>
      {#if musicPlayer.queue.length > 0}
        <button
          type="button"
          class="clear"
          onclick={() => musicPlayer.clearQueue()}
          title={$t("study.music.queue_clear") as string}
        >{$t("study.music.queue_clear")}</button>
      {/if}
      <button type="button" class="close" onclick={close} aria-label={$t("study.common.close")}>×</button>
    </header>
    {#if musicPlayer.queue.length === 0}
      <p class="muted">{$t("study.music.queue_empty")}</p>
    {:else}
      <ol class="list">
        {#each musicPlayer.queue as track, i (i + ":" + track.id)}
          {@const current = i === musicPlayer.queueIndex}
          <li class="row" class:current>
            <button type="button" class="play-cell" onclick={() => jumpTo(i)}>
              <CoverImage
                src={track.cover_path}
                alt={track.title ?? track.path}
                size={32}
                fallbackSeed={track.album ?? track.title}
                rounded="sm"
                trackId={track.id}
              />
              <span class="info">
                <span class="title">{track.title ?? track.path}</span>
                {#if track.artist}
                  <span class="artist">{track.artist}</span>
                {/if}
              </span>
              <span class="dur">{fmtDuration(track.duration_ms)}</span>
            </button>
          </li>
        {/each}
      </ol>
    {/if}
  </div>
{/if}

<style>
  .queue-panel {
    position: fixed;
    right: 12px;
    bottom: 92px;
    width: 360px;
    max-height: 60vh;
    background: var(--surface, var(--button-elevated));
    border: 1px solid var(--content-border);
    border-radius: 12px;
    box-shadow: 0 16px 48px color-mix(in oklab, black 38%, transparent);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    z-index: 100;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
  }
  .head h3 {
    margin: 0;
    flex: 1;
    font-size: 13px;
    font-weight: 700;
    color: var(--secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .count {
    font-size: 11px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
  }
  .clear {
    padding: 4px 10px;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 999px;
    color: var(--tertiary);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .clear:hover {
    color: var(--secondary);
    border-color: var(--secondary);
  }
  .close {
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--tertiary);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .close:hover { color: var(--secondary); background: color-mix(in oklab, var(--accent) 8%, transparent); }
  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .row { padding: 0; }
  .row.current .play-cell { background: color-mix(in oklab, var(--accent) 14%, transparent); }
  .row.current .title { color: var(--accent); }
  .play-cell {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--secondary);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .play-cell:hover { background: color-mix(in oklab, var(--accent) 6%, transparent); }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .title {
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 10px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dur {
    font-size: 10px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--tertiary);
    font-size: 12px;
    padding: 14px;
    margin: 0;
    text-align: center;
  }
</style>
