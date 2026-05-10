<script lang="ts">
  import { musicPlayer } from "$lib/study-music/player-store.svelte";

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  const ratio = $derived(musicPlayer.muted ? 0 : musicPlayer.volume);

  function ratioFromEvent(e: PointerEvent): number {
    if (!trackEl) return 0;
    const rect = trackEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    return Math.max(0, Math.min(1, x / rect.width));
  }

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    musicPlayer.setVolume(ratioFromEvent(e));
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    musicPlayer.setVolume(ratioFromEvent(e));
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="volume">
  <button
    type="button"
    class="mute-btn"
    onclick={() => musicPlayer.toggleMute()}
    aria-label={musicPlayer.muted ? "Tirar mudo" : "Mudo"}
  >
    {#if musicPlayer.muted || musicPlayer.volume === 0}
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <line x1="23" y1="9" x2="17" y2="15"/>
        <line x1="17" y1="9" x2="23" y2="15"/>
      </svg>
    {:else if ratio < 0.4}
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
        <path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>
      </svg>
    {/if}
  </button>
  <div
    class="track"
    bind:this={trackEl}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    role="slider"
    aria-valuemin="0"
    aria-valuemax="1"
    aria-valuenow={ratio}
    tabindex="0"
  >
    <div class="bar"></div>
    <div class="filled" style:width="{ratio * 100}%"></div>
    <div class="thumb" style:left="{ratio * 100}%"></div>
  </div>
</div>

<style>
  .volume {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .mute-btn {
    background: transparent;
    border: 0;
    padding: 6px;
    color: var(--tertiary);
    cursor: pointer;
    display: grid;
    place-items: center;
    border-radius: 4px;
    transition: color 120ms ease, background 120ms ease;
  }
  .mute-btn:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .track {
    position: relative;
    width: 90px;
    height: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .bar {
    position: absolute;
    inset: 50% 0 auto;
    transform: translateY(-50%);
    height: 4px;
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
    width: 10px;
    height: 10px;
    margin-left: -5px;
    margin-top: -5px;
    background: var(--secondary);
    border-radius: 50%;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }
  .track:hover .thumb {
    opacity: 1;
  }
</style>
