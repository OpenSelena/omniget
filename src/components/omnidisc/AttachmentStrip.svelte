<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { removePendingAttachment, retryAttachment } from "$lib/stores/omnidisc-store.svelte";
  import type { PendingAttachment } from "$lib/omnidisc/types";

  let { channelId, items }: { channelId: string; items: PendingAttachment[] } = $props();

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

  function percent(item: PendingAttachment): number {
    if (item.total <= 0) return 0;
    return Math.min(100, Math.round((item.sent / item.total) * 100));
  }

  function status(item: PendingAttachment): string {
    switch (item.state) {
      case "preparing":
        return item.encrypted ? $t("omnidisc.attachments.preparing") : sizeLabel(item.total);
      case "resuming":
        return $t("omnidisc.attachments.resuming");
      case "failed":
        return item.error
          ? translateBackendError(item.error, $t)
          : $t("omnidisc.attachments.failed");
      case "done":
        return `${sizeLabel(item.total)} · ${$t("omnidisc.attachments.ready")}`;
      default:
        return $t("omnidisc.attachments.uploading", { percent: percent(item) });
    }
  }
</script>

{#if items.length > 0}
  <ul class="strip" aria-label={$t("omnidisc.attachments.title")}>
    {#each items as item (item.id)}
      <li class="chip" class:failed={item.state === "failed"} class:ready={item.state === "done"}>
        {#if item.previewUrl}
          <img class="thumb" src={item.previewUrl} alt="" />
        {:else}
          <span class="thumb glyph" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3v5h5" /><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /></svg>
          </span>
        {/if}
        <span class="detail">
          <span class="name" title={item.name}>{item.name}</span>
          <span class="meta">{status(item)}</span>
          {#if item.state !== "done" && item.state !== "failed"}
            <span
              class="bar"
              role="progressbar"
              aria-valuenow={percent(item)}
              aria-valuemin="0"
              aria-valuemax="100"
            >
              <span class="fill" style:width={`${percent(item)}%`}></span>
            </span>
          {/if}
        </span>
        {#if item.state === "failed"}
          <button type="button" class="mini" onclick={() => void retryAttachment(channelId, item.id)}>
            {$t("omnidisc.attachments.retry")}
          </button>
        {/if}
        <button
          type="button"
          class="close"
          onclick={() => removePendingAttachment(channelId, item.id)}
          aria-label={$t("omnidisc.attachments.remove", { name: item.name })}
          title={$t("omnidisc.attachments.remove", { name: item.name })}
        >×</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .strip {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chip {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    max-width: 260px;
    padding: 6px var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .chip.ready {
    border-color: var(--success);
  }

  .chip.failed {
    border-color: var(--danger);
  }

  .thumb {
    width: 32px;
    height: 32px;
    flex: 0 0 auto;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--fill-1);
  }

  .glyph {
    display: grid;
    place-items: center;
    color: var(--text-muted);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-size: var(--text-xs);
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

  .chip.failed .meta {
    color: var(--danger);
  }

  .bar {
    display: block;
    height: 3px;
    border-radius: var(--radius-full);
    background: var(--fill-2);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width var(--duration-fast) var(--ease-out);
  }

  .mini,
  .close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    line-height: 1;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
  }

  .mini {
    color: var(--accent);
    font-weight: 600;
  }

  .mini:hover,
  .close:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .mini:focus-visible,
  .close:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
