<script lang="ts">
  interface Props {
    open: boolean;
    title?: string;
    message?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    variant?: "default" | "danger";
    onConfirm?: () => void;
    onCancel?: () => void;
  }

  let {
    open = $bindable(false),
    title = "",
    message = "",
    confirmLabel = "Confirmar",
    cancelLabel = "Cancelar",
    variant = "default",
    onConfirm,
    onCancel,
  }: Props = $props();

  let cancelEl: HTMLButtonElement | null = $state(null);
  let confirmEl: HTMLButtonElement | null = $state(null);

  $effect(() => {
    if (open && cancelEl) {
      queueMicrotask(() => cancelEl?.focus());
    }
  });

  function close(reason: "confirm" | "cancel") {
    open = false;
    if (reason === "confirm" && onConfirm) onConfirm();
    if (reason === "cancel" && onCancel) onCancel();
  }

  function onOverlayKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close("cancel");
    } else if (e.key === "Tab") {
      const focusable = [cancelEl, confirmEl].filter(Boolean) as HTMLElement[];
      if (!focusable.length) return;
      const active = document.activeElement as HTMLElement | null;
      const idx = active ? focusable.indexOf(active) : -1;
      e.preventDefault();
      const next = e.shiftKey
        ? (idx - 1 + focusable.length) % focusable.length
        : (idx + 1) % focusable.length;
      focusable[next].focus();
    }
  }
</script>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) close("cancel");
    }}
    onkeydown={onOverlayKey}
  >
    <div
      class="sheet"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby={title ? "confirm-title" : undefined}
    >
      {#if title}<h3 id="confirm-title">{title}</h3>{/if}
      {#if message}<p class="msg">{message}</p>{/if}
      <div class="actions">
        <button
          type="button"
          class="btn outline"
          bind:this={cancelEl}
          onclick={() => close("cancel")}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          class="btn filled"
          class:danger={variant === "danger"}
          bind:this={confirmEl}
          onclick={() => close("confirm")}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.45));
    backdrop-filter: blur(8px) saturate(140%);
    -webkit-backdrop-filter: blur(8px) saturate(140%);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 1rem;
    animation: fadein 180ms ease-out;
  }
  .sheet {
    background: var(--popup-bg, var(--button-elevated));
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius, 11px);
    padding: calc(var(--padding, 10px) * 2);
    max-width: 420px;
    width: 100%;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
    animation: pop 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 16px;
    font-weight: 500;
    color: var(--secondary);
  }
  .msg {
    margin: 0 0 1.25rem;
    font-size: 14px;
    color: var(--secondary);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .btn {
    padding: 0.5rem 1rem;
    border-radius: var(--border-radius, 11px);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition:
      background 150ms ease,
      border-color 150ms ease;
  }
  .btn.outline {
    background: transparent;
    border: 1px solid var(--content-border);
    color: var(--secondary);
  }
  .btn.outline:hover {
    border-color: var(--accent);
  }
  .btn.filled {
    background: var(--accent);
    color: var(--on-accent);
    border: 1px solid var(--accent);
  }
  .btn.filled:hover {
    background: color-mix(in oklab, var(--accent) 85%, black);
    border-color: color-mix(in oklab, var(--accent) 85%, black);
  }
  .btn.filled.danger {
    background: var(--error);
    border-color: var(--error);
    color: var(--on-error, var(--on-accent));
  }
  .btn.filled.danger:hover {
    background: color-mix(in oklab, var(--error) 85%, black);
    border-color: color-mix(in oklab, var(--error) 85%, black);
  }
  .btn:focus-visible {
    outline: var(--focus-ring, 2px solid var(--accent));
    outline-offset: 2px;
  }
  @keyframes fadein {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .sheet {
      animation: none;
    }
  }
</style>
