<script lang="ts">
  import {
    notesShell,
    NOTES_SHELL_DOCK_MIN,
    NOTES_SHELL_DOCK_MAX,
  } from "$lib/study-notes/shell-store.svelte";

  type Props = { side: "left" | "right" };
  let { side }: Props = $props();

  let dragging = $state(false);
  let startX = 0;
  let startWidth = 0;

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    startX = e.clientX;
    startWidth =
      side === "left" ? notesShell.dockLeftWidth : notesShell.dockRightWidth;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const delta = e.clientX - startX;
    if (side === "left") {
      notesShell.setDockLeftWidth(startWidth + delta);
    } else {
      notesShell.setDockRightWidth(startWidth - delta);
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
    void notesShell.persistBackend();
  }

  function onDoubleClick() {
    if (side === "left") {
      notesShell.setDockLeftWidth(240);
    } else {
      notesShell.setDockRightWidth(280);
    }
    void notesShell.persistBackend();
  }
</script>

<div
  class="handle"
  class:left={side === "left"}
  class:right={side === "right"}
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-label={side === "left" ? "Redimensionar dock esquerdo" : "Redimensionar dock direito"}
  aria-valuemin={NOTES_SHELL_DOCK_MIN}
  aria-valuemax={NOTES_SHELL_DOCK_MAX}
  aria-valuenow={side === "left" ? notesShell.dockLeftWidth : notesShell.dockRightWidth}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  ondblclick={onDoubleClick}
></div>

<style>
  .handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background 120ms ease;
    z-index: 3;
  }
  .handle.left {
    right: -2px;
  }
  .handle.right {
    left: -2px;
  }
  .handle:hover,
  .handle.dragging {
    background: color-mix(in oklab, var(--accent) 50%, transparent);
  }
  .handle:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
</style>
