<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import { notesPagesOutline, type OutlineEntry } from "$lib/notes-bridge";

  let entries = $state<OutlineEntry[]>([]);
  let loading = $state(false);
  let lastPageId = $state<number | null>(null);

  const pageId = $derived(tabsStore.activeTab?.page_id ?? null);

  async function reload(id: number) {
    loading = true;
    try {
      entries = await notesPagesOutline(id);
    } catch {
      entries = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = pageId;
    if (id === lastPageId) return;
    untrack(() => {
      lastPageId = id;
      if (id == null) {
        entries = [];
        return;
      }
      void reload(id);
    });
  });

  function jumpTo(blockId: number) {
    const el = document.querySelector<HTMLElement>(`[data-block-id="${blockId}"]`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
      el.classList.add("nb-flash");
      setTimeout(() => el.classList.remove("nb-flash"), 800);
    }
  }

  onMount(() => {
    if (pageId != null) void reload(pageId);
  });
</script>

<aside class="nb-dock">
  <header class="dock-head">
    <span class="dock-title">Outline</span>
  </header>
  <div class="body">
    {#if pageId == null}
      <p class="empty">Sem página ativa.</p>
    {:else if loading}
      <p class="empty">Carregando…</p>
    {:else if entries.length === 0}
      <p class="empty">Sem cabeçalhos nesta página.</p>
    {:else}
      <ul class="entries">
        {#each entries as entry (entry.block_id)}
          <li class="entry" style:--lvl={entry.level}>
            <button
              type="button"
              class="entry-btn"
              onclick={() => jumpTo(entry.block_id)}
              title={entry.content}
            >
              <span class="content">{entry.content || "(vazio)"}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</aside>

<style>
  .nb-dock {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .dock-head {
    padding: 8px 12px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary, var(--muted-fg));
  }
  .dock-title {
    font-weight: 600;
    color: var(--secondary, var(--text));
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 6px 4px;
  }
  .empty {
    padding: 14px 14px;
    margin: 0;
    color: var(--tertiary, var(--muted-fg));
    font-size: 12px;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .entry {
    --lvl: 1;
  }
  .entry-btn {
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    color: var(--secondary, var(--text));
    font: inherit;
    font-size: 12px;
    line-height: 1.4;
    padding: 4px 8px 4px calc(8px + (var(--lvl) - 1) * 14px);
    cursor: pointer;
    border-radius: 4px;
    display: block;
  }
  .entry-btn:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }
  .content {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }
  :global(.nb-flash) {
    animation: nb-flash 800ms ease-out;
  }
  @keyframes nb-flash {
    0% {
      background: color-mix(in oklab, var(--accent) 30%, transparent);
    }
    100% {
      background: transparent;
    }
  }
</style>
