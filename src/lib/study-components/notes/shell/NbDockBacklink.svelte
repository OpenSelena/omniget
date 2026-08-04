<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import {
    notesPagesLinkedRefs,
    notesPagesUnlinkedRefs,
    notesBlocksPropertyListKeys,
    type LinkedRef,
    type PropertyKeyStat,
  } from "$lib/notes-bridge";

  type Tab = "linked" | "unlinked" | "props";
  let tab = $state<Tab>("linked");

  let linked = $state<LinkedRef[]>([]);
  let unlinked = $state<LinkedRef[]>([]);
  let props = $state<PropertyKeyStat[]>([]);
  let loading = $state(false);
  let lastPageId = $state<number | null>(null);

  const pageId = $derived(tabsStore.activeTab?.page_id ?? null);

  async function reload(id: number) {
    loading = true;
    try {
      const [l, u, p] = await Promise.all([
        notesPagesLinkedRefs(id),
        notesPagesUnlinkedRefs(id),
        notesBlocksPropertyListKeys({ pageId: id }),
      ]);
      linked = l;
      unlinked = u;
      props = p;
    } catch {
      linked = [];
      unlinked = [];
      props = [];
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
        linked = [];
        unlinked = [];
        props = [];
        return;
      }
      void reload(id);
    });
  });

  async function openByPageId(pid: number) {
    const wnd = tabsStore.activeWndId ?? tabsStore.leafIds[0];
    if (wnd == null) return;
    await tabsStore.openTab({ wndId: wnd, pageId: pid, viewKind: "editor", activate: true });
  }

  onMount(() => {
    if (pageId != null) void reload(pageId);
  });
</script>

<aside class="nb-dock">
  <header class="dock-head">
    <span class="dock-title">Backlinks</span>
  </header>
  <nav class="tabs" role="tablist">
    <button
      type="button"
      class="tab"
      class:active={tab === "linked"}
      role="tab"
      aria-selected={tab === "linked"}
      onclick={() => (tab = "linked")}
    >
      Linked <span class="count">{linked.length}</span>
    </button>
    <button
      type="button"
      class="tab"
      class:active={tab === "unlinked"}
      role="tab"
      aria-selected={tab === "unlinked"}
      onclick={() => (tab = "unlinked")}
    >
      Unlinked <span class="count">{unlinked.length}</span>
    </button>
    <button
      type="button"
      class="tab"
      class:active={tab === "props"}
      role="tab"
      aria-selected={tab === "props"}
      onclick={() => (tab = "props")}
    >
      Props <span class="count">{props.length}</span>
    </button>
  </nav>
  <div class="body">
    {#if pageId == null}
      <p class="empty">Sem página ativa.</p>
    {:else if loading}
      <p class="empty">Carregando…</p>
    {:else if tab === "linked"}
      {#if linked.length === 0}
        <p class="empty">Nenhuma referência direta.</p>
      {:else}
        <ul class="refs">
          {#each linked as r (r.block_id)}
            <li class="ref">
              <button
                type="button"
                class="ref-page"
                onclick={() => openByPageId(r.page_id)}
                title={r.page_name}
              >
                {r.page_name}
              </button>
              <p class="snippet">{r.snippet}</p>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if tab === "unlinked"}
      {#if unlinked.length === 0}
        <p class="empty">Nenhuma menção sem link.</p>
      {:else}
        <ul class="refs">
          {#each unlinked as r (r.block_id)}
            <li class="ref">
              <button
                type="button"
                class="ref-page"
                onclick={() => openByPageId(r.page_id)}
                title={r.page_name}
              >
                {r.page_name}
              </button>
              <p class="snippet">{r.snippet}</p>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if tab === "props"}
      {#if props.length === 0}
        <p class="empty">Nenhuma propriedade nesta página.</p>
      {:else}
        <ul class="props">
          {#each props as p (p.key)}
            <li class="prop">
              <span class="key">{p.key}</span>
              <span class="meta">{p.block_count} blocos · {p.distinct_values} valores</span>
            </li>
          {/each}
        </ul>
      {/if}
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
  .tabs {
    display: flex;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 25%, transparent);
  }
  .tab {
    flex: 1;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    padding: 6px 4px;
    font: inherit;
    font-size: 11px;
    color: var(--tertiary, var(--muted-fg));
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .tab:hover {
    color: var(--secondary, var(--text));
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .count {
    font-size: 9px;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 999px;
    padding: 1px 5px;
    margin-left: 4px;
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
  .refs {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .ref {
    padding: 6px 10px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 18%, transparent);
  }
  .ref-page {
    background: transparent;
    border: 0;
    color: var(--accent);
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .ref-page:hover {
    text-decoration: underline;
  }
  .snippet {
    margin: 3px 0 0;
    font-size: 11px;
    line-height: 1.4;
    color: var(--secondary, var(--text));
    opacity: 0.8;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .props {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .prop {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 6px 10px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 18%, transparent);
    gap: 8px;
  }
  .key {
    font-size: 12px;
    font-weight: 600;
    color: var(--secondary, var(--text));
  }
  .meta {
    font-size: 10px;
    color: var(--tertiary, var(--muted-fg));
  }
</style>
