<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { notesShell } from "$lib/study-notes/shell-store.svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import { docksStore } from "$lib/study-notes/docks-store.svelte";
  import { notebooksStore } from "$lib/study-notes/notebooks-store.svelte";
  import NbStatusBar from "./NbStatusBar.svelte";
  import NbDockLeft from "./NbDockLeft.svelte";
  import NbDockRight from "./NbDockRight.svelte";
  import NbDockSidebar from "./NbDockSidebar.svelte";
  import NbWndTree from "./NbWndTree.svelte";

  let { children } = $props();

  function exitFullscreen() {
    const target = notesShell.consumeReturnUrl();
    void goto(target);
  }

  function shouldIgnoreEsc(target: HTMLElement | null): boolean {
    if (!target) return false;
    const tag = target.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return true;
    if (target.isContentEditable) return true;
    if (target.closest('[data-modal="true"]')) return true;
    if (target.closest('[role="dialog"]')) return true;
    return false;
  }

  function shouldIgnoreShortcut(target: HTMLElement | null): boolean {
    if (!target) return false;
    if (target.isContentEditable) return true;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
  }

  async function reopenLastClosed() {
    const popped = tabsStore.popRecentlyClosed();
    if (!popped) return;
    const exists = tabsStore.leafIds.includes(popped.wndId);
    const wndId = exists ? popped.wndId : (tabsStore.activeWndId ?? tabsStore.leafIds[0] ?? 1);
    if (popped.viewKind === "editor" && popped.pageId == null) return;
    await tabsStore.openTab({
      wndId,
      pageId: popped.pageId,
      viewKind: popped.viewKind,
      activate: true,
    });
  }

  function dispatchNewNotebook() {
    window.dispatchEvent(new CustomEvent("study:notes:notebook-new"));
  }

  async function handleShortcut(e: KeyboardEvent) {
    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) return;
    if (shouldIgnoreShortcut(e.target as HTMLElement | null)) return;
    const wndId = tabsStore.activeWndId;

    if (e.shiftKey && (e.key === "n" || e.key === "N") && !e.altKey) {
      e.preventDefault();
      dispatchNewNotebook();
      return;
    }
    if (e.altKey && /^[1-9]$/.test(e.key) && !e.shiftKey) {
      e.preventDefault();
      const ok = notebooksStore.switchToIndex(parseInt(e.key, 10) - 1);
      if (!ok) {
        /* no-op when index out of range */
      }
      return;
    }

    if (e.key === "t" && !e.shiftKey && !e.altKey) {
      e.preventDefault();
      const trigger = document.querySelector<HTMLButtonElement>(
        `[data-wnd-id="${wndId}"] .add`,
      );
      trigger?.click();
      return;
    }
    if (e.key === "w" && !e.shiftKey && !e.altKey) {
      e.preventDefault();
      const tabId = tabsStore.activeTabId;
      if (tabId != null) await tabsStore.closeTab(tabId);
      return;
    }
    if (e.key === "T" && e.shiftKey) {
      e.preventDefault();
      await reopenLastClosed();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      tabsStore.cycleActiveTab(e.shiftKey ? -1 : 1);
      return;
    }
    if (e.key === "\\" && !e.shiftKey && wndId != null) {
      e.preventDefault();
      await tabsStore.splitWnd(wndId, "vertical");
      return;
    }
    if (e.key === "|" && wndId != null) {
      e.preventDefault();
      await tabsStore.splitWnd(wndId, "horizontal");
      return;
    }
    if (/^[1-9]$/.test(e.key) && !e.shiftKey && !e.altKey) {
      e.preventDefault();
      tabsStore.activateTabIndex(parseInt(e.key, 10) - 1);
      return;
    }
    if (e.altKey && wndId != null) {
      let dir: "left" | "right" | "up" | "down" | null = null;
      if (e.key === "ArrowLeft") dir = "left";
      else if (e.key === "ArrowRight") dir = "right";
      else if (e.key === "ArrowUp") dir = "up";
      else if (e.key === "ArrowDown") dir = "down";
      if (dir) {
        e.preventDefault();
        tabsStore.focusNeighborWnd(dir);
      }
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
      if (!shouldIgnoreEsc(e.target as HTMLElement | null)) {
        e.preventDefault();
        exitFullscreen();
        return;
      }
    }
    void handleShortcut(e);
  }

  onMount(() => {
    notesShell.loadLocal();
    void notesShell.hydrateFromBackend();
    void tabsStore.hydrate();
    void docksStore.hydrate();
    void notebooksStore.hydrate();

    if (typeof window !== "undefined") {
      try {
        const referrer = document.referrer;
        if (referrer && !referrer.includes("/study/notes")) {
          const url = new URL(referrer, window.location.origin);
          if (url.origin === window.location.origin) {
            notesShell.rememberReturnUrl(url.pathname + url.search);
          }
        }
      } catch {
        /* ignore */
      }
    }

    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });

  const leftHasVisible = $derived(docksStore.visibleByPosition("left").length > 0);
  const rightHasVisible = $derived(docksStore.visibleByPosition("right").length > 0);
  const dockLeftPx = $derived(
    notesShell.dockLeftVisible && leftHasVisible ? `${notesShell.dockLeftWidth}px` : "0px",
  );
  const dockRightPx = $derived(
    notesShell.dockRightVisible && rightHasVisible ? `${notesShell.dockRightWidth}px` : "0px",
  );
  const tree = $derived(tabsStore.tree);
  const hasSplits = $derived(!!tree && tree.split_dir !== null && tree.split_dir !== undefined);
</script>

<div
  class="nb-shell"
  style:--nb-dock-left={dockLeftPx}
  style:--nb-dock-right={dockRightPx}
>
  <div class="nb-rail nb-rail-left">
    <NbDockSidebar side="left" />
  </div>

  <div class="nb-dock-left">
    <NbDockLeft />
  </div>

  <main class="nb-main">
    <button
      type="button"
      class="exit-btn"
      onclick={exitFullscreen}
      aria-label="Sair do modo fullscreen"
      title="Sair (Esc)"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
      <span class="esc-hint">ESC</span>
    </button>

    {#snippet activeContentSnippet()}
      <div class="nb-route-host">
        {@render children?.()}
      </div>
    {/snippet}
    {#if tree}
      <div class="nb-tree" class:has-splits={hasSplits}>
        <NbWndTree node={tree} activeContent={activeContentSnippet} />
      </div>
    {:else}
      <div class="nb-route-host">
        {@render children?.()}
      </div>
    {/if}
  </main>

  <div class="nb-dock-right">
    <NbDockRight />
  </div>

  <div class="nb-rail nb-rail-right">
    <NbDockSidebar side="right" />
  </div>

  <div class="nb-status">
    <NbStatusBar />
  </div>
</div>

<style>
  .nb-shell {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    grid-template-columns: 32px var(--nb-dock-left, 240px) 1fr var(--nb-dock-right, 280px) 32px;
    grid-template-rows: 1fr 24px;
    grid-template-areas:
      "rail-left dock-left main dock-right rail-right"
      "statusbar statusbar statusbar statusbar statusbar";
    background:
      radial-gradient(ellipse at 30% 0%, color-mix(in oklab, var(--accent) 18%, transparent) 0%, transparent 60%),
      radial-gradient(ellipse at 80% 100%, color-mix(in oklab, var(--accent) 8%, transparent) 0%, transparent 70%),
      var(--primary, var(--page-bg));
    color: var(--secondary, var(--text));
    transition: background 600ms ease;
  }

  .nb-rail {
    overflow: hidden;
  }
  .nb-rail-left {
    grid-area: rail-left;
  }
  .nb-rail-right {
    grid-area: rail-right;
  }

  .nb-dock-left {
    grid-area: dock-left;
    border-right: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    overflow: hidden;
    min-width: 0;
  }

  .nb-dock-right {
    grid-area: dock-right;
    border-left: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    overflow: hidden;
    min-width: 0;
  }

  .nb-main {
    grid-area: main;
    position: relative;
    overflow: hidden;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .nb-tree {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 0;
  }
  .nb-tree.has-splits {
    padding: 4px;
  }
  .nb-route-host {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .nb-status {
    grid-area: statusbar;
    border-top: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
  }

  .exit-btn {
    position: absolute;
    top: 10px;
    right: 16px;
    z-index: 5;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 999px;
    color: var(--tertiary, var(--muted-fg));
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .exit-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, var(--button));
  }
  .esc-hint {
    font-family: ui-monospace, monospace;
    font-size: 10px;
    padding: 1px 5px;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 4px;
    color: var(--secondary, var(--text));
    letter-spacing: 0.04em;
  }

  @media (max-width: 760px) {
    .nb-shell {
      grid-template-columns: 1fr;
      grid-template-areas:
        "main"
        "statusbar";
    }
    .nb-dock-left,
    .nb-dock-right,
    .nb-rail {
      display: none;
    }
  }
</style>
