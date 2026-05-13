<script lang="ts">
  type Kind = "card" | "channel" | "row";
  type Props = { kind?: Kind; count?: number };
  let { kind = "card", count = 1 }: Props = $props();

  let total = $derived(Math.max(1, count));
</script>

{#each Array(total) as _, idx (idx)}
  {#if kind === "card"}
    <div class="skel-card" aria-hidden="true">
      <div class="thumb shimmer"></div>
      <div class="line shimmer wide"></div>
      <div class="line shimmer mid"></div>
    </div>
  {:else if kind === "channel"}
    <div class="skel-channel" aria-hidden="true">
      <div class="avatar shimmer"></div>
      <div class="line shimmer wide"></div>
      <div class="line shimmer narrow"></div>
    </div>
  {:else}
    <div class="skel-row" aria-hidden="true">
      <div class="row-thumb shimmer"></div>
      <div class="row-info">
        <div class="line shimmer wide"></div>
        <div class="line shimmer mid"></div>
      </div>
    </div>
  {/if}
{/each}

<style>
  .skel-card { display: flex; flex-direction: column; gap: 8px; }
  .skel-card .thumb { aspect-ratio: 16 / 9; border-radius: 8px; }
  .skel-channel { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 12px 8px; }
  .skel-channel .avatar { width: 96px; height: 96px; border-radius: 50%; }
  .skel-row {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 14px;
    align-items: center;
    padding: 8px;
    border-radius: 10px;
  }
  .skel-row .row-thumb { width: 160px; aspect-ratio: 16 / 9; border-radius: 8px; }
  .row-info { display: flex; flex-direction: column; gap: 6px; }
  .line {
    height: 12px;
    border-radius: 4px;
  }
  .line.wide { width: 90%; }
  .line.mid { width: 60%; }
  .line.narrow { width: 40%; }
  .shimmer {
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--button) 40%, transparent) 0%,
      color-mix(in oklab, var(--button) 80%, transparent) 50%,
      color-mix(in oklab, var(--button) 40%, transparent) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.6s linear infinite;
  }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .shimmer { animation: none; background: color-mix(in oklab, var(--button) 60%, transparent); }
  }
</style>
