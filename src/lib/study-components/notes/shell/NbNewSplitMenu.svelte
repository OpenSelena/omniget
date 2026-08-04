<script lang="ts">
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";

  type Props = { wndId: number };

  let { wndId }: Props = $props();

  let open = $state(false);
  let buttonEl: HTMLButtonElement | null = null;

  function toggle() {
    open = !open;
  }

  async function doSplit(direction: "horizontal" | "vertical") {
    open = false;
    await tabsStore.splitWnd(wndId, direction);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      open = false;
      buttonEl?.focus();
    }
  }

  function onWindowMouseDown(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node | null;
    if (buttonEl && target && !buttonEl.contains(target)) {
      open = false;
    }
  }
</script>

<svelte:window onmousedown={onWindowMouseDown} onkeydown={onKey} />

<div class="nb-new-split-menu">
  <button
    type="button"
    class="trigger"
    class:active={open}
    onclick={toggle}
    bind:this={buttonEl}
    aria-haspopup="menu"
    aria-expanded={open}
    title="Novo split"
  >
    <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
      <rect x="1" y="1" width="14" height="14" rx="1" />
      <line x1="8" y1="1" x2="8" y2="15" />
    </svg>
  </button>
  {#if open}
    <div class="menu" role="menu">
      <button class="item" role="menuitem" onclick={() => doSplit("vertical")}>
        <span class="icon">⇨</span>
        Split à direita
        <span class="shortcut">Ctrl+\</span>
      </button>
      <button class="item" role="menuitem" onclick={() => doSplit("horizontal")}>
        <span class="icon">⇩</span>
        Split abaixo
        <span class="shortcut">Ctrl+Shift+\</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .nb-new-split-menu {
    position: relative;
    display: inline-flex;
    align-items: center;
    height: 100%;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 100%;
    background: transparent;
    border: 0;
    color: var(--tertiary, var(--muted-fg));
    cursor: pointer;
    transition: color 120ms ease, background 120ms ease;
  }
  .trigger:hover,
  .trigger.active {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 50;
    min-width: 180px;
    background: var(--secondary-bg, var(--page-bg));
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    color: var(--text);
    text-align: left;
    font-size: 12px;
    cursor: pointer;
    border-radius: 4px;
    transition: background 80ms ease;
  }
  .item:hover {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .icon {
    width: 16px;
    text-align: center;
    color: var(--tertiary, var(--muted-fg));
  }
  .shortcut {
    margin-left: auto;
    font-family: ui-monospace, monospace;
    font-size: 10px;
    color: var(--tertiary, var(--muted-fg));
  }
</style>
