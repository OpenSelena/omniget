<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { downloadAttachment } from "$lib/stores/omnidisc-store.svelte";
  import { portal } from "$lib/omnidisc/popover";
  import type { OmnidiscAttachment, OmnidiscMessage } from "$lib/omnidisc/types";

  let { message }: { message: OmnidiscMessage } = $props();

  let lightbox = $state<OmnidiscAttachment | null>(null);
  let lightboxEl = $state<HTMLElement | null>(null);
  let lightboxTrigger: HTMLElement | null = null;
  let busyId = $state<string | null>(null);
  let savedPath = $state<string | null>(null);
  let error = $state<string | null>(null);

  let items = $derived(message.attachments ?? []);

  function sizeLabel(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB"];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
  }

  function kindOf(a: OmnidiscAttachment): "image" | "video" | "audio" | "file" {
    // An encrypted attachment is ciphertext at that URL: nothing can play it in
    // place, so it is always offered as a download.
    if (a.encrypted || !a.url) return "file";
    const mime = a.contentType ?? "";
    if (mime.startsWith("image/")) return "image";
    if (mime.startsWith("video/")) return "video";
    if (mime.startsWith("audio/")) return "audio";
    return "file";
  }

  async function save(attachment: OmnidiscAttachment) {
    busyId = attachment.id;
    error = null;
    savedPath = null;
    try {
      savedPath = await downloadAttachment(message.channelId, message, attachment);
    } catch (e) {
      const raw = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
      error = translateBackendError(raw, $t);
    } finally {
      busyId = null;
    }
  }

  function openLightbox(attachment: OmnidiscAttachment, trigger: HTMLElement) {
    lightboxTrigger = trigger;
    lightbox = attachment;
  }

  function closeLightbox() {
    lightbox = null;
    lightboxTrigger?.focus();
    lightboxTrigger = null;
  }

  $effect(() => {
    if (lightbox) lightboxEl?.focus();
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && lightbox) {
      e.preventDefault();
      closeLightbox();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if items.length > 0}
  <ul class="attachments">
    {#each items as attachment (attachment.id)}
      {@const kind = kindOf(attachment)}
      <li class="item" class:media={kind === "image"}>
        {#if kind === "image"}
          <button
            type="button"
            class="image-button"
            style:aspect-ratio={attachment.width && attachment.height
              ? `${attachment.width} / ${attachment.height}`
              : undefined}
            onclick={(e) => openLightbox(attachment, e.currentTarget)}
            aria-label={$t("omnidisc.attachments.open_image", { name: attachment.filename })}
          >
            <img
              src={attachment.thumbnailUrl ?? attachment.url}
              alt={attachment.filename}
              loading="lazy"
              width={attachment.width}
              height={attachment.height}
            />
          </button>
        {:else if kind === "video"}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video class="player" src={attachment.url} controls preload="metadata"></video>
        {:else if kind === "audio"}
          <audio class="player" src={attachment.url} controls preload="metadata"></audio>
        {:else}
          <div class="file">
            <span class="glyph" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3v5h5" /><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /></svg>
            </span>
            <span class="detail">
              <span class="name" title={attachment.filename}>{attachment.filename}</span>
              <span class="meta">
                {sizeLabel(attachment.size)}{#if attachment.encrypted} · {$t("omnidisc.attachments.encrypted_note")}{/if}
              </span>
            </span>
            <button
              type="button"
              class="save"
              disabled={busyId === attachment.id}
              onclick={() => void save(attachment)}
            >
              {busyId === attachment.id
                ? $t("omnidisc.attachments.downloading")
                : $t("omnidisc.attachments.download")}
            </button>
          </div>
        {/if}
      </li>
    {/each}
  </ul>

  {#if savedPath}
    <p class="note" role="status">{$t("omnidisc.attachments.saved", { path: savedPath })}</p>
  {/if}
  {#if error}
    <p class="note danger" role="alert">{error}</p>
  {/if}
{/if}

{#if lightbox}
  <div
    class="lightbox"
    role="dialog"
    aria-modal="true"
    aria-label={lightbox.filename}
    tabindex="-1"
    use:portal
    bind:this={lightboxEl}
  >
    <img src={lightbox.url} alt={lightbox.filename} />
    <button
      type="button"
      class="close"
      onclick={closeLightbox}
      aria-label={$t("omnidisc.attachments.close_image")}
    >×</button>
  </div>
{/if}

<style>
  .attachments {
    list-style: none;
    margin: var(--space-1) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .item {
    max-width: 420px;
  }

  .image-button {
    display: block;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--fill-1);
    overflow: hidden;
    cursor: zoom-in;
    max-width: 100%;
  }

  .image-button img {
    display: block;
    max-width: 100%;
    max-height: 320px;
    width: auto;
    height: auto;
  }

  .image-button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .player {
    display: block;
    max-width: 100%;
    max-height: 320px;
    border-radius: var(--radius-md);
  }

  audio.player {
    width: 320px;
    max-height: none;
  }

  .file {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .glyph {
    display: grid;
    place-items: center;
    color: var(--text-muted);
  }

  .detail {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .save {
    flex: 0 0 auto;
    padding: 4px var(--space-2);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .save:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .save:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .save:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .note {
    margin: var(--space-1) 0 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .note.danger {
    color: var(--danger);
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: var(--space-5);
    background: var(--scrim, rgba(0, 0, 0, 0.72));
  }

  .lightbox img {
    max-width: 100%;
    max-height: 100%;
    border-radius: var(--radius-md);
  }

  .lightbox .close {
    position: absolute;
    top: var(--space-4);
    right: var(--space-4);
    width: 36px;
    height: 36px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-lg);
    line-height: 1;
    cursor: pointer;
  }

  .lightbox .close:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
