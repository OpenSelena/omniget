<script lang="ts">
  import { onMount } from "svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import { notesInboxList, type PageSummary } from "$lib/notes-bridge";

  let pages = $state<PageSummary[]>([]);
  let loading = $state(false);

  async function reload() {
    loading = true;
    try {
      pages = await notesInboxList();
    } catch {
      pages = [];
    } finally {
      loading = false;
    }
  }

  async function open(pageId: number) {
    const wnd = tabsStore.activeWndId ?? tabsStore.leafIds[0];
    if (wnd == null) return;
    await tabsStore.openTab({ wndId: wnd, pageId, viewKind: "editor", activate: true });
  }

  onMount(() => {
    void reload();
  });
</script>

<aside class="nb-dock">
  <header class="dock-head">
    <span class="dock-title">Inbox</span>
    <button class="refresh" type="button" onclick={() => void reload()} title="Recarregar" aria-label="Recarregar">
      ↻
    </button>
  </header>
  <div class="body">
    {#if loading && pages.length === 0}
      <p class="empty">Carregando…</p>
    {:else if pages.length === 0}
      <p class="empty">Nada para triar — caixa vazia.</p>
    {:else}
      <ul class="entries">
        {#each pages as p (p.id)}
          <li>
            <button type="button" class="entry-btn" onclick={() => open(p.id)} title={p.name}>
              <span class="title">{p.title || p.name}</span>
              <span class="sub">{p.block_count} {p.block_count === 1 ? "bloco" : "blocos"}</span>
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
    display: flex;
    align-items: center;
    justify-content: space-between;
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
  .refresh {
    background: transparent;
    border: 0;
    color: inherit;
    cursor: pointer;
    font-size: 13px;
    padding: 0 4px;
    border-radius: 4px;
  }
  .refresh:hover {
    color: var(--accent);
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 6px 4px;
  }
  .empty {
    padding: 14px;
    margin: 0;
    color: var(--tertiary, var(--muted-fg));
    font-size: 12px;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .entry-btn {
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    color: var(--secondary, var(--text));
    font: inherit;
    font-size: 12px;
    line-height: 1.35;
    padding: 5px 10px;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .entry-btn:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 10px;
    opacity: 0.6;
  }
</style>
