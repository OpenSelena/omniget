<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { t } from "$lib/i18n";
  import ContextHint from "$components/hints/ContextHint.svelte";

  let { url = $bindable(""), onInput } = $props();

  let dragOver = $state(false);

  async function openTorrentFile() {
    const selected = await open({
      title: "Select .torrent file",
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
      multiple: false,
    });
    if (selected && typeof selected === "string") {
      url = selected;
      onInput?.();
    }
  }

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
  <button
    class="torrent-btn"
    onclick={openTorrentFile}
    title="Open .torrent file"
    aria-label="Open .torrent file"
  >
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="3" />
      <circle cx="5" cy="6" r="2" />
      <circle cx="19" cy="6" r="2" />
      <circle cx="5" cy="18" r="2" />
      <circle cx="19" cy="18" r="2" />
      <path d="M9.5 10.5L6.5 7.5" />
      <path d="M14.5 10.5l3-3" />
      <path d="M9.5 13.5l-3 3" />
      <path d="M14.5 13.5l3 3" />
    </svg>
  </button>
  <ContextHint text={$t('hints.omnibox')} dismissKey="omnibox" />
</div>

<style>
  .omnibox-wrapper {
    width: 100%;
    position: relative;
    display: flex;
    align-items: center;
    gap: 0;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--button);
    transition: border-color 0.15s;
  }

  .omnibox-wrapper:focus-within {
    border-color: var(--secondary);
  }

  .omnibox-wrapper.drag-over {
    border-color: var(--blue);
    background: color-mix(in srgb, var(--blue) 8%, var(--button));
  }

  .omnibox {
    flex: 1;
    padding: var(--padding) calc(var(--padding) + 4px);
    font-size: 14.5px;
    background: transparent;
    color: var(--secondary);
    border: none;
    border-radius: var(--border-radius) 0 0 var(--border-radius);
    outline: none;
  }

  .omnibox::placeholder {
    color: var(--gray);
  }

  .torrent-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    flex-shrink: 0;
    background: none;
    border: none;
    border-left: 1px solid var(--input-border);
    border-radius: 0 var(--border-radius) var(--border-radius) 0;
    color: var(--gray);
    cursor: pointer;
    padding: 0;
  }

  @media (hover: hover) {
    .torrent-btn:hover {
      color: var(--secondary);
      background: var(--sidebar-highlight);
    }
  }

  .torrent-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
