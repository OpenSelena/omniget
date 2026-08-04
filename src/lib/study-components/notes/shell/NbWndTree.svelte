<script lang="ts">
  import type { WndNode } from "$lib/notes-bridge";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import NbTabStrip from "./NbTabStrip.svelte";
  import NbSplitHandle from "./NbSplitHandle.svelte";
  import NbWorkspace from "./NbWorkspace.svelte";
  import Self from "./NbWndTree.svelte";
  import { type Snippet } from "svelte";

  type Props = {
    node: WndNode;
    activeContent?: Snippet<[]>;
  };

  let { node, activeContent }: Props = $props();

  const activeWndId = $derived(tabsStore.activeWndId);
</script>

{#if node.split_dir === null || node.split_dir === undefined}
  <div
    class="nb-wnd-leaf"
    class:active={activeWndId === node.id}
    role="region"
    onpointerdown={() => tabsStore.setActiveWnd(node.id)}
  >
    <div class="leaf-tabs">
      <NbTabStrip wndId={node.id} />
    </div>
    <div class="leaf-body">
      <NbWorkspace wndId={node.id} {activeContent} />
    </div>
  </div>
{:else if node.children.length === 2}
  {@const dir = node.split_dir}
  {@const r = Math.max(0.05, Math.min(0.95, node.ratio))}
  {@const r1 = Math.round(r * 1000) / 10}
  {@const r2 = Math.round((1 - r) * 1000) / 10}
  <div class="nb-wnd-split" class:vertical={dir === "vertical"} class:horizontal={dir === "horizontal"}>
    <div
      class="split-pane"
      style:flex-basis="{r1}%"
    >
      <Self node={node.children[0]} {activeContent} />
    </div>
    <NbSplitHandle wndId={node.id} direction={dir} ratio={node.ratio} />
    <div
      class="split-pane"
      style:flex-basis="{r2}%"
    >
      <Self node={node.children[1]} {activeContent} />
    </div>
  </div>
{:else}
  <div class="nb-wnd-empty">Layout inválido</div>
{/if}

<style>
  .nb-wnd-leaf {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
    border: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
    border-radius: 4px;
    background: var(--primary, var(--page-bg));
    transition: border-color 120ms ease;
  }
  .nb-wnd-leaf.active {
    border-color: color-mix(in oklab, var(--accent) 70%, var(--content-border));
  }
  .leaf-tabs {
    height: 32px;
    flex-shrink: 0;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
    overflow: hidden;
  }
  .leaf-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .nb-wnd-split {
    position: relative;
    display: flex;
    width: 100%;
    height: 100%;
  }
  .nb-wnd-split.vertical {
    flex-direction: row;
  }
  .nb-wnd-split.horizontal {
    flex-direction: column;
  }
  .split-pane {
    flex-grow: 1;
    flex-shrink: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 2px;
  }
  .nb-wnd-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--tertiary, var(--muted-fg));
  }
</style>
