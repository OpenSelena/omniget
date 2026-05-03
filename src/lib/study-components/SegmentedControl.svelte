<script lang="ts">
  import type { Snippet } from "svelte";

  interface Option {
    value: string;
    label: string;
    icon?: Snippet;
  }

  interface Props {
    options: Option[];
    value: string;
    ariaLabel?: string;
  }

  let { options = [], value = $bindable(""), ariaLabel = "" }: Props = $props();

  function handleKey(e: KeyboardEvent, i: number) {
    if (!options.length) return;
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = (i - 1 + options.length) % options.length;
      value = options[next].value;
    } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      const next = (i + 1) % options.length;
      value = options[next].value;
    } else if (e.key === "Home") {
      e.preventDefault();
      value = options[0].value;
    } else if (e.key === "End") {
      e.preventDefault();
      value = options[options.length - 1].value;
    }
  }
</script>

<div class="seg" role="tablist" aria-label={ariaLabel}>
  {#each options as opt, i (opt.value)}
    <button
      type="button"
      role="tab"
      aria-selected={value === opt.value}
      tabindex={value === opt.value ? 0 : -1}
      class:active={value === opt.value}
      onclick={() => (value = opt.value)}
      onkeydown={(e) => handleKey(e, i)}
    >
      {#if opt.icon}<span class="icon">{@render opt.icon()}</span>{/if}
      <span>{opt.label}</span>
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius, 11px);
    overflow: hidden;
    background: var(--primary);
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .seg::-webkit-scrollbar {
    display: none;
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.85rem;
    background: transparent;
    color: var(--tertiary);
    border: 0;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    white-space: nowrap;
    transition:
      background 150ms ease,
      color 150ms ease;
  }
  .seg button:hover:not(.active) {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    color: var(--secondary);
  }
  .seg button.active {
    background: var(--accent);
    color: var(--on-accent);
  }
  .seg button:focus-visible {
    outline: var(--focus-ring, 2px solid var(--accent));
    outline-offset: -2px;
  }
  .icon {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
