<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t } from "$lib/i18n";
  import {
    unwatchStream,
    setStreamVolume,
    refreshStreamStats,
    getStreamStats,
  } from "$lib/stores/omnidisc-stream-store.svelte";

  let userId = $derived(page.url.searchParams.get("user") ?? "");
  let volume = $state(100);
  let fullscreen = $state(false);
  let controlsVisible = $state(true);
  let stats = $derived(getStreamStats());
  let mine = $derived(stats?.watching.find((w) => w.user_id === userId) ?? null);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  let viewportEl: HTMLDivElement;

  async function reportViewport() {
    if (!viewportEl || !userId) return;
    const rect = viewportEl.getBoundingClientRect();
    const scale = window.devicePixelRatio || 1;
    try {
      await invoke("omnidisc_stream_set_viewport", {
        args: {
          user_id: userId,
          x: rect.left,
          y: rect.top,
          width: rect.width,
          height: rect.height,
          scale,
          surface_width: Math.round(window.innerWidth * scale),
          surface_height: Math.round(window.innerHeight * scale),
          background: [0, 0, 0],
        },
      });
    } catch {
      /* window may be closing */
    }
  }

  function onVolume(v: number) {
    volume = v;
    void setStreamVolume(userId, v / 100);
  }

  async function toggleFullscreen() {
    const win = getCurrentWindow();
    fullscreen = !fullscreen;
    await win.setFullscreen(fullscreen);
  }

  async function stop() {
    await unwatchStream(userId);
    await getCurrentWindow().close();
  }

  function nudgeControls() {
    controlsVisible = true;
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (controlsVisible = false), 2600);
  }

  onMount(() => {
    const root = document.documentElement;
    const prevHtml = root.style.background;
    const prevBody = document.body.style.background;
    root.style.background = "transparent";
    document.body.style.background = "transparent";

    void reportViewport();
    const ro = new ResizeObserver(() => void reportViewport());
    if (viewportEl) ro.observe(viewportEl);
    window.addEventListener("resize", reportViewport);

    const statsTimer = setInterval(() => void refreshStreamStats(), 2000);
    nudgeControls();

    return () => {
      ro.disconnect();
      window.removeEventListener("resize", reportViewport);
      clearInterval(statsTimer);
      if (hideTimer) clearTimeout(hideTimer);
      root.style.background = prevHtml;
      document.body.style.background = prevBody;
      void invoke("omnidisc_stream_unwatch", { userId }).catch(() => undefined);
    };
  });
</script>

<div
  class="viewport"
  bind:this={viewportEl}
  role="presentation"
  onmousemove={nudgeControls}
  class:idle={!controlsVisible}
>
  <div class="overlay" class:hidden={!controlsVisible}>
    <div class="badge">
      {#if mine && mine.width > 0}
        {mine.width}×{mine.height} · {Math.round(mine.fps_received)}fps{#if mine.decoder} · {mine.decoder}{/if}
      {:else}
        {$t("omnidisc.stream.connecting")}
      {/if}
    </div>
    <div class="controls">
      <label class="vol">
        <span class="sr-only">{$t("omnidisc.stream.volume")}</span>
        <input type="range" min="0" max="200" step="5" value={volume} oninput={(e) => onVolume(Number(e.currentTarget.value))} />
        <span class="vol-val">{volume}%</span>
      </label>
      <button type="button" class="ctl" onclick={toggleFullscreen} title={$t("omnidisc.stream.fullscreen")}>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg>
      </button>
      <button type="button" class="ctl stop" onclick={stop} title={$t("omnidisc.stream.stop_watching")}>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </div>
  </div>
</div>

<style>
  :global(body) {
    background: transparent;
  }

  .viewport {
    position: fixed;
    inset: 0;
    background: transparent;
    cursor: default;
  }

  .viewport.idle {
    cursor: none;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    pointer-events: none;
    transition: opacity 200ms ease;
  }

  .overlay.hidden {
    opacity: 0;
  }

  .badge {
    align-self: flex-start;
    margin: var(--space-3);
    padding: 4px var(--space-2);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, black 55%, transparent);
    color: white;
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
    pointer-events: auto;
  }

  .controls {
    align-self: center;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-full, 999px);
    background: color-mix(in srgb, black 55%, transparent);
    pointer-events: auto;
  }

  .vol {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 180px;
  }

  .vol input {
    flex: 1;
    accent-color: var(--accent);
  }

  .vol-val {
    color: white;
    font-size: var(--text-xs);
    min-width: 36px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .ctl {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: var(--radius-full, 999px);
    background: color-mix(in srgb, white 14%, transparent);
    color: white;
    cursor: pointer;
  }

  .ctl:hover {
    background: color-mix(in srgb, white 26%, transparent);
  }

  .ctl.stop:hover {
    background: var(--danger);
  }

  .ctl:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay {
      transition: none;
    }
  }
</style>
