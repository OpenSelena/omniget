<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { telegramExpandAlbum, type TelegramMediaItem, type TelegramChatType } from "$lib/study-telegram-bridge";

  let { chatId, chatType, messageId, onClose }: {
    chatId: number;
    chatType: TelegramChatType;
    messageId: number;
    onClose: () => void;
  } = $props();

  let items = $state<TelegramMediaItem[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let index = $state(0);

  const currentItem = $derived(items[index] ?? null);

  async function load() {
    loading = true;
    error = null;
    try {
      items = await telegramExpandAlbum({ chatId, chatType, messageId });
      const targetIdx = items.findIndex((it) => it.message_id === messageId);
      index = targetIdx >= 0 ? targetIdx : 0;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function next() {
    if (items.length === 0) return;
    index = (index + 1) % items.length;
  }

  function prev() {
    if (items.length === 0) return;
    index = (index - 1 + items.length) % items.length;
  }

  function onKeydown(ev: KeyboardEvent) {
    if (ev.key === "Escape") {
      onClose();
    } else if (ev.key === "ArrowRight") {
      next();
    } else if (ev.key === "ArrowLeft") {
      prev();
    }
  }

  onMount(() => {
    void load();
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="album-overlay" onclick={onClose} role="presentation"></div>
<div class="album-modal" role="dialog" aria-modal="true" aria-label={$t("study.library.telegram.album_title")}>
  <header class="album-head">
    <span class="album-counter">
      {#if items.length > 0}
        {index + 1} / {items.length}
      {:else if loading}
        {$t("study.common.loading")}
      {/if}
    </span>
    <button type="button" class="album-close" onclick={onClose} aria-label={$t("study.common.close")}>
      ✕
    </button>
  </header>
  <div class="album-body">
    {#if loading}
      <p class="muted">{$t("study.common.loading")}</p>
    {:else if error}
      <p class="error small">{error}</p>
    {:else if currentItem}
      <div class="album-stage">
        <button type="button" class="nav-btn left" onclick={prev} disabled={items.length < 2} aria-label={$t("study.library.telegram.album_prev")}>
          ‹
        </button>
        <div class="album-content">
          <span class="album-name">{currentItem.file_name}</span>
          <span class="album-meta muted small">{currentItem.media_type} · {currentItem.message_id}</span>
          {#if currentItem.caption}
            <p class="album-caption">{currentItem.caption}</p>
          {/if}
        </div>
        <button type="button" class="nav-btn right" onclick={next} disabled={items.length < 2} aria-label={$t("study.library.telegram.album_next")}>
          ›
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .album-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 1000;
    cursor: pointer;
    animation: fade-in var(--tg-duration-normal, 200ms) var(--tg-easing-default, ease);
  }
  .album-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(90vw, 720px);
    max-height: 80vh;
    background: var(--surface);
    border: 1px solid var(--content-border);
    border-radius: 12px;
    z-index: 1001;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .album-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--content-border);
  }
  .album-counter {
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
  }
  .album-close {
    background: transparent;
    border: 0;
    font-size: 16px;
    color: var(--tertiary);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .album-close:hover {
    background: color-mix(in oklab, var(--secondary) 8%, transparent);
  }
  .album-body {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
  }
  .album-stage {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 16px;
  }
  .nav-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: 1px solid var(--content-border);
    background: var(--surface);
    color: var(--secondary);
    cursor: pointer;
    font-size: 22px;
    line-height: 1;
  }
  .nav-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .nav-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .album-content {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    text-align: center;
  }
  .album-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
    word-break: break-word;
  }
  .album-caption {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--tertiary);
    max-height: 200px;
    overflow-y: auto;
  }
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
