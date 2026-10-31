<script lang="ts">
  import { onMount } from "svelte";

  let { storageKey, min = 240, max = 400, defaultWidth = 280, cssVar = "--mini-list-width" }: {
    storageKey: string;
    min?: number;
    max?: number;
    defaultWidth?: number;
    cssVar?: string;
  } = $props();

  let dragging = $state(false);
  let startX = 0;
  let startWidth = defaultWidth;

  function applyWidth(w: number) {
    const clamped = Math.max(min, Math.min(max, w));
    document.documentElement.style.setProperty(cssVar, `${clamped}px`);
    return clamped;
  }

  onMount(() => {
    const stored = localStorage.getItem(storageKey);
    const initial = stored ? parseInt(stored, 10) : defaultWidth;
    applyWidth(Number.isFinite(initial) ? initial : defaultWidth);
  });

  function onPointerDown(ev: PointerEvent) {
    dragging = true;
    startX = ev.clientX;
    const cur = parseInt(getComputedStyle(document.documentElement).getPropertyValue(cssVar), 10);
    startWidth = Number.isFinite(cur) ? cur : defaultWidth;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
  }

  function onPointerMove(ev: PointerEvent) {
    if (!dragging) return;
    const delta = ev.clientX - startX;
    const newWidth = applyWidth(startWidth + delta);
    localStorage.setItem(storageKey, String(newWidth));
  }

  function onPointerUp(ev: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resize-bar"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
></div>

<style>
  .resize-bar {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background var(--tg-duration-fast, 150ms) var(--tg-easing-default, ease);
    flex-shrink: 0;
    touch-action: none;
  }
  .resize-bar:hover,
  .resize-bar.dragging {
    background: color-mix(in oklab, var(--accent) 40%, transparent);
  }
  .resize-bar.dragging {
    cursor: col-resize;
  }
</style>
