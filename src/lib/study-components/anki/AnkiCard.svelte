<script lang="ts">
  type Variant = "default" | "elevated" | "ghost" | "highlight";

  let {
    variant = "default" as Variant,
    padding = "m" as "s" | "m" | "l",
    interactive = false,
    children,
    onclick,
  } = $props<{
    variant?: Variant;
    padding?: "s" | "m" | "l";
    interactive?: boolean;
    children?: any;
    onclick?: (e: MouseEvent) => void;
  }>();
</script>

{#if onclick}
  <button
    type="button"
    class="ank-card ank-card--{variant} ank-pad-{padding}"
    class:ank-card--interactive={interactive}
    onclick={onclick}
  >
    {@render children?.()}
  </button>
{:else}
  <div
    class="ank-card ank-card--{variant} ank-pad-{padding}"
    class:ank-card--interactive={interactive}
  >
    {@render children?.()}
  </div>
{/if}

<style>
  .ank-card {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: inherit;
    text-align: left;
    font: inherit;
    transition: border-color var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out);
  }

  .ank-card.ank-pad-s {
    padding: var(--space-3);
  }
  .ank-card.ank-pad-m {
    padding: var(--space-5);
  }
  .ank-card.ank-pad-l {
    padding: var(--space-6);
  }

  .ank-card--default {
    background: var(--surface);
  }

  .ank-card--elevated {
    background: var(--surface);
    box-shadow: var(--elev-2);
    border-color: transparent;
  }

  .ank-card--ghost {
    background: transparent;
    border-color: var(--border);
  }

  .ank-card--highlight {
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--accent) 14%, transparent),
      transparent 60%
    );
    border-color: var(--accent);
  }

  .ank-card--interactive {
    cursor: pointer;
  }
  button.ank-card {
    cursor: pointer;
  }

  .ank-card--interactive:hover,
  button.ank-card:hover {
    border-color: var(--accent);
    box-shadow: var(--elev-2);
    transform: translateY(-1px);
  }
</style>
