<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import { notesGraph, type Graph, type GraphNode } from "$lib/notes-bridge";

  let graph = $state<Graph | null>(null);
  let loading = $state(false);
  let lastPageId = $state<number | null>(null);

  const pageId = $derived(tabsStore.activeTab?.page_id ?? null);

  type Neighbor = { node: GraphNode; weight: number };

  const neighbors = $derived.by<Neighbor[]>(() => {
    if (!graph || pageId == null) return [];
    const nodeById = new Map(graph.nodes.map((n) => [n.id, n]));
    const out: Neighbor[] = [];
    for (const e of graph.edges) {
      if (e.from === pageId && e.to !== pageId) {
        const n = nodeById.get(e.to);
        if (n) out.push({ node: n, weight: e.weight });
      } else if (e.to === pageId && e.from !== pageId) {
        const n = nodeById.get(e.from);
        if (n) out.push({ node: n, weight: e.weight });
      }
    }
    out.sort((a, b) => b.weight - a.weight);
    return out;
  });

  const center = $derived.by<GraphNode | null>(() => {
    if (!graph || pageId == null) return null;
    return graph.nodes.find((n) => n.id === pageId) ?? null;
  });

  async function reload() {
    loading = true;
    try {
      graph = await notesGraph();
    } catch {
      graph = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = pageId;
    if (id === lastPageId) return;
    untrack(() => {
      lastPageId = id;
      if (!graph) void reload();
    });
  });

  async function open(pid: number) {
    const wnd = tabsStore.activeWndId ?? tabsStore.leafIds[0];
    if (wnd == null) return;
    await tabsStore.openTab({ wndId: wnd, pageId: pid, viewKind: "editor", activate: true });
  }

  onMount(() => {
    void reload();
  });
</script>

<aside class="nb-dock">
  <header class="dock-head">
    <span class="dock-title">Graph</span>
    <button class="refresh" type="button" onclick={() => void reload()} title="Recarregar" aria-label="Recarregar">
      ↻
    </button>
  </header>
  <div class="body">
    {#if loading && !graph}
      <p class="empty">Construindo graph…</p>
    {:else if pageId == null}
      <p class="empty">Sem página ativa.</p>
    {:else if !center}
      <p class="empty">Página não está no grafo.</p>
    {:else}
      <div class="center">
        <span class="dot" aria-hidden="true"></span>
        <span class="label">{center.name}</span>
        <span class="meta">{center.ref_count} refs</span>
      </div>
      {#if neighbors.length === 0}
        <p class="empty">Sem links 1-hop.</p>
      {:else}
        <ul class="neighbors">
          {#each neighbors as n (n.node.id)}
            <li>
              <button type="button" class="entry-btn" onclick={() => open(n.node.id)}>
                <span class="weight" aria-hidden="true" style:--w={Math.min(n.weight, 5)}>·</span>
                <span class="name">{n.node.name}</span>
                <span class="w">×{n.weight}</span>
              </button>
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
  .center {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    margin: 4px;
    border-radius: 6px;
    background: color-mix(in oklab, var(--accent) 15%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
    font-size: 12px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text);
  }
  .meta {
    font-size: 10px;
    color: var(--tertiary, var(--muted-fg));
  }
  .neighbors {
    list-style: none;
    margin: 0;
    padding: 4px 0;
  }
  .entry-btn {
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    color: var(--secondary, var(--text));
    font: inherit;
    font-size: 12px;
    padding: 4px 10px;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .entry-btn:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }
  .weight {
    color: color-mix(in oklab, var(--accent) calc(40% + var(--w) * 12%), transparent);
    font-size: 16px;
    line-height: 0.5;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .w {
    font-size: 10px;
    opacity: 0.6;
  }
</style>
