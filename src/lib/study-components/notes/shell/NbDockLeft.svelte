<script lang="ts">
  import { docksStore } from "$lib/study-notes/docks-store.svelte";
  import NbResizeHandle from "./NbResizeHandle.svelte";
  import NbDockFiles from "./NbDockFiles.svelte";
  import NbDockOutline from "./NbDockOutline.svelte";

  const visible = $derived(docksStore.visibleByPosition("left"));
</script>

{#if visible.length > 0}
  <aside class="nb-dock nb-dock-left" data-count={visible.length}>
    <div class="stack" class:multi={visible.length > 1}>
      {#each visible as dock (dock.dock_id)}
        <section class="slot" data-id={dock.dock_id}>
          {#if dock.dock_id === "files"}
            <NbDockFiles />
          {:else if dock.dock_id === "outline"}
            <NbDockOutline />
          {:else}
            <div class="placeholder">
              <p class="hint">{dock.dock_id}</p>
              <p class="sub">Dock sem renderer dedicado.</p>
            </div>
          {/if}
        </section>
      {/each}
    </div>
    <NbResizeHandle side="left" />
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
