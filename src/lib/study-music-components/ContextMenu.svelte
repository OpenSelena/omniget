<script lang="ts">
  export type ContextMenuItem = {
    id: string;
    label: string;
    icon?: string;
    shortcut?: string;
    danger?: boolean;
    separator?: boolean;
    disabled?: boolean;
  };

  type Props = {
    open: boolean;
    x: number;
    y: number;
    items: ContextMenuItem[];
    onAction: (id: string) => void;
    onClose: () => void;
  };

  let { open, x, y, items, onAction, onClose }: Props = $props();

  let menuEl = $state<HTMLDivElement | null>(null);

  const adjustedPos = $derived.by(() => {
    if (!open || typeof window === "undefined") return { left: x, top: y };
    const rect = menuEl?.getBoundingClientRect();
    const w = rect?.width ?? 220;
    const h = rect?.height ?? 200;
    let left = x;
    let top = y;
    if (left + w > window.innerWidth - 8) left = window.innerWidth - w - 8;
    if (top + h > window.innerHeight - 8) top = window.innerHeight - h - 8;
    if (left < 8) left = 8;
    if (top < 8) top = 8;
    return { left, top };
  });

  $effect(() => {
    if (!open || typeof document === "undefined") return;
    function handleClick(e: MouseEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  });
</script>

{#if open}
  <div
    bind:this={menuEl}
    class="ctx-menu"
    style="left: {adjustedPos.left}px; top: {adjustedPos.top}px;"
    role="menu"
    tabindex="-1"
  >
    {#each items as item (item.id)}
      {#if item.separator}
        <div class="separator" role="separator"></div>
      {:else}
        <button
          type="button"
          class="item"
          class:danger={item.danger}
          disabled={item.disabled}
          role="menuitem"
          onclick={() => {
            if (!item.disabled) {
              onAction(item.id);
              onClose();
            }
          }}
        >
          {#if item.icon}
            <span class="icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
                <path d={item.icon}></path>
              </svg>
            </span>
          {/if}
          <span class="label">{item.label}</span>
          {#if item.shortcut}
            <span class="shortcut">{item.shortcut}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx-menu {
    position: fixed;
    z-index: 350;
    min-width: 220px;
    padding: 4px;
    background: rgb(28, 28, 30);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    gap: 1px;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    background: transparent;
    border: 0;
    border-radius: 5px;
    color: inherit;
    font-family: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    width: 100%;
  }
  .item:hover:not(:disabled) {
    background: var(--accent);
    color: var(--on-accent, white);
  }
  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .item.danger {
    color: rgb(248, 113, 113);
  }
  .item.danger:hover:not(:disabled) {
    background: rgba(220, 38, 38, 0.85);
    color: white;
  }
  .icon {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    display: grid;
    place-items: center;
    color: inherit;
    opacity: 0.7;
  }
  .item:hover .icon { opacity: 1; }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .shortcut {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.4);
    font-family: ui-monospace, monospace;
  }
  .item:hover .shortcut { color: rgba(255, 255, 255, 0.8); }
  .separator {
    height: 1px;
    margin: 4px 6px;
    background: rgba(255, 255, 255, 0.08);
  }
</style>
