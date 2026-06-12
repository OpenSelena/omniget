<script lang="ts">
  let { label }: { label: string } = $props();
</script>

{#if label}
  {#key label}
    <div class="zoom-indicator mono" aria-live="polite">{label}</div>
  {/key}
{/if}

<style>
  .zoom-indicator {
    position: fixed;
    top: 64px;
    left: 50%;
    transform: translateX(-50%);
    padding: 6px 14px;
    background: var(--popup-bg, var(--bg-elevated, var(--bg)));
    color: var(--text, var(--secondary));
    border: 1px solid var(--border, var(--content-border));
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
    z-index: 60;
    pointer-events: none;
    animation: zoom-fade 1.2s ease-out forwards;
  }
  .mono {
    font-family: var(--font-mono, "IBM Plex Mono", ui-monospace, monospace);
  }
  @keyframes zoom-fade {
    0% {
      opacity: 1;
    }
    70% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .zoom-indicator {
      animation: none;
    }
  }
</style>
