<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    attachFile,
    getDraft,
    getPendingAttachments,
    hasUploadsInFlight,
    setDraft,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscMessage } from "$lib/omnidisc/types";
  import AttachmentStrip from "./AttachmentStrip.svelte";
  import EmojiPicker from "./EmojiPicker.svelte";

  const MAX_HEIGHT = 220;

  let {
    channelId,
    draftKey,
    placeholder,
    disabled = false,
    canAttach = true,
    typingNames = [],
    replyTo = null,
    onSend,
    onTyping,
    onCancelReply,
    onEditLast,
  }: {
    channelId: string;
    draftKey: string;
    placeholder: string;
    disabled?: boolean;
    canAttach?: boolean;
    typingNames?: string[];
    replyTo?: OmnidiscMessage | null;
    onSend: (text: string) => void;
    onTyping?: () => void;
    onCancelReply?: () => void;
    onEditLast?: () => void;
  } = $props();

  let textarea = $state<HTMLTextAreaElement | null>(null);
  let value = $state("");
  let emojiOpen = $state(false);
  let wrapper = $state<HTMLElement | null>(null);
  let dropActive = $state(false);
  let attachError = $state<string | null>(null);

  let attachments = $derived(getPendingAttachments(channelId));
  let uploading = $derived(hasUploadsInFlight(channelId));

  function fail(e: unknown) {
    const raw = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
    attachError = translateBackendError(raw, $t);
  }

  async function addPaths(paths: string[]) {
    attachError = null;
    for (const path of paths) {
      const name = path.split(/[\\/]/).pop() ?? "file";
      try {
        await attachFile(channelId, { path, name });
      } catch (e) {
        fail(e);
        return;
      }
    }
  }

  async function pickFiles() {
    if (!canAttach || disabled) return;
    try {
      const picked = await openFileDialog({ multiple: true });
      if (!picked) return;
      await addPaths(Array.isArray(picked) ? picked : [picked]);
    } catch (e) {
      fail(e);
    }
  }

  async function addBlobs(files: File[]) {
    attachError = null;
    for (const file of files) {
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        const name = file.name && file.name.length > 0 ? file.name : "pasted";
        const path = await invoke<string>("omnidisc_stage_file", {
          name,
          bytes: Array.from(bytes),
        });
        const previewUrl = file.type.startsWith("image/") ? URL.createObjectURL(file) : undefined;
        await attachFile(channelId, { path, name, previewUrl });
      } catch (e) {
        fail(e);
        return;
      }
    }
  }

  function onPaste(e: ClipboardEvent) {
    if (!canAttach || disabled) return;
    const files = Array.from(e.clipboardData?.files ?? []);
    if (files.length === 0) return;
    e.preventDefault();
    void addBlobs(files);
  }

  function overChat(position: { x: number; y: number } | undefined): boolean {
    const area = wrapper?.closest(".chat-view") ?? wrapper;
    if (!area || !position) return false;
    const rect = area.getBoundingClientRect();
    const scale = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
    const x = position.x / scale;
    const y = position.y / scale;
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  }

  // Tauri delivers file drops on the webview, not on the element, so the drop
  // target is worked out from the pointer position instead of a DOM event.
  onMount(() => {
    let stop: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (!canAttach || disabled) return;
        if (payload.type === "over") {
          dropActive = overChat(payload.position);
          return;
        }
        if (payload.type === "drop") {
          const inside = overChat(payload.position);
          dropActive = false;
          if (inside && payload.paths.length > 0) void addPaths(payload.paths);
          return;
        }
        dropActive = false;
      })
      .then((off) => {
        stop = off;
      })
      .catch(() => {});
    return () => stop?.();
  });

  $effect(() => {
    const key = draftKey;
    value = untrack(() => getDraft(key));
    tick().then(autoGrow);
  });

  function autoGrow() {
    const el = textarea;
    if (!el) return;
    el.style.height = "auto";
    const next = Math.min(el.scrollHeight, MAX_HEIGHT);
    el.style.height = `${next}px`;
    el.style.overflowY = el.scrollHeight > MAX_HEIGHT ? "auto" : "hidden";
  }

  function onInput() {
    setDraft(draftKey, value);
    autoGrow();
    if (value.trim().length > 0) onTyping?.();
  }

  function send() {
    const text = value.trim();
    if (disabled || uploading) return;
    if (!text && attachments.length === 0) return;
    onSend(text);
    value = "";
    setDraft(draftKey, "");
    tick().then(autoGrow);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
      return;
    }
    if (e.key === "Escape" && replyTo) {
      e.preventDefault();
      onCancelReply?.();
      return;
    }
    if (e.key === "ArrowUp" && value.length === 0 && onEditLast) {
      e.preventDefault();
      onEditLast();
    }
  }

  function insertEmoji(char: string) {
    const el = textarea;
    emojiOpen = false;
    if (!el) {
      value = `${value}${char}`;
      setDraft(draftKey, value);
      return;
    }
    const start = el.selectionStart ?? value.length;
    const end = el.selectionEnd ?? value.length;
    value = `${value.slice(0, start)}${char}${value.slice(end)}`;
    setDraft(draftKey, value);
    tick().then(() => {
      el.focus();
      el.setSelectionRange(start + char.length, start + char.length);
      autoGrow();
    });
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (!emojiOpen) return;
    if (wrapper && e.target instanceof Node && wrapper.contains(e.target)) return;
    emojiOpen = false;
  }

  let canSend = $derived(
    !disabled && !uploading && (value.trim().length > 0 || attachments.length > 0),
  );

  let typingLabel = $derived.by(() => {
    if (typingNames.length === 0) return "";
    if (typingNames.length === 1) return $t("omnidisc.messages.typing_one", { name: typingNames[0] });
    if (typingNames.length === 2) return $t("omnidisc.messages.typing_two", { a: typingNames[0], b: typingNames[1] });
    return $t("omnidisc.messages.typing_many");
  });
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<form class="composer" class:drop-active={dropActive} bind:this={wrapper} onsubmit={(e) => { e.preventDefault(); send(); }}>
  {#if dropActive}
    <p class="drop-hint" aria-live="polite">{$t("omnidisc.composer.drop_here")}</p>
  {/if}
  {#if attachError}
    <p class="attach-error" role="alert">{attachError}</p>
  {/if}
  <AttachmentStrip {channelId} items={attachments} />
  {#if replyTo}
    <div class="reply-bar">
      <span class="reply-label">{$t("omnidisc.composer.replying_to", { name: replyTo.authorName })}</span>
      <span class="reply-preview">{replyTo.content.slice(0, 80)}</span>
      <button type="button" class="reply-cancel" onclick={() => onCancelReply?.()} aria-label={$t("omnidisc.composer.cancel_reply")}>×</button>
    </div>
  {/if}
  {#if emojiOpen}
    <div class="emoji-pop">
      <EmojiPicker onPick={insertEmoji} onClose={() => (emojiOpen = false)} />
    </div>
  {/if}
  <div class="row">
  <textarea
    bind:this={textarea}
    bind:value
    class="input"
    rows="1"
    {placeholder}
    {disabled}
    aria-label={placeholder}
    oninput={onInput}
    onkeydown={onKeydown}
    onpaste={onPaste}
  ></textarea>
  {#if canAttach}
    <button
      type="button"
      class="emoji-btn attach-btn"
      onclick={() => void pickFiles()}
      disabled={disabled}
      aria-label={$t("omnidisc.composer.attach")}
      title={$t("omnidisc.composer.attach")}
    >
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21.4 11.05 12.2 20.2a5 5 0 0 1-7.1-7.1l9.2-9.15a3.3 3.3 0 1 1 4.7 4.7l-9.2 9.15a1.7 1.7 0 0 1-2.4-2.4l8.5-8.45" /></svg>
    </button>
  {/if}
  <button
    type="button"
    class="emoji-btn"
    onclick={() => (emojiOpen = !emojiOpen)}
    aria-expanded={emojiOpen}
    disabled={disabled}
    aria-label={$t("omnidisc.emoji.title")}
    title={$t("omnidisc.emoji.title")}
  >
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M9 10h.01M15 10h.01M8.5 14.5a4.5 4.5 0 0 0 7 0" /></svg>
  </button>
  <button type="submit" class="send" disabled={!canSend} aria-label={$t("omnidisc.composer.send")}>
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M22 2L11 13" />
      <path d="M22 2l-7 20-4-9-9-4z" />
    </svg>
  </button>
  </div>
  <p class="hint" aria-live="polite">
    {#if uploading}
      <span class="typing">{$t("omnidisc.composer.uploading_wait")}</span>
    {:else if typingLabel}
      <span class="typing">{typingLabel}</span>
    {:else}
      {$t("omnidisc.composer.hint")}
    {/if}
  </p>
</form>

<style>
  .composer {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: end;
    gap: var(--space-2);
  }

  .composer.drop-active {
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
  }

  .drop-hint {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--accent);
  }

  .attach-error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  .input {
    grid-column: 1;
    grid-row: 1;
    width: 100%;
    box-sizing: border-box;
    resize: none;
    min-height: 42px;
    max-height: 220px;
    padding: 10px var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-base);
    line-height: 1.45;
    overflow-y: hidden;
  }

  .input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .input:disabled {
    opacity: 0.6;
  }

  .reply-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
  }

  .reply-label {
    font-weight: 600;
    color: var(--text);
    flex: 0 0 auto;
  }

  .reply-preview {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .reply-cancel {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-base);
    line-height: 1;
    cursor: pointer;
  }

  .reply-cancel:hover {
    color: var(--text);
  }

  .emoji-pop {
    position: absolute;
    right: var(--space-4);
    bottom: 100%;
    z-index: 40;
  }

  .attach-btn {
    grid-column: 2;
  }

  .emoji-btn {
    grid-column: 3;
    grid-row: 1;
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border: 1px solid var(--input-border);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .emoji-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--fill-1);
  }

  .emoji-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .emoji-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .send {
    grid-column: 4;
    grid-row: 1;
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    cursor: pointer;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .send:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .send:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .hint {
    margin: 0;
    min-height: 1.2em;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .typing {
    color: var(--text);
  }

  @media (prefers-reduced-motion: reduce) {
    .send {
      transition: none;
    }
  }
</style>
