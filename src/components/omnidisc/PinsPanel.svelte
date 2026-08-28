<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { getPins, loadPins, setPinned, userName } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscMessage } from "$lib/omnidisc/types";

  let {
    instanceId,
    channelId,
    canPin = false,
    onJump,
    onClose,
  }: {
    instanceId: string;
    channelId: string;
    canPin?: boolean;
    onJump: (messageId: string) => void;
    onClose: () => void;
  } = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let pins = $derived(getPins(channelId));

  $effect(() => {
    const id = channelId;
    loading = true;
    error = null;
    loadPins(id)
      .catch((e) => {
        error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
      })
      .finally(() => {
        loading = false;
      });
  });

  function timeLabel(ts: number): string {
    return new Date(ts).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  async function unpin(message: OmnidiscMessage) {
    error = null;
    try {
      await setPinned(channelId, message.id, false);
    } catch (e) {
      error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
    }
  }
</script>

<aside class="pins-panel" aria-label={$t("omnidisc.pins.title")}>
  <header class="head">
    <h3>{$t("omnidisc.pins.title")}</h3>
    <button type="button" class="close" onclick={onClose} aria-label={$t("common.close")}>×</button>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if loading}
    <div class="state" aria-busy="true">
      {#each Array(3) as _, i (i)}
        <span class="skeleton-line"></span>
      {/each}
    </div>
  {:else if pins.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.pins.empty_title")}</p>
      <p class="state-body">{$t("omnidisc.pins.empty_body")}</p>
    </div>
  {:else}
    <ul class="list">
      {#each pins as pin (pin.id)}
        <li class="item">
          <button type="button" class="jump" onclick={() => onJump(pin.id)}>
            <span class="item-head">
              <span class="item-author">{userName(instanceId, pin.authorId)}</span>
              <span class="item-time">{timeLabel(pin.createdAt)}</span>
            </span>
            <span class="item-text">{pin.content.slice(0, 220)}</span>
          </button>
          {#if canPin}
            <button type="button" class="unpin" onclick={() => void unpin(pin)}>{$t("omnidisc.pins.unpin")}</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .pins-panel {
    width: 300px;
    flex: 0 0 300px;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--surface-mut);
    border-left: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h3 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .close {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-base);
    line-height: 1;
    cursor: pointer;
  }

  .close:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) 0;
  }

  .state-title {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text);
  }

  .state-body {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  .skeleton-line {
    height: 44px;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
  }

  .jump {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .jump:hover {
    background: var(--fill-1);
  }

  .item-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .item-author {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .item-time {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .item-text {
    font-size: var(--text-sm);
    color: var(--text-secondary, var(--text));
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .unpin {
    align-self: flex-start;
    padding: 2px var(--space-2);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .unpin:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .jump:focus-visible,
  .unpin:focus-visible,
  .close:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-line {
      animation: none;
    }
  }
</style>
