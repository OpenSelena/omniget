<script lang="ts">
  type Variant = "primary" | "ghost" | "outline" | "success" | "warning" | "error";
  type Size = "s" | "m" | "l";

  let {
    variant = "ghost" as Variant,
    size = "m" as Size,
    icon = "" as string,
    block = false,
    disabled = false,
    type = "button" as "button" | "submit",
    children,
    onclick,
  } = $props<{
    variant?: Variant;
    size?: Size;
    icon?: string;
    block?: boolean;
    disabled?: boolean;
    type?: "button" | "submit";
    children?: any;
    onclick?: (e: MouseEvent) => void;
  }>();
</script>

<button
  {type}
  class="ank-btn ank-btn--{variant} ank-size-{size}"
  class:ank-block={block}
  {disabled}
  onclick={onclick}
>
  {#if icon}<span class="ank-icon" aria-hidden="true">{icon}</span>{/if}
  <span class="ank-label">{@render children?.()}</span>
</button>

<style>
  .ank-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--surface-hi);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out);
  }

  .ank-block {
    width: 100%;
  }

  .ank-size-s {
    padding: 5px 10px;
    font-size: var(--text-xs);
  }

  .ank-size-l {
    padding: var(--space-3) var(--space-5);
    font-size: var(--text-md);
  }

  .ank-btn:hover:not(:disabled) {
    background: var(--surface-mut);
  }

  .ank-btn:active:not(:disabled) {
    transform: translateY(1px);
  }

  .ank-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .ank-btn--primary {
    background: var(--accent);
    color: var(--on-accent);
  }

  .ank-btn--primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 88%, black);
  }

  .ank-btn--outline {
    background: transparent;
    border-color: var(--border-hi);
  }

  .ank-btn--success {
    background: var(--success);
    color: white;
  }
  .ank-btn--success:hover:not(:disabled) {
    background: color-mix(in oklab, var(--success) 88%, black);
  }

  .ank-btn--warning {
    background: var(--warning);
    color: white;
  }
  .ank-btn--warning:hover:not(:disabled) {
    background: color-mix(in oklab, var(--warning) 88%, black);
  }

  .ank-btn--error {
    background: var(--error);
    color: white;
  }
  .ank-btn--error:hover:not(:disabled) {
    background: color-mix(in oklab, var(--error) 88%, black);
  }

  .ank-icon {
    font-size: 1.05em;
    line-height: 1;
  }
</style>
