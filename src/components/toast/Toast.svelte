<script lang="ts">
  import { getToasts, dismissToast, type ToastType } from "$lib/stores/toast-store.svelte";

  let toasts = $derived(getToasts());

  function iconPath(type: ToastType): string {
    switch (type) {
      case "success":
        return "M5 12l5 5L20 7";
      case "error":
        return "M18 6L6 18M6 6l12 12";
      case "info":
        return "M12 8v4m0 4h.01M12 2a10 10 0 100 20 10 10 0 000-20z";
    }
  }
</script>

{#if toasts.length > 0}
  <div class="toast-container" aria-live="polite" aria-atomic="false">
    {#each toasts as toast (toast.id)}
      <div
        class="toast"
        data-type={toast.type}
        class:closing={toast.closing}
        role={toast.type === "error" ? "alert" : "status"}
        aria-live={toast.type === "error" ? "assertive" : "polite"}
      >
        <svg class="toast-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d={iconPath(toast.type)} />
        </svg>
        <span class="toast-message">{toast.message}</span>
        <button class="toast-close" onclick={() => dismissToast(toast.id)} aria-label="Close">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: var(--space-3);
    right: var(--space-3);
    z-index: 9999;
    display: flex;
    flex-direction: column-reverse;
    gap: var(--space-2);
    pointer-events: none;
    max-width: 400px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--surface-hi);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-3);
    border-left: 3px solid var(--info);
    pointer-events: auto;
    animation: toast-in var(--duration-bounce) var(--ease-spring);
  }

  .toast[data-type="success"] { border-left-color: var(--success); }
  .toast[data-type="error"]   { border-left-color: var(--danger); }
  .toast[data-type="info"]    { border-left-color: var(--info); }

  .toast.closing {
    animation: toast-out var(--duration-base) var(--ease-out) forwards;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes toast-out {
    from { opacity: 1; transform: translateY(0); }
    to   { opacity: 0; transform: translateY(-4px); }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: toast-fade-in var(--duration-base) ease-out;
    }
    .toast.closing {
      animation: toast-fade-out var(--duration-base) ease-out forwards;
    }
    @keyframes toast-fade-in {
      from { opacity: 0; }
      to   { opacity: 1; }
    }
    @keyframes toast-fade-out {
      from { opacity: 1; }
      to   { opacity: 0; }
    }
  }

  .toast-icon {
    flex-shrink: 0;
    pointer-events: none;
  }

  .toast[data-type="success"] .toast-icon { color: var(--success); }
  .toast[data-type="error"] .toast-icon   { color: var(--danger); }
  .toast[data-type="info"] .toast-icon    { color: var(--info); }

  .toast-message {
    flex: 1;
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--text);
    min-width: 0;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .toast-close {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-xs);
    color: var(--text-dim);
    cursor: pointer;
    transition: color var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .toast-close :global(svg) {
    pointer-events: none;
  }

  @media (hover: hover) {
    .toast-close:hover {
      color: var(--secondary);
      background: var(--button-stroke);
    }
  }

  .toast-close:active {
    background: var(--content-border);
  }
</style>
