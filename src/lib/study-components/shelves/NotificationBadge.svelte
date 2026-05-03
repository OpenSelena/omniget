<script lang="ts">
  let { count = 0 }: { count?: number } = $props();
  let pulse = $state(false);
  let prevCount = $state(0);

  $effect(() => {
    const current = count;
    if (current > prevCount) {
      pulse = true;
      const timer = setTimeout(() => (pulse = false), 600);
      prevCount = current;
      return () => clearTimeout(timer);
    }
    prevCount = current;
  });
</script>

{#if count > 0}
  <span class="badge" class:pulse aria-label="{count} notificações novas">
    {count > 99 ? "99+" : count}
  </span>
{/if}

<style>
  .badge {
    position: absolute;
    top: 8px;
    right: 8px;
    min-width: 18px;
    height: 18px;
    padding: 0 6px;
    background: var(--accent);
    color: var(--accent-contrast, white);
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    text-align: center;
    border-radius: 9px;
    box-shadow: 0 2px 6px color-mix(in oklab, var(--accent) 40%, transparent);
    z-index: 2;
  }

  .badge.pulse {
    animation: pulse-once 600ms ease-out;
  }

  @keyframes pulse-once {
    0% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.25);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .badge.pulse {
      animation: none;
    }
  }
</style>
