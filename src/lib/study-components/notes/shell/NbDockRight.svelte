<script lang="ts">
  import { docksStore } from "$lib/study-notes/docks-store.svelte";
  import NbResizeHandle from "./NbResizeHandle.svelte";
  import NbDockBacklink from "./NbDockBacklink.svelte";
  import NbDockBookmark from "./NbDockBookmark.svelte";
  import NbDockTag from "./NbDockTag.svelte";
  import NbDockInbox from "./NbDockInbox.svelte";
  import NbDockGraph from "./NbDockGraph.svelte";

  const visible = $derived(docksStore.visibleByPosition("right"));
</script>

{#if visible.length > 0}
  <aside class="nb-dock nb-dock-right" data-count={visible.length}>
    <NbResizeHandle side="right" />
    <div class="stack" class:multi={visible.length > 1}>
      {#each visible as dock (dock.dock_id)}
        <section class="slot" data-id={dock.dock_id}>
          {#if dock.dock_id === "backlink"}
            <NbDockBacklink />
          {:else if dock.dock_id === "bookmark"}
            <NbDockBookmark />
          {:else if dock.dock_id === "tag"}
            <NbDockTag />
          {:else if dock.dock_id === "inbox"}
            <NbDockInbox />
          {:else if dock.dock_id === "graph"}
            <NbDockGraph />
          {:else}
            <div class="placeholder">
              <p class="hint">{dock.dock_id}</p>
              <p class="sub">Dock sem renderer dedicado.</p>
            </div>
          {/if}
        </section>
      {/each}
    </div>
  </aside>
{/if}

<style>
  .nb-dock {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: color-mix(in oklab, var(--surface-bg, var(--primary)) 70%, transparent);
    overflow: hidden;
  }
  .stack {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .stack.multi .slot:not(:last-child) {
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
  }
  .stack.multi .slot {
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
  }
  .stack:not(.multi) .slot {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .placeholder {
    padding: 20px 16px;
    color: var(--tertiary, var(--muted-fg));
  }
  .hint {
    margin: 0 0 4px;
    font-size: 13px;
    color: var(--secondary, var(--text));
  }
  .sub {
    margin: 0;
    font-size: 11px;
    opacity: 0.75;
  }
</style>
