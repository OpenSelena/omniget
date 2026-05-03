<script lang="ts">
  import type { NotePage } from "$lib/notes-bridge";

  type Props = {
    page: NotePage | null;
    onSegmentClick: (path: string) => void;
  };

  let { page, onSegmentClick }: Props = $props();

  type Segment = { label: string; path: string };

  const segments = $derived.by<Segment[]>(() => {
    if (!page) return [];
    const parts = page.name.split("/").filter((s) => s.length > 0);
    const out: Segment[] = [];
    let acc = "";
    for (const part of parts) {
      acc = acc ? `${acc}/${part}` : part;
      out.push({ label: part, path: acc });
    }
    return out;
  });
</script>

{#if page}
  <nav class="breadcrumb" aria-label="Page path">
    {#each segments as seg, i (seg.path)}
      {#if i > 0}
        <span class="sep" aria-hidden="true">/</span>
      {/if}
      {#if i === segments.length - 1}
        <span class="seg current">{seg.label}</span>
      {:else}
        <button
          type="button"
          class="seg link"
          onclick={() => onSegmentClick(seg.path)}
          title="Ir para {seg.path}"
        >
          {seg.label}
        </button>
      {/if}
    {/each}
  </nav>
{/if}

<style>
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--tertiary);
    padding: 2px 0;
  }
  .sep {
    color: var(--tertiary);
    opacity: 0.6;
  }
  .seg {
    padding: 2px 4px;
    border-radius: 3px;
  }
  .seg.current {
    color: var(--secondary);
  }
  .seg.link {
    border: 0;
    background: transparent;
    cursor: pointer;
    color: var(--accent);
    font: inherit;
  }
  .seg.link:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    text-decoration: underline;
  }
</style>
