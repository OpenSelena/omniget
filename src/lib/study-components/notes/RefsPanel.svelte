<script lang="ts">
  import { t } from "$lib/i18n";
  import {
    notesBlocksPropertyListKeys,
    type LinkedRef,
    type PropertyKeyStat,
  } from "$lib/notes-bridge";

  type Props = {
    linkedRefs: LinkedRef[];
    unlinkedRefs: LinkedRef[];
    loading: boolean;
    active: boolean;
    pageId: number | null;
    onPageNavigate: (name: string) => void;
  };

  let { linkedRefs, unlinkedRefs, loading, active, pageId, onPageNavigate }: Props = $props();

  type Tab = "linked" | "unlinked" | "props";
  let tab = $state<Tab>("linked");

  let propsLoading = $state(false);
  let propsList = $state<PropertyKeyStat[]>([]);

  const RESERVED_KEYS = ["status", "deadline", "timestamp", "tags"];

  async function loadProps() {
    if (!pageId) {
      propsList = [];
      return;
    }
    propsLoading = true;
    try {
      const all = await notesBlocksPropertyListKeys({ pageId });
      propsList = all;
    } catch (e) {
      console.error("loadProps failed", e);
      propsList = [];
    } finally {
      propsLoading = false;
    }
  }

  $effect(() => {
    if (active && tab === "props" && pageId !== null) {
      void loadProps();
    }
  });

  $effect(() => {
    if (pageId !== null && tab === "props") {
      void loadProps();
    }
  });
</script>

<aside class="refs-panel">
  {#if active}
    <nav class="tabs" role="tablist" aria-label="Refs e propriedades">
      <button
        type="button"
        class="tab"
        class:active={tab === "linked"}
        role="tab"
        aria-selected={tab === "linked"}
        onclick={() => (tab = "linked")}
      >
        Linked <span class="tab-count">{linkedRefs.length}</span>
      </button>
      <button
        type="button"
        class="tab"
        class:active={tab === "unlinked"}
        role="tab"
        aria-selected={tab === "unlinked"}
        onclick={() => (tab = "unlinked")}
      >
        Unlinked <span class="tab-count">{unlinkedRefs.length}</span>
      </button>
      <button
        type="button"
        class="tab"
        class:active={tab === "props"}
        role="tab"
        aria-selected={tab === "props"}
        onclick={() => (tab = "props")}
      >
        Props
      </button>
    </nav>

    {#if tab === "linked"}
      {#if loading}
        <div class="state-sm">Carregando…</div>
      {:else if linkedRefs.length === 0}
        <p class="empty">{$t("study.library.notes_no_references")}</p>
      {:else}
        <ul class="ref-list">
          {#each linkedRefs as r (r.block_id)}
            <li>
              <button
                class="ref-row"
                onclick={() => onPageNavigate(r.page_name)}
              >
                <span class="ref-page">{r.page_name}</span>
                <span class="ref-snippet">{r.snippet}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if tab === "unlinked"}
      {#if unlinkedRefs.length === 0}
        <p class="empty">Nada por aqui.</p>
      {:else}
        <ul class="ref-list">
          {#each unlinkedRefs.slice(0, 30) as r (r.block_id)}
            <li>
              <button
                class="ref-row subtle"
                onclick={() => onPageNavigate(r.page_name)}
              >
                <span class="ref-page">{r.page_name}</span>
                <span class="ref-snippet">{r.snippet}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if tab === "props"}
      {#if propsLoading}
        <div class="state-sm">Carregando propriedades…</div>
      {:else if propsList.length === 0}
        <p class="empty">Sem propriedades nesta página.</p>
      {:else}
        <ul class="prop-list">
          {#each propsList as p (p.key)}
            <li class="prop-row">
              <span class="prop-key" class:reserved={RESERVED_KEYS.includes(p.key)}>
                {p.key}
              </span>
              <span class="prop-stats">
                {p.block_count} bloco{p.block_count === 1 ? "" : "s"} · {p.distinct_values} valor{p.distinct_values === 1 ? "" : "es"}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</aside>

<style>
  .refs-panel {
    border-left: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    padding: 12px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    padding-bottom: 4px;
  }
  .tab {
    flex: 1;
    padding: 6px 8px;
    border: 0;
    background: transparent;
    color: var(--tertiary);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 120ms ease, border-color 120ms ease;
    border-radius: var(--border-radius) var(--border-radius) 0 0;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab-count {
    display: inline-block;
    margin-left: 4px;
    padding: 1px 6px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--accent);
    border-radius: 999px;
    font-size: 10px;
    text-transform: none;
    font-weight: 500;
  }
  .ref-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ref-row {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--input-border) 18%, transparent);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background 150ms ease;
  }
  .ref-row:hover {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .ref-row.subtle {
    background: transparent;
    border: 1px dashed color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .ref-page {
    font-size: 11px;
    color: var(--accent);
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .ref-snippet {
    font-size: 12px;
    color: var(--secondary);
    line-height: 1.4;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
  .prop-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .prop-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--input-border) 14%, transparent);
  }
  .prop-key {
    font-size: 12px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--text);
    font-weight: 500;
  }
  .prop-key.reserved {
    color: var(--accent);
  }
  .prop-stats {
    font-size: 10px;
    color: var(--tertiary);
  }
  .state-sm {
    color: var(--tertiary);
    font-size: 12px;
  }
  .empty {
    color: var(--tertiary);
    font-size: 12px;
    padding: 8px 0;
    margin: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .ref-row,
    .tab {
      transition: none;
    }
  }
</style>
