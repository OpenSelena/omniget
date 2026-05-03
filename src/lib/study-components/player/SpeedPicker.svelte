<script lang="ts">
  type Props = {
    value: number;
    onChange: (next: number) => void;
  };

  let { value, onChange }: Props = $props();
  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLDivElement | null>(null);

  const options = [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

  function pick(speed: number) {
    onChange(speed);
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
    if (open && e.key === "Escape") {
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

  function fmt(v: number): string {
    return Number.isInteger(v) ? `${v}×` : `${v}×`;
  }
</script>

<div class="speed">
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label="Velocidade de reprodução"
    onclick={() => (open = !open)}
  >
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="10" />
      <polyline points="12 6 12 12 15 13" />
    </svg>
    <span class="lbl">{fmt(value)}</span>
  </button>
  {#if open}
    <div class="menu" role="listbox" bind:this={menu}>
      {#each options as o (o)}
        {@const sel = Math.abs(o - value) < 0.001}
        <button
          type="button"
          role="option"
          class="opt"
          class:selected={sel}
          aria-selected={sel}
          onclick={() => pick(o)}
        >
          <span class="check">{sel ? "✓" : ""}</span>
          <span>{fmt(o)}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .speed {
    position: relative;
    display: inline-block;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 8px;
    color: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .trigger:hover {
    background: color-mix(in oklab, var(--accent) 10%, var(--content-bg) 90%);
  }

  .lbl {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .menu {
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    min-width: 100px;
    background: var(--content-bg);
    border: 1px solid color-mix(in oklab, var(--content-border) 80%, transparent);
    border-radius: 10px;
    box-shadow: 0 8px 24px color-mix(in oklab, black 30%, transparent);
    padding: 4px;
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    width: 100%;
    font-variant-numeric: tabular-nums;
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
    text-align: center;
    color: var(--accent);
    flex: 0 0 auto;
  }
</style>
