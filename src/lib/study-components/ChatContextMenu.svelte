<script lang="ts">
  import { onMount } from "svelte";

  export type MenuItem = {
    id: string;
    label: string;
    icon?: string;
    danger?: boolean;
    disabled?: boolean;
    onSelect?: () => void;
  };

  type MenuItemOrSeparator = MenuItem | { separator: true };

  let {
    x,
    y,
    items,
    onClose,
  } = $props<{
    x: number;
    y: number;
    items: MenuItemOrSeparator[];
    onClose: () => void;
  }>();

  let menuEl: HTMLDivElement | undefined = $state();
  let activeIdx = $state<number>(-1);

  const focusableIndices = $derived(
    (items as MenuItemOrSeparator[])
      .map((it, i) =>
        "separator" in it || (it as MenuItem).disabled ? -1 : i,
      )
      .filter((i) => i >= 0),
  );

  function close() {
    onClose();
  }

  function selectByIndex(idx: number) {
    const it = items[idx];
    if (!it || "separator" in it || it.disabled) return;
    it.onSelect?.();
    close();
  }

  function moveActive(delta: 1 | -1) {
    if (focusableIndices.length === 0) return;
    const cur = focusableIndices.indexOf(activeIdx);
    const next =
      cur < 0
        ? delta > 0
          ? focusableIndices[0]
          : focusableIndices[focusableIndices.length - 1]
        : focusableIndices[
            (cur + delta + focusableIndices.length) % focusableIndices.length
          ];
    activeIdx = next;
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveActive(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveActive(-1);
    } else if (e.key === "Enter" || e.key === " ") {
      if (activeIdx >= 0) {
        e.preventDefault();
        selectByIndex(activeIdx);
      }
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) close();
  }

  onMount(() => {
    document.addEventListener("mousedown", handleClickOutside, true);
    document.addEventListener("keydown", handleKey);
    if (menuEl) {
      const r = menuEl.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let nx = x;
      let ny = y;
      if (nx + r.width > vw - 8) nx = Math.max(8, vw - r.width - 8);
      if (ny + r.height > vh - 8) ny = Math.max(8, vh - r.height - 8);
      menuEl.style.left = `${nx}px`;
      menuEl.style.top = `${ny}px`;
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside, true);
      document.removeEventListener("keydown", handleKey);
    };
  });
</script>

<div
  bind:this={menuEl}
  class="ctx-menu"
  role="menu"
  style:left="{x}px"
  style:top="{y}px"
>
  {#each items as it, i (i)}
    {#if "separator" in it}
      <div class="ctx-sep" role="separator" aria-hidden="true"></div>
    {:else}
      <button
        type="button"
        class="ctx-item"
        class:danger={it.danger}
        class:disabled={it.disabled}
        class:active={i === activeIdx}
        disabled={it.disabled}
        role="menuitem"
        onmousemove={() => (activeIdx = i)}
        onclick={() => selectByIndex(i)}
      >
        {#if it.icon}<span class="ctx-icon" aria-hidden="true">{it.icon}</span>{/if}
        <span class="ctx-label">{it.label}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .ctx-menu {
    position: fixed;
    z-index: 200;
    min-width: 220px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--content-border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18), 0 0 0 1px rgba(0, 0, 0, 0.04);
    display: flex;
    flex-direction: column;
    gap: 2px;
    animation: ctx-fade-in 120ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-origin: top left;
  }
  @keyframes ctx-fade-in {
    from { opacity: 0; transform: scale(0.96); }
    to   { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ctx-menu { animation: none; }
  }
  .ctx-sep {
    height: 1px;
    background: var(--content-border);
    margin: 4px 6px;
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    background: transparent;
    border: none;
    color: var(--secondary);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    border-radius: 6px;
    transition: background var(--tg-duration-fast, 150ms) var(--tg-easing-default, ease);
  }
  .ctx-item.active,
  .ctx-item:hover:not(.disabled):not(:disabled) {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    color: var(--text);
  }
  .ctx-item.danger {
    color: var(--error);
  }
  .ctx-item.danger.active,
  .ctx-item.danger:hover:not(.disabled):not(:disabled) {
    background: color-mix(in oklab, var(--error) 12%, transparent);
  }
  .ctx-item.disabled,
  .ctx-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ctx-icon {
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    font-size: 14px;
  }
  .ctx-label {
    flex: 1;
  }
</style>
