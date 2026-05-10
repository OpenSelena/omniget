<script lang="ts">
  import { fmtSecondsToTime } from "$lib/study-music/format";

  type Props = {
    current: number;
    duration: number;
    onSeek: (seconds: number) => void;
    showTime?: boolean;
  };

  let { current, duration, onSeek, showTime = true }: Props = $props();

  let dragging = $state(false);
  let dragValue = $state(0);
  let trackEl = $state<HTMLDivElement | null>(null);

  const ratio = $derived.by(() => {
    if (dragging) return dragValue;
    if (!duration || duration <= 0 || !Number.isFinite(duration)) return 0;
    return Math.max(0, Math.min(1, current / duration));
  });

  const displayTime = $derived(dragging ? dragValue * duration : current);

  function ratioFromEvent(e: PointerEvent): number {
    if (!trackEl) return 0;
    const rect = trackEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    return Math.max(0, Math.min(1, x / rect.width));
  }

  function onPointerDown(e: PointerEvent) {
    if (!duration || duration <= 0) return;
    e.preventDefault();
    dragging = true;
    dragValue = ratioFromEvent(e);
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    dragValue = ratioFromEvent(e);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    onSeek(dragValue * duration);
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="progress-row">
  {#if showTime}
    <span class="t left">{fmtSecondsToTime(displayTime)}</span>
  {/if}
  <div
    class="track"
    bind:this={trackEl}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    role="slider"
    aria-valuemin="0"
    aria-valuemax={duration || 0}
    aria-valuenow={current}
    tabindex="0"
  >
    <div class="bar"></div>
    <div class="filled" style:width="{ratio * 100}%"></div>
    <div class="thumb" style:left="{ratio * 100}%"></div>
  </div>
  {#if showTime}
    <span class="t right">{fmtSecondsToTime(duration)}</span>
  {/if}
</div>

<style>
  .progress-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
  }
  .t {
    font-size: 11px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
  }
  .track {
    position: relative;
    flex: 1;
    height: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .track:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 4px;
  }
  .bar {
    position: absolute;
    inset: 50% 0 auto;
    transform: translateY(-50%);
    height: 4px;
    width: 100%;
    background: color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 2px;
  }
  .filled {
    position: absolute;
    inset: 50% auto auto 0;
    transform: translateY(-50%);
    height: 4px;
    background: var(--secondary);
    border-radius: 2px;
    pointer-events: none;
  }
  .track:hover .filled {
    background: var(--accent);
  }
  .thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    margin-left: -6px;
    margin-top: -6px;
    background: var(--secondary);
    border-radius: 50%;
    pointer-events: none;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .track:hover .thumb {
    opacity: 1;
  }
</style>
