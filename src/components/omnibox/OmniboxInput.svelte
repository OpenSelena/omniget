<script lang="ts">
  import { t } from "$lib/i18n";
  import ContextHint from "$components/hints/ContextHint.svelte";

  let { url = $bindable(""), onInput } = $props();

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
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  role="group"
>
  <input
    class="omnibox"
    type="text"
    placeholder={$t('omnibox.placeholder')}
    bind:value={url}
    oninput={onInput}
  />
  {#if url.length > 0}
    <button class="clear-btn" onclick={() => { url = ""; onInput?.(); }} aria-label={$t('common.clear')}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 6L6 18M6 6l12 12" />
      </svg>
    </button>
  {/if}
  <ContextHint text={$t('hints.omnibox')} dismissKey="omnibox" />
</div>

<style>
  .omnibox-wrapper {
    width: 100%;
    position: relative;
    transition: border-color 0.15s;
  }

  .omnibox-wrapper.drag-over .omnibox {
    border-color: var(--blue);
    background: color-mix(in srgb, var(--blue) 8%, var(--button));
  }

  .omnibox {
    width: 100%;
    padding: var(--padding) 38px var(--padding) calc(var(--padding) + 4px);
    font-size: 14.5px;
    background: var(--button);
    border-radius: var(--border-radius);
    color: var(--secondary);
    border: 1px solid var(--input-border);
  }

  .omnibox::placeholder {
    color: var(--gray);
  }

  .omnibox:focus-visible {
    border-color: var(--secondary);
    outline: none;
  }

  .clear-btn {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: calc(var(--border-radius) / 2);
    color: var(--gray);
    cursor: pointer;
    padding: 0;
  }

  .clear-btn :global(svg) {
    pointer-events: none;
  }

  @media (hover: hover) {
    .clear-btn:hover {
      color: var(--secondary);
      background: var(--button-stroke);
    }
  }
</style>
