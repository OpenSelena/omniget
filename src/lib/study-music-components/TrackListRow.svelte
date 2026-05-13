<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    index?: number | null;
    coverUrl?: string | null;
    title: string;
    subtitle?: string | null;
    durationText?: string | null;
    isPlaying?: boolean;
    showIndex?: boolean;
    onPlay?: () => void;
    actions?: Snippet;
    ariaLabelPlay?: string;
  };

  let {
    index = null,
    coverUrl = null,
    title,
    subtitle = null,
    durationText = null,
    isPlaying = false,
    showIndex = true,
    onPlay,
    actions,
    ariaLabelPlay,
  }: Props = $props();

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onPlay?.();
    }
  }
</script>

<div class="row" class:is-playing={isPlaying} role="row">
  <button
    type="button"
    class="leading"
    onclick={() => onPlay?.()}
    onkeydown={handleKey}
    aria-label={ariaLabelPlay ?? title}
  >
    {#if coverUrl}
      <img class="cover-mini" src={coverUrl} alt="" loading="lazy" />
    {:else if showIndex && index != null}
      <span class="index tabular-nums">{index + 1}</span>
    {:else}
      <span class="index tabular-nums" aria-hidden="true">–</span>
    {/if}
  </button>
  <button
    type="button"
    class="body"
    onclick={() => onPlay?.()}
    aria-label={ariaLabelPlay ?? title}
  >
    <span class="title-line" title={title}>{title}</span>
    {#if subtitle}
      <span class="subtitle" title={subtitle}>{subtitle}</span>
    {/if}
  </button>
  {#if durationText}
    <span class="duration tabular-nums" aria-hidden="true">{durationText}</span>
  {/if}
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 48px 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 6px 8px;
    border-radius: 8px;
    transition: background 120ms ease;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .row.is-playing {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .leading {
    width: 48px;
    height: 48px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    overflow: hidden;
    padding: 0;
    cursor: pointer;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.55);
  }
  .cover-mini {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .index {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: rgba(255, 255, 255, 0.55);
  }
  .row.is-playing .index {
    color: var(--accent);
  }
  .body {
    text-align: left;
    background: transparent;
    border: 0;
    padding: 0;
    min-width: 0;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: inherit;
  }
  .title-line {
    font-size: 14px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.92);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.is-playing .title-line {
    color: var(--accent);
  }
  .subtitle {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .duration {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55);
    padding: 0 8px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  @media (prefers-reduced-motion: reduce) {
    .row {
      transition: none;
    }
  }
</style>
