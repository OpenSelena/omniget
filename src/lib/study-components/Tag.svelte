<script lang="ts">
  interface Props {
    label: string;
    color?: string;
    active?: boolean;
    removable?: boolean;
    onClick?: (e: Event) => void;
    onRemove?: (e: Event) => void;
  }

  let {
    label,
    color = "var(--accent)",
    active = false,
    removable = false,
    onClick,
    onRemove,
  }: Props = $props();

  function handleClick(e: Event) {
    if (onClick) onClick(e);
  }

  function handleKey(e: KeyboardEvent) {
    if (onClick && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      onClick(e);
    }
  }

  function handleRemove(e: Event) {
    e.stopPropagation();
    if (onRemove) onRemove(e);
  }
</script>

{#if onClick}
  <button
    type="button"
    class="tag clickable"
    class:active
    style="--tag-color: {color};"
    onclick={handleClick}
    onkeydown={handleKey}
  >
    <span class="label">{label}</span>
    {#if removable}
      <span
        class="x"
        role="button"
        tabindex="0"
        aria-label="Remove"
        onclick={handleRemove}
        onkeydown={(e) => (e.key === "Enter" || e.key === " ") && handleRemove(e)}
      >✕</span>
    {/if}
  </button>
{:else}
  <span class="tag" class:active style="--tag-color: {color};">
    <span class="label">{label}</span>
    {#if removable}
      <button type="button" class="x" aria-label="Remove" onclick={handleRemove}>✕</button>
    {/if}
  </span>
{/if}

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    color: var(--secondary);
    background: color-mix(in oklab, var(--tag-color) 8%, transparent);
    border: 1px solid color-mix(in oklab, var(--tag-color) 30%, var(--content-border));
    transition:
      background 150ms ease,
      border-color 150ms ease;
    line-height: 1.3;
    font-family: inherit;
  }
  button.tag {
    cursor: pointer;
  }
  .tag.clickable:hover {
    background: color-mix(in oklab, var(--tag-color) 15%, transparent);
    border-color: var(--tag-color);
  }
  .tag.active {
    background: color-mix(in oklab, var(--tag-color) 25%, transparent);
    border-color: var(--tag-color);
  }
  .tag:focus-visible {
    outline: var(--focus-ring, 2px solid var(--tag-color));
    outline-offset: 2px;
  }
  .x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border: 0;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 10px;
    padding: 0;
    border-radius: 50%;
    font-family: inherit;
    line-height: 1;
  }
  .x:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--tag-color) 20%, transparent);
  }
</style>
