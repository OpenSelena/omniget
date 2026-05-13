<script lang="ts" generics="T extends { id: string | number }">
  import type { Snippet } from "svelte";

  type Props = {
    items: T[];
    onReorder: (orderedIds: Array<string | number>) => void;
    item: Snippet<[T, number, boolean]>;
    disabled?: boolean;
  };

  let { items, onReorder, item, disabled = false }: Props = $props();

  let draggingIndex = $state<number | null>(null);
  let overIndex = $state<number | null>(null);

  function onDragStart(e: DragEvent, idx: number) {
    if (disabled || items.length < 2) {
      e.preventDefault();
      return;
    }
    draggingIndex = idx;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(idx));
    }
  }

  function onDragOver(e: DragEvent, idx: number) {
    if (disabled) return;
    if (draggingIndex === null) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    if (overIndex !== idx) overIndex = idx;
  }

  function onDragLeave(idx: number) {
    if (overIndex === idx) overIndex = null;
  }

  function onDrop(e: DragEvent, target: number) {
    if (disabled) return;
    e.preventDefault();
    const from = draggingIndex;
    draggingIndex = null;
    overIndex = null;
    if (from === null || from === target) return;
    const next = items.slice();
    const [moved] = next.splice(from, 1);
    next.splice(target, 0, moved);
    onReorder(next.map((it) => it.id));
  }

  function onDragEnd() {
    draggingIndex = null;
    overIndex = null;
  }
</script>

<ul class="drag-list" class:disabled>
  {#each items as it, idx (it.id)}
    <li
      class="drag-item"
      class:dragging={draggingIndex === idx}
      class:drag-over={overIndex === idx && draggingIndex !== idx}
      draggable={!disabled && items.length >= 2}
      ondragstart={(e) => onDragStart(e, idx)}
      ondragover={(e) => onDragOver(e, idx)}
      ondragleave={() => onDragLeave(idx)}
      ondrop={(e) => onDrop(e, idx)}
      ondragend={onDragEnd}
    >
      {@render item(it, idx, draggingIndex === idx)}
    </li>
  {/each}
</ul>

<style>
  .drag-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .drag-item {
    position: relative;
    transition: transform 120ms ease, opacity 120ms ease;
    cursor: grab;
  }
  .drag-item:active {
    cursor: grabbing;
  }
  .drag-item.dragging {
    opacity: 0.45;
  }
  .drag-item.drag-over {
    box-shadow: inset 0 2px 0 0 var(--accent);
  }
  .drag-list.disabled .drag-item {
    cursor: default;
  }
  @media (prefers-reduced-motion: reduce) {
    .drag-item {
      transition: none;
    }
  }
</style>
