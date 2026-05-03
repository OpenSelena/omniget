<script lang="ts">
  import { onMount } from "svelte";
  import { getDownloads } from "$lib/stores/download-store.svelte";

  let lastNonZero = $state(0);
  let now = $state(Date.now());

  const downloads = $derived(getDownloads());
  const totalDownSpeed = $derived.by(() => {
    let sum = 0;
    for (const item of downloads.values()) {
      if (item.kind === "generic" && (item.status === "downloading" || item.status === "seeding")) {
        sum += item.speed;
      } else if (item.kind === "course" && item.status === "downloading") {
        sum += item.speed;
      }
    }
    return sum;
  });

  $effect(() => {
    if (totalDownSpeed > 0) {
      lastNonZero = Date.now();
    }
  });

  const visible = $derived.by(() => {
    if (totalDownSpeed > 0) return true;
    return now - lastNonZero < 3000;
  });

  function fmtRate(bps: number): string {
    if (bps < 1024) return `${bps.toFixed(0)} B/s`;
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
    if (bps < 1024 * 1024 * 1024) return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
    return `${(bps / (1024 * 1024 * 1024)).toFixed(2)} GB/s`;
  }

  onMount(() => {
    const handle = setInterval(() => {
      now = Date.now();
    }, 500);
    return () => clearInterval(handle);
  });
</script>

{#if visible}
  <div class="bandwidth-pill" class:fading={totalDownSpeed === 0} aria-live="polite">
    <span class="arrow" aria-hidden="true">↓</span>
    <span class="rate">{fmtRate(totalDownSpeed)}</span>
  </div>
{/if}

<style>
  .bandwidth-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 15%, transparent);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    border: 1px solid color-mix(in oklab, var(--accent) 30%, transparent);
    transition: opacity 250ms ease;
  }
  .bandwidth-pill.fading {
    opacity: 0.5;
  }
  .arrow {
    font-size: 12px;
    opacity: 0.85;
  }
  @media (prefers-reduced-motion: reduce) {
    .bandwidth-pill {
      transition: none;
    }
  }
</style>
