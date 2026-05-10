<script lang="ts">
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";

  type Props = {
    wndId: number;
    direction: "horizontal" | "vertical";
    ratio: number;
  };

  let { wndId, direction, ratio }: Props = $props();

  let dragging = $state(false);
  let containerEl: HTMLElement | null = null;

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    const handle = e.currentTarget as HTMLElement;
    containerEl = handle.parentElement;
    if (!containerEl) return;
    dragging = true;
    handle.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    let next: number;
    if (direction === "vertical") {
      next = (e.clientX - rect.left) / rect.width;
    } else {
      next = (e.clientY - rect.top) / rect.height;
    }
    next = Math.max(0.05, Math.min(0.95, next));
    void tabsStore.setRatio(wndId, next);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    const handle = e.currentTarget as HTMLElement;
    try {
      handle.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  }

  function onDoubleClick() {
    void tabsStore.setRatio(wndId, 0.5);
  }
</script>

<div
  class="nb-split-handle"
  class:vertical={direction === "vertical"}
  class:horizontal={direction === "horizontal"}
  class:dragging
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  ondblclick={onDoubleClick}
  role="separator"
  aria-orientation={direction === "vertical" ? "vertical" : "horizontal"}
  aria-valuenow={Math.round(ratio * 100)}
  aria-valuemin={5}
  aria-valuemax={95}
  tabindex="-1"
></div>

<style>
  .nb-split-handle {
    position: absolute;
    background: transparent;
    z-index: 5;
    transition: background 120ms ease;
    user-select: none;
    touch-action: none;
  }
  .nb-split-handle.vertical {
    width: 4px;
    height: 100%;
    top: 0;
    cursor: col-resize;
    transform: translateX(-2px);
  }
  .nb-split-handle.horizontal {
    height: 4px;
    width: 100%;
    left: 0;
    cursor: row-resize;
    transform: translateY(-2px);
  }
  .nb-split-handle:hover {
    background: color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .nb-split-handle.dragging {
    background: var(--accent);
  }
</style>
