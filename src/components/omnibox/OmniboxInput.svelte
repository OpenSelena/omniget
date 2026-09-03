<script lang="ts">
  import { t } from "$lib/i18n";
  import ContextHint from "$components/hints/ContextHint.svelte";

  let { url = $bindable(""), onInput, prominent = false }: { url?: string; onInput?: () => void; prominent?: boolean } = $props();

  let dragOver = $state(false);

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;

    if (e.dataTransfer?.files?.length) {
      const file = e.dataTransfer.files[0];
      if (file.name.endsWith(".torrent")) {
        url = (file as any).path || file.name;
        onInput?.();
        return;
      }
    }

    const text = e.dataTransfer?.getData("text/plain");
    if (text) {
      url = text.trim();
      onInput?.();
    }
  }
</script>

<div
  class="omnibox-wrapper"
  class:drag-over={dragOver}
  class:prominent
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  role="group"
>
  <svg class="omnibox-glyph" viewBox="0 0 20 20" width="15" height="15" fill="currentColor" aria-hidden="true">
    <path d="M8.5 2.5a6 6 0 1 0 3.67 10.74l3.8 3.79a1 1 0 0 0 1.41-1.41l-3.79-3.8A6 6 0 0 0 8.5 2.5zM4.5 8.5a4 4 0 1 1 8 0 4 4 0 0 1-8 0z" />
  </svg>
  <input
    class="omnibox"
    type="text"
    placeholder={$t('omnibox.placeholder')}
    bind:value={url}
    oninput={onInput}
  />
  {#if url.length > 0}
    <button class="clear-btn" onclick={() => { url = ""; onInput?.(); }} aria-label={$t('common.clear')}>
      <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
        <path d="M8 1.4a6.6 6.6 0 1 0 0 13.2A6.6 6.6 0 0 0 8 1.4zm2.35 3.5a.75.75 0 0 1 1.06 1.06L9.06 8l2.35 2.35a.75.75 0 1 1-1.06 1.06L8 9.06 5.65 11.4a.75.75 0 1 1-1.06-1.06L6.94 8 4.6 5.65A.75.75 0 0 1 5.65 4.6L8 6.94l2.35-2.04z" />
      </svg>
    </button>
  {/if}
  <ContextHint text={$t('hints.omnibox')} dismissKey="omnibox" />
</div>

<style>
  .omnibox-wrapper {
    width: 100%;
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 8px 0 12px;
    background: var(--control-bg);
    border-radius: var(--radius-md);
  }

  .omnibox-wrapper.prominent {
    height: 44px;
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px -12px color-mix(in srgb, var(--accent) 42%, transparent);
  }

  .omnibox-wrapper.prominent .omnibox {
    font-size: var(--text-md);
  }

  .omnibox-wrapper.drag-over {
    background: color-mix(in srgb, var(--blue) 8%, var(--control-bg));
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .omnibox-wrapper.prominent:focus-within {
    box-shadow:
      0 16px 40px -12px color-mix(in srgb, var(--accent) 42%, transparent),
      0 0 0 4px var(--accent-soft);
  }

  .omnibox-glyph {
    flex-shrink: 0;
    color: var(--text-dim);
    pointer-events: none;
  }

  .omnibox {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    font-size: var(--text-base);
    background: transparent;
    color: var(--text);
    border: none;
  }

  .omnibox::placeholder {
    color: var(--text-dim);
  }

  .omnibox:focus {
    outline: none;
  }

  .omnibox-wrapper:focus-within {
    box-shadow: 0 0 0 4px var(--accent-soft);
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
    border-radius: var(--radius-full);
    color: var(--text-dim);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
  }

  .clear-btn :global(svg) {
    pointer-events: none;
  }

  @media (hover: hover) {
    .clear-btn:hover {
      color: var(--text-muted);
      background: var(--fill-2);
    }
  }

  .clear-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
