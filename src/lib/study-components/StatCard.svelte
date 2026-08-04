<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    value: string | number;
    hint?: string;
    trend?: "up" | "down" | "flat" | null;
    accent?: boolean;
    icon?: Snippet;
  }

  let {
    label,
    value,
    hint = "",
    trend = null,
    accent = false,
    icon,
  }: Props = $props();

  const trendChar = $derived(
    trend === "up" ? "▲" : trend === "down" ? "▼" : trend === "flat" ? "—" : ""
  );
</script>

<div class="stat-card" class:accent>
  {#if icon}
    <div class="icon">{@render icon()}</div>
  {/if}
  <span class="label">{label}</span>
  <strong class="value">{value}</strong>
  {#if hint || trend}
    <span class="hint">
      {#if trend}
        <span
          class="trend"
          class:up={trend === "up"}
          class:down={trend === "down"}
          aria-hidden="true"
        >{trendChar}</span>
      {/if}
      {hint}
    </span>
  {/if}
</div>

<style>
  .stat-card {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: calc(var(--padding, 10px) * 2);
    background: var(--button-elevated);
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius, 11px);
  }
  .icon {
    color: var(--accent);
    width: 20px;
    height: 20px;
    margin-bottom: 0.25rem;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .label {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 11px;
    color: var(--tertiary);
  }
  .value {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-variant-numeric: tabular-nums;
    font-size: 1.75rem;
    font-weight: 500;
    color: var(--secondary);
    line-height: 1.1;
  }
  .stat-card.accent .value {
    color: var(--accent);
  }
  .hint {
    font-size: 12px;
    color: var(--tertiary);
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .trend {
    font-size: 11px;
    color: var(--tertiary);
  }
  .trend.up {
    color: var(--success);
  }
  .trend.down {
    color: var(--error);
  }
</style>
