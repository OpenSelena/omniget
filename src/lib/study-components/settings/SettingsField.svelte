<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    label: string;
    description?: string;
    valueDisplay?: string;
    children?: Snippet;
  };

  let { label, description, valueDisplay, children }: Props = $props();
</script>

<div class="field">
  <div class="text">
    <span class="label">{label}</span>
    {#if description}
      <span class="desc">{description}</span>
    {/if}
  </div>
  <div class="control">
    {@render children?.()}
    {#if valueDisplay}
      <span class="value">{valueDisplay}</span>
    {/if}
  </div>
</div>

<style>
  .field {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
  }

  .field:last-child {
    border-bottom: none;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .label {
    font-size: 14px;
    font-weight: 500;
  }

  .desc {
    font-size: 12px;
    color: color-mix(in oklab, currentColor 60%, transparent);
    line-height: 1.4;
  }

  .control {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 0 0 auto;
  }

  .value {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: color-mix(in oklab, currentColor 70%, transparent);
    min-width: 40px;
    text-align: right;
  }
</style>
