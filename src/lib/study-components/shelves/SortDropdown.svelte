<script lang="ts">
  import type { CatalogSort, CatalogSortDirection } from "$lib/study-bridge";

  type Option = {
    sort: CatalogSort;
    direction: CatalogSortDirection;
    label: string;
  };

  type Props = {
    value: CatalogSort;
    direction: CatalogSortDirection;
    onChange: (sort: CatalogSort, direction: CatalogSortDirection) => void;
  };

  let { value, direction, onChange }: Props = $props();
  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLDivElement | null>(null);

  const options: Option[] = [
    { sort: "last_watched", direction: "desc", label: "Recentemente assistidos" },
    { sort: "times_watched", direction: "desc", label: "Mais assistidos" },
    { sort: "name", direction: "asc", label: "Nome (A → Z)" },
    { sort: "name", direction: "desc", label: "Nome (Z → A)" },
    { sort: "progress", direction: "desc", label: "Maior progresso" },
    { sort: "progress", direction: "asc", label: "Menor progresso" },
    { sort: "added", direction: "desc", label: "Adicionado recentemente" },
    { sort: "platform", direction: "asc", label: "Plataforma" },
  ];

  const currentLabel = $derived.by(() => {
    const match = options.find((o) => o.sort === value && o.direction === direction);
    return match?.label ?? "Ordenar";
  });

  function pick(o: Option) {
    onChange(o.sort, o.direction);
    open = false;
    trigger?.focus();
  }

  function onDocClick(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    if (trigger?.contains(target) || menu?.contains(target)) return;
    open = false;
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      open = false;
      trigger?.focus();
    }
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener("click", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="sort">
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <span class="label">{currentLabel}</span>
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <polyline points="6 9 12 15 18 9" />
    </svg>
  </button>
  {#if open}
    <div class="menu" role="listbox" bind:this={menu}>
      {#each options as o (o.sort + o.direction)}
        {@const selected = o.sort === value && o.direction === direction}
        <button
          type="button"
          class="opt"
          class:selected
          role="option"
          aria-selected={selected}
          onclick={() => pick(o)}
        >
          <span class="check" aria-hidden="true">{selected ? "✓" : ""}</span>
          <span>{o.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sort {
    position: relative;
    display: inline-block;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 8px;
    color: inherit;
    font-size: 13px;
    cursor: pointer;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .trigger:hover {
    background: color-mix(in oklab, var(--accent) 10%, var(--content-bg) 90%);
  }

  .label {
    font-weight: 500;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 240px;
    background: var(--content-bg);
    border: 1px solid color-mix(in oklab, var(--content-border) 80%, transparent);
    border-radius: 10px;
    box-shadow: 0 8px 24px color-mix(in oklab, black 16%, transparent);
    padding: 4px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    width: 100%;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .opt:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }

  .opt.selected {
    color: var(--accent);
    font-weight: 600;
  }

  .check {
    width: 14px;
    color: var(--accent);
    text-align: center;
  }
</style>
