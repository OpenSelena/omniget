<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { t } from "$lib/i18n";

  let {
    open,
    title,
    body,
    submitLabel,
    busy = false,
    error = null,
    canSubmit = true,
    onSubmit,
    onClose,
    children,
  }: {
    open: boolean;
    title: string;
    body?: string;
    submitLabel: string;
    busy?: boolean;
    error?: string | null;
    canSubmit?: boolean;
    onSubmit: () => void;
    onClose: () => void;
    children: Snippet;
  } = $props();

  let panel = $state<HTMLElement | null>(null);

  $effect(() => {
    if (!open) return;
    tick().then(() => {
      const first = panel?.querySelector<HTMLElement>("input, select, textarea, button");
      first?.focus();
    });
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !busy) {
      e.stopPropagation();
      onClose();
    }
  }

  function submit(e: Event) {
    e.preventDefault();
    if (busy || !canSubmit) return;
    onSubmit();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onkeydown={onKeydown} onclick={(e) => { if (e.target === e.currentTarget && !busy) onClose(); }}>
    <div class="panel" bind:this={panel} role="dialog" aria-modal="true" aria-labelledby="od-prompt-title" aria-busy={busy}>
    <form class="panel-form" onsubmit={submit}>
      <h2 id="od-prompt-title" class="title">{title}</h2>
      {#if body}
        <p class="body">{body}</p>
      {/if}
      <div class="fields">
        {@render children()}
      </div>
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
      <div class="actions">
        <button type="button" class="ghost" onclick={onClose} disabled={busy}>{$t("common.cancel")}</button>
        <button type="submit" class="primary" disabled={busy || !canSubmit}>
          {busy ? $t("omnidisc.guild.working") : submitLabel}
        </button>
      </div>
    </form>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: var(--space-5);
    background: color-mix(in srgb, var(--bg) 70%, transparent);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: 100%;
    max-width: 400px;
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .panel-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--text);
  }

  .body {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .fields :global(.field) {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .fields :global(.label) {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--text);
  }

  .fields :global(.input) {
    box-sizing: border-box;
    width: 100%;
    padding: 10px var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-base);
  }

  .fields :global(.input:focus-visible) {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .fields :global(.hint) {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
    color: var(--danger);
    font-size: var(--text-sm);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .primary {
    padding: 10px var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }

  .primary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .ghost {
    padding: 10px var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .ghost:hover {
    color: var(--text);
  }

  .primary:focus-visible,
  .ghost:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
