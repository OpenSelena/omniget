<script lang="ts">
  import { onMount } from "svelte";
  import { tabsStore } from "$lib/study-notes/tabs-store.svelte";
  import {
    notesTagsList,
    notesPagesListByTag,
    type TagSummary,
    type PageSummary,
  } from "$lib/notes-bridge";

  let tags = $state<TagSummary[]>([]);
  let loading = $state(false);
  let activeTag = $state<string | null>(null);
  let activeTagPages = $state<PageSummary[]>([]);
  let activeLoading = $state(false);

  async function reload() {
    loading = true;
    try {
      tags = await notesTagsList(80);
    } catch {
      tags = [];
    } finally {
      loading = false;
    }
  }

  async function pickTag(name: string) {
    if (activeTag === name) {
      activeTag = null;
      activeTagPages = [];
      return;
    }
    activeTag = name;
    activeLoading = true;
    try {
      activeTagPages = await notesPagesListByTag(name);
    } catch {
      activeTagPages = [];
    } finally {
      activeLoading = false;
    }
  }

  async function openPage(pageId: number) {
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
    <span class="dock-title">Tags</span>
    <button class="refresh" type="button" onclick={() => void reload()} title="Recarregar" aria-label="Recarregar">
      ↻
    </button>
  </header>
  <div class="body">
    {#if loading && tags.length === 0}
      <p class="empty">Carregando…</p>
    {:else if tags.length === 0}
      <p class="empty">Nenhuma tag.</p>
    {:else}
      <ul class="tag-list">
        {#each tags as tag (tag.name)}
          <li>
            <button
              type="button"
              class="tag"
              class:active={activeTag === tag.name}
              onclick={() => pickTag(tag.name)}
            >
              <span class="hash">#</span>
              <span class="name">{tag.name}</span>
              <span class="count">{tag.ref_count}</span>
            </button>
          </li>
        {/each}
      </ul>

      {#if activeTag}
        <div class="tag-pages">
          <header class="sub-head">
            Páginas com #{activeTag}
            {#if activeLoading}<span class="loading">…</span>{/if}
          </header>
          {#if activeTagPages.length === 0 && !activeLoading}
            <p class="empty">Nenhuma página.</p>
          {:else}
            <ul class="entries">
              {#each activeTagPages as p (p.id)}
                <li>
                  <button type="button" class="entry-btn" onclick={() => openPage(p.id)}>
                    {p.title || p.name}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
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
  .tag-list {
    list-style: none;
    margin: 0;
    padding: 4px;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: color-mix(in oklab, var(--content-border) 25%, transparent);
    border: 1px solid transparent;
    color: var(--secondary, var(--text));
    font: inherit;
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 999px;
    cursor: pointer;
  }
  .tag:hover {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .tag.active {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    border-color: color-mix(in oklab, var(--accent) 50%, transparent);
  }
  .hash {
    opacity: 0.55;
  }
  .count {
    font-size: 9.5px;
    opacity: 0.7;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    padding: 0 5px;
    border-radius: 999px;
  }
  .tag-pages {
    margin-top: 6px;
    border-top: 1px solid color-mix(in oklab, var(--content-border) 25%, transparent);
    padding-top: 6px;
  }
  .sub-head {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 6px 10px 2px;
    color: var(--tertiary, var(--muted-fg));
  }
  .loading {
    font-style: italic;
    margin-left: 4px;
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
    padding: 4px 10px;
    cursor: pointer;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-btn:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }
</style>
