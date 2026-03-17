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
    padding: var(--padding) calc(var(--padding) + 4px);
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
</style>
