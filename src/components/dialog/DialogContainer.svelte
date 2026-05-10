<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    isOpen = $bindable(false),
    titleId = "dialog-title",
    onClose,
    children,
  }: {
    isOpen?: boolean;
    titleId?: string;
    onClose?: () => void;
    children?: Snippet;
  } = $props();

  let dialogEl = $state<HTMLDialogElement | null>(null);
  let previousFocusEl = $state<HTMLElement | null>(null);
  let closing = $state(false);

  $effect(() => {
    if (!dialogEl) return;
    if (isOpen && !dialogEl.open) {
      previousFocusEl = document.activeElement as HTMLElement;
      dialogEl.showModal();
    } else if (!isOpen && dialogEl.open && !closing) {
      closing = true;
      setTimeout(() => {
        closing = false;
        dialogEl?.close();
        previousFocusEl?.focus();
        previousFocusEl = null;
      }, 150);
    }
  });

  function handleClose() {
    closing = true;
    setTimeout(() => {
      closing = false;
      dialogEl?.close();
      isOpen = false;
      onClose?.();
      previousFocusEl?.focus();
      previousFocusEl = null;
    }, 150);
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) {
      handleClose();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="dialog-container"
  class:closing
  aria-labelledby={titleId}
  aria-modal="true"
  onclick={handleBackdropClick}
  oncancel={(e) => {
    e.preventDefault();
    handleClose();
  }}
>
  <div class="dialog-content">
    {@render children?.()}
  </div>
</dialog>

<style>
  .dialog-container {
    border: none;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    padding: 0;
    width: 92%;
    max-width: 540px;
    max-height: 80vh;
    animation: dialog-in var(--duration-base) var(--ease-out);
    box-shadow: var(--elev-3);
  }

  .dialog-container::backdrop {
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    animation: backdrop-in var(--duration-base) var(--ease-out);
  }

  .dialog-container.closing {
    animation: dialog-out var(--duration-base) var(--ease-out) forwards;
  }

  .dialog-container.closing::backdrop {
    animation: backdrop-out var(--duration-base) var(--ease-out) forwards;
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes dialog-out {
    from {
      opacity: 1;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(0.96);
    }
  }

  @keyframes backdrop-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes backdrop-out {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  .dialog-content {
    display: flex;
    flex-direction: column;
    max-height: 80vh;
  }

  @media (prefers-reduced-motion: reduce) {
    .dialog-container {
      animation: none;
    }

    .dialog-container::backdrop {
      animation: none;
    }

    .dialog-container.closing {
      animation: none;
    }

    .dialog-container.closing::backdrop {
      animation: none;
    }
  }
</style>
