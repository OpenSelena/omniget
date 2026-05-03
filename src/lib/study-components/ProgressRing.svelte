<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    value?: number;
    size?: number;
    stroke?: number;
    label?: string;
    center?: Snippet;
  }

  let {
    value = 0,
    size = 48,
    stroke = 4,
    label = "",
    center,
  }: Props = $props();

  const clamped = $derived(Math.max(0, Math.min(100, Number(value) || 0)));
  const r = $derived((size - stroke) / 2);
  const circumference = $derived(2 * Math.PI * r);
  const dashoffset = $derived(circumference * (1 - clamped / 100));
  const strokeColor = $derived(
    clamped >= 100
      ? "var(--success)"
      : clamped > 0
        ? "var(--accent)"
        : "var(--content-border)"
  );
  const labelSize = $derived(Math.max(10, Math.round(size * 0.22)));
</script>

<div class="ring" style="width: {size}px; height: {size}px;">
  <svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
    <circle
      cx={size / 2}
      cy={size / 2}
      r={r}
      fill="none"
      stroke="var(--content-border)"
      stroke-width={stroke}
    />
    <circle
      class="bar"
      cx={size / 2}
      cy={size / 2}
      r={r}
      fill="none"
      stroke={strokeColor}
      stroke-width={stroke}
      stroke-linecap="round"
      stroke-dasharray={circumference}
      stroke-dashoffset={dashoffset}
      transform="rotate(-90 {size / 2} {size / 2})"
    />
  </svg>
  {#if center}
    <div class="center">{@render center()}</div>
  {:else if label}
    <div class="center">
      <span class="label" style="font-size: {labelSize}px;">{label}</span>
    </div>
  {/if}
</div>

<style>
  .ring {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .bar {
    transition:
      stroke-dashoffset 500ms ease-out,
      stroke 600ms ease-out;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--secondary);
  }
  .label {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }
  @media (prefers-reduced-motion: reduce) {
    .bar {
      transition: none;
    }
  }
</style>
