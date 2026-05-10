<script lang="ts">
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import type { TabSummary } from "$lib/notes-bridge";

  type Props = {
    tab: TabSummary;
    x: number;
    y: number;
    onClose: () => void;
  };

  let { tab, x, y, onClose }: Props = $props();

  async function close() {
    onClose();
    await tabsStore.closeTab(tab.id);
  }
  async function closeOthers() {
    onClose();
    await tabsStore.closeOtherTabsInWnd(tab.wnd_id, tab.id);
  }
  async function closeRight() {
    onClose();
    await tabsStore.closeTabsToRight(tab.wnd_id, tab.id);
  }
  async function togglePin() {
    onClose();
    await tabsStore.pinTab(tab.id, !tab.pinned);
  }
  async function moveNewSplitRight() {
    onClose();
    const newWnd = await tabsStore.splitWnd(tab.wnd_id, "vertical");
    await tabsStore.moveTabToWnd(tab.id, newWnd);
  }
  async function moveNewSplitDown() {
    onClose();
    const newWnd = await tabsStore.splitWnd(tab.wnd_id, "horizontal");
    await tabsStore.moveTabToWnd(tab.id, newWnd);
  }

  function onWindowMouseDown(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.closest('[data-nb-context-menu="true"]')) return;
    onClose();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onmousedown={onWindowMouseDown} onkeydown={onKey} />

<div
  class="nb-tab-context-menu"
  data-nb-context-menu="true"
  style="left: {x}px; top: {y}px;"
  role="menu"
>
  <button class="item" role="menuitem" onclick={close}>
    Fechar
    <span class="shortcut">Ctrl+W</span>
  </button>
  <button class="item" role="menuitem" onclick={closeOthers}>Fechar outras</button>
  <button class="item" role="menuitem" onclick={closeRight}>Fechar à direita</button>
  <div class="sep"></div>
  <button class="item" role="menuitem" onclick={togglePin}>
    {tab.pinned ? "Desafixar" : "Fixar"}
  </button>
  <div class="sep"></div>
  <button class="item" role="menuitem" onclick={moveNewSplitRight}>Mover pra novo split à direita</button>
  <button class="item" role="menuitem" onclick={moveNewSplitDown}>Mover pra novo split abaixo</button>
</div>

<style>
  .nb-tab-context-menu {
    position: fixed;
    z-index: 200;
    min-width: 220px;
    background: var(--secondary-bg, var(--page-bg));
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 6px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.32);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: transparent;
    border: 0;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    transition: background 80ms ease;
  }
  .item:hover {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .shortcut {
    margin-left: auto;
    font-family: ui-monospace, monospace;
    font-size: 10px;
    color: var(--tertiary, var(--muted-fg));
  }
  .sep {
    height: 1px;
    margin: 4px 0;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
  }
</style>
