<script lang="ts">
  import { tick } from "svelte";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import type { OmnidiscMessage } from "$lib/omnidisc/types";
  import { placeMenu, portal } from "$lib/omnidisc/popover";
  import EmojiPicker from "./EmojiPicker.svelte";
  import MessageAttachments from "./MessageAttachments.svelte";

  let {
    message,
    showHeader,
    daySeparator,
    unreadDivider = false,
    highlighted = false,
    replyPreview = null,
    canEdit = false,
    canDelete = false,
    canPin = false,
    canReact = false,
    editing = false,
    onRetry,
    onDiscard,
    onReact,
    onReply,
    onStartEdit,
    onSubmitEdit,
    onCancelEdit,
    onDelete,
    onTogglePin,
    onCopyLink,
    onCopyId,
    onMarkUnread,
    onJumpToReply,
    onOpenProfile,
  }: {
    message: OmnidiscMessage;
    showHeader: boolean;
    daySeparator: boolean;
    unreadDivider?: boolean;
    highlighted?: boolean;
    replyPreview?: OmnidiscMessage | null;
    canEdit?: boolean;
    canDelete?: boolean;
    canPin?: boolean;
    canReact?: boolean;
    editing?: boolean;
    onRetry?: (message: OmnidiscMessage) => void;
    onDiscard?: (message: OmnidiscMessage) => void;
    onReact?: (message: OmnidiscMessage, emoji: string) => void;
    onReply?: (message: OmnidiscMessage) => void;
    onStartEdit?: (message: OmnidiscMessage) => void;
    onSubmitEdit?: (message: OmnidiscMessage, content: string) => void;
    onCancelEdit?: () => void;
    onDelete?: (message: OmnidiscMessage) => void;
    onTogglePin?: (message: OmnidiscMessage) => void;
    onCopyLink?: (message: OmnidiscMessage) => void;
    onCopyId?: (message: OmnidiscMessage) => void;
    onMarkUnread?: (message: OmnidiscMessage) => void;
    onJumpToReply?: (messageId: string) => void;
    onOpenProfile?: (userId: string, anchor: HTMLElement) => void;
  } = $props();

  let menuOpen = $state(false);
  let pickerOpen = $state(false);
  let editValue = $state("");
  let editArea = $state<HTMLTextAreaElement | null>(null);
  let root = $state<HTMLElement | null>(null);
  let actionsEl = $state<HTMLElement | null>(null);
  let menuEl = $state<HTMLElement | null>(null);
  let pickerEl = $state<HTMLElement | null>(null);
  let menuBtn = $state<HTMLButtonElement | null>(null);
  let pickerBtn = $state<HTMLButtonElement | null>(null);
  let overlayPos = $state({ left: 0, top: 0 });
  let placed = $state(false);

  $effect(() => {
    if (!editing) return;
    editValue = message.content;
    tick().then(() => {
      editArea?.focus();
      editArea?.setSelectionRange(editValue.length, editValue.length);
    });
  });

  function dayLabel(ts: number): string {
    const date = new Date(ts);
    const today = new Date();
    const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
    const startOfDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const diffDays = Math.round((startOfToday - startOfDay) / 86_400_000);
    if (diffDays === 0) return $t("omnidisc.messages.today");
    if (diffDays === 1) return $t("omnidisc.messages.yesterday");
    return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }

  function timeLabel(ts: number): string {
    return new Date(ts).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  }

  function overlayNode(): HTMLElement | null {
    return menuEl ?? pickerEl;
  }

  function closeOverlays(restoreFocus = false) {
    const trigger = menuOpen ? menuBtn : pickerOpen ? pickerBtn : null;
    menuOpen = false;
    pickerOpen = false;
    placed = false;
    if (restoreFocus) trigger?.focus();
  }

  function place() {
    const node = overlayNode();
    const anchor = actionsEl?.getBoundingClientRect();
    if (!node || !anchor) return;
    const box = node.getBoundingClientRect();
    overlayPos = placeMenu(
      anchor,
      { width: box.width, height: box.height },
      { width: window.innerWidth, height: window.innerHeight },
    );
    placed = true;
  }

  async function toggleOverlay(which: "menu" | "picker") {
    const wasOpen = which === "menu" ? menuOpen : pickerOpen;
    closeOverlays();
    if (wasOpen) return;
    if (which === "menu") menuOpen = true;
    else pickerOpen = true;
    await tick();
    place();
    if (which !== "menu") return;
    await tick();
    menuEl?.querySelector<HTMLButtonElement>("[role='menuitem']")?.focus();
  }

  // The menu lives on the body, so nothing inside the message can be trusted to
  // contain the click any more, and a scroll would leave it hanging in place.
  $effect(() => {
    if (!menuOpen && !pickerOpen) return;
    const inside = (target: EventTarget | null): boolean =>
      target instanceof Node && (overlayNode()?.contains(target) ?? false);
    const dismiss = (e: PointerEvent) => {
      if (inside(e.target)) return;
      if (root && e.target instanceof Node && root.contains(e.target)) return;
      closeOverlays();
    };
    const scrolled = (e: Event) => {
      if (inside(e.target)) return;
      closeOverlays();
    };
    const resized = () => closeOverlays();
    window.addEventListener("pointerdown", dismiss, true);
    window.addEventListener("scroll", scrolled, true);
    window.addEventListener("resize", resized);
    return () => {
      window.removeEventListener("pointerdown", dismiss, true);
      window.removeEventListener("scroll", scrolled, true);
      window.removeEventListener("resize", resized);
    };
  });

  function react(emoji: string) {
    closeOverlays(true);
    onReact?.(message, emoji);
  }

  function editKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onCancelEdit?.();
      return;
    }
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submitEdit();
    }
  }

  function submitEdit() {
    const text = editValue.trim();
    if (!text || text === message.content) {
      onCancelEdit?.();
      return;
    }
    onSubmitEdit?.(message, text);
  }

  let initial = $derived(message.authorName.trim().charAt(0).toUpperCase() || "?");
  let pending = $derived(message.delivery === "pending");
  let failed = $derived(message.delivery === "failed");
  // MLS proves a group member sent this; it does not prove the identity string
  // in their credential. Anything the roster does not vouch for is called out.
  let unverified = $derived(message.senderVerified === false);
  let sent = $derived(message.delivery === "sent" || message.delivery === undefined);
  let showActions = $derived(sent && (canReact || canEdit || canDelete || canPin || !!onReply));
</script>

{#if daySeparator}
  <div class="day-separator" role="separator" aria-label={dayLabel(message.createdAt)}>
    <span>{dayLabel(message.createdAt)}</span>
  </div>
{/if}

{#if unreadDivider}
  <div class="unread-divider" role="separator" aria-label={$t("omnidisc.messages.new_messages")}>
    <span>{$t("omnidisc.messages.new_messages")}</span>
  </div>
{/if}

<article
  bind:this={root}
  class="message"
  class:grouped={!showHeader}
  class:pending
  class:failed
  class:highlighted
  class:editing
  aria-label={message.authorName}
  data-message-id={message.id}
>
  {#if replyPreview || message.replyToId}
    <button
      type="button"
      class="reply-quote"
      onclick={() => message.replyToId && onJumpToReply?.(message.replyToId)}
      disabled={!message.replyToId}
    >
      <span class="reply-mark" aria-hidden="true">↰</span>
      {#if replyPreview}
        <span class="reply-author">{replyPreview.authorName}</span>
        <span class="reply-text">{replyPreview.content.slice(0, 120)}</span>
      {:else}
        <span class="reply-text muted">{$t("omnidisc.messages.reply_unavailable")}</span>
      {/if}
    </button>
  {/if}

  {#if showHeader}
    <button
      type="button"
      class="avatar"
      onclick={(e) => onOpenProfile?.(message.authorId, e.currentTarget)}
      aria-label={$t("omnidisc.profile.open", { name: message.authorName })}
    >{initial}</button>
    <div class="body">
      <header class="meta">
        <button
          type="button"
          class="author"
          onclick={(e) => onOpenProfile?.(message.authorId, e.currentTarget)}
        >{message.authorName}</button>
        <time class="time" datetime={new Date(message.createdAt).toISOString()}>{timeLabel(message.createdAt)}</time>
        {#if message.pinned}
          <span class="tag">{$t("omnidisc.messages.pinned")}</span>
        {/if}
        {#if unverified}
          {@render unverifiedBadge()}
        {/if}
      </header>
      {#if editing}
        <div class="edit">
          <textarea
            bind:this={editArea}
            bind:value={editValue}
            class="edit-input"
            rows="2"
            aria-label={$t("omnidisc.messages.edit_label")}
            onkeydown={editKeydown}
          ></textarea>
          <div class="edit-actions">
            <button type="button" class="mini primary" onclick={submitEdit}>{$t("omnidisc.messages.edit_save")}</button>
            <button type="button" class="mini" onclick={() => onCancelEdit?.()}>{$t("common.cancel")}</button>
          </div>
        </div>
      {:else if message.awaitingDecryption}
        <p class="content locked">{$t("omnidisc.e2ee.unavailable")}</p>
      {:else}
        {#if message.content}
          <p class="content">{message.content}{#if message.editedAt}<span class="edited"> {$t("omnidisc.messages.edited")}</span>{/if}</p>
        {/if}
        <MessageAttachments {message} />
      {/if}
    </div>
  {:else}
    <time class="time gutter" datetime={new Date(message.createdAt).toISOString()}>{timeLabel(message.createdAt)}</time>
    <div class="body">
      {#if unverified}
        <div class="unverified-row">{@render unverifiedBadge()}</div>
      {/if}
      {#if editing}
        <div class="edit">
          <textarea
            bind:this={editArea}
            bind:value={editValue}
            class="edit-input"
            rows="2"
            aria-label={$t("omnidisc.messages.edit_label")}
            onkeydown={editKeydown}
          ></textarea>
          <div class="edit-actions">
            <button type="button" class="mini primary" onclick={submitEdit}>{$t("omnidisc.messages.edit_save")}</button>
            <button type="button" class="mini" onclick={() => onCancelEdit?.()}>{$t("common.cancel")}</button>
          </div>
        </div>
      {:else if message.awaitingDecryption}
        <p class="content locked">{$t("omnidisc.e2ee.unavailable")}</p>
      {:else}
        {#if message.content}
          <p class="content">{message.content}{#if message.editedAt}<span class="edited"> {$t("omnidisc.messages.edited")}</span>{/if}</p>
        {/if}
        <MessageAttachments {message} />
      {/if}
    </div>
  {/if}

  {#if pending}
    <span class="status" role="status">{$t("omnidisc.messages.pending")}</span>
  {:else if failed}
    <div class="status failed-row" role="alert">
      <span>{$t("omnidisc.messages.failed")} {message.error ? translateBackendError(message.error, $t) : ""}</span>
      <button type="button" class="mini" onclick={() => onRetry?.(message)}>{$t("omnidisc.messages.retry")}</button>
      <button type="button" class="mini ghost" onclick={() => onDiscard?.(message)}>{$t("omnidisc.messages.discard")}</button>
    </div>
  {/if}

  {#if message.reactions && message.reactions.length > 0}
    <ul class="reactions">
      {#each message.reactions as r (r.emoji)}
        <li>
          <button
            type="button"
            class="reaction"
            class:me={r.me}
            disabled={!canReact}
            onclick={() => onReact?.(message, r.emoji)}
            aria-pressed={r.me}
            title={r.me ? $t("omnidisc.messages.reaction_remove", { emoji: r.emoji.split(":")[0] }) : $t("omnidisc.messages.reaction_add", { emoji: r.emoji.split(":")[0] })}
          >
            <span>{r.emoji.split(":")[0]}</span>
            <span class="count">{r.count}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if showActions && !editing}
    <div class="actions" class:open={menuOpen || pickerOpen} bind:this={actionsEl}>
      {#if canReact}
        <button
          type="button"
          class="action"
          bind:this={pickerBtn}
          onclick={() => void toggleOverlay("picker")}
          aria-expanded={pickerOpen}
          title={$t("omnidisc.messages.react")}
          aria-label={$t("omnidisc.messages.react")}
        >
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M9 10h.01M15 10h.01M8.5 14.5a4.5 4.5 0 0 0 7 0" /></svg>
        </button>
      {/if}
      {#if onReply}
        <button type="button" class="action" onclick={() => onReply?.(message)} title={$t("omnidisc.messages.reply")} aria-label={$t("omnidisc.messages.reply")}>
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 14L4 9l5-5" /><path d="M4 9h9a7 7 0 0 1 7 7v4" /></svg>
        </button>
      {/if}
      {#if canEdit}
        <button type="button" class="action" onclick={() => onStartEdit?.(message)} title={$t("omnidisc.messages.edit")} aria-label={$t("omnidisc.messages.edit")}>
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
        </button>
      {/if}
      <button
        type="button"
        class="action"
        bind:this={menuBtn}
        onclick={() => void toggleOverlay("menu")}
        aria-expanded={menuOpen}
        title={$t("omnidisc.messages.more")}
        aria-label={$t("omnidisc.messages.more")}
      >
        <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="1.6" /><circle cx="12" cy="12" r="1.6" /><circle cx="19" cy="12" r="1.6" /></svg>
      </button>
    </div>

    {#if pickerOpen}
      <div
        class="overlay picker-anchor"
        class:placed
        use:portal
        bind:this={pickerEl}
        style="left: {overlayPos.left}px; top: {overlayPos.top}px;"
      >
        <EmojiPicker onPick={react} onClose={() => closeOverlays(true)} />
      </div>
    {/if}

    {#if menuOpen}
      <div
        class="overlay menu"
        class:placed
        role="menu"
        tabindex="-1"
        aria-label={$t("omnidisc.messages.more")}
        use:portal
        bind:this={menuEl}
        style="left: {overlayPos.left}px; top: {overlayPos.top}px;"
        onkeydown={(e) => { if (e.key === "Escape") closeOverlays(true); }}
      >
        {#if canPin}
          <button type="button" class="menu-item" role="menuitem" onclick={() => { closeOverlays(true); onTogglePin?.(message); }}>
            {message.pinned ? $t("omnidisc.messages.unpin") : $t("omnidisc.messages.pin")}
          </button>
        {/if}
        <button type="button" class="menu-item" role="menuitem" onclick={() => { closeOverlays(true); onMarkUnread?.(message); }}>
          {$t("omnidisc.messages.mark_unread")}
        </button>
        <button type="button" class="menu-item" role="menuitem" onclick={() => { closeOverlays(true); onCopyLink?.(message); }}>
          {$t("omnidisc.messages.copy_link")}
        </button>
        <button type="button" class="menu-item" role="menuitem" onclick={() => { closeOverlays(true); onCopyId?.(message); }}>
          {$t("omnidisc.messages.copy_id")}
        </button>
        {#if canDelete}
          <button type="button" class="menu-item danger" role="menuitem" onclick={() => { closeOverlays(true); onDelete?.(message); }}>
            {$t("omnidisc.messages.delete")}
          </button>
        {/if}
      </div>
    {/if}
  {/if}
</article>

{#snippet unverifiedBadge()}
  <span
    class="unverified"
    role="note"
    title={$t("omnidisc.e2ee.unverified_hint", { name: message.authorName })}
  >
    <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M10.3 3.6 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.6a2 2 0 0 0-3.4 0z" />
      <path d="M12 9v4M12 17h.01" />
    </svg>
    {$t("omnidisc.e2ee.unverified")}
  </span>
{/snippet}

<style>
  .day-separator,
  .unread-divider {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-4) var(--space-2);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
  }

  .day-separator::before,
  .day-separator::after,
  .unread-divider::before,
  .unread-divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--border);
  }

  .unread-divider {
    color: var(--accent);
    padding-top: var(--space-2);
  }

  .unread-divider::before,
  .unread-divider::after {
    background: var(--accent);
  }

  .message {
    position: relative;
    display: grid;
    grid-template-columns: 40px 1fr;
    column-gap: var(--space-3);
    padding: var(--space-1) var(--space-4);
    margin-top: var(--space-3);
  }

  .message.grouped {
    margin-top: 0;
  }

  .message:hover {
    background: var(--fill-1);
  }

  .message.highlighted {
    background: var(--accent-soft);
  }

  .message.editing {
    background: var(--fill-1);
  }

  .message.pending .content {
    color: var(--text-muted);
  }

  .reply-quote {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: 0 0 2px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
    overflow: hidden;
  }

  .reply-quote:disabled {
    cursor: default;
  }

  .reply-quote:hover:not(:disabled) .reply-text {
    color: var(--text);
  }

  .reply-author {
    font-weight: 600;
    color: var(--text-secondary, var(--text));
    flex: 0 0 auto;
  }

  .reply-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .reply-text.muted {
    font-style: italic;
  }

  .avatar {
    width: 40px;
    height: 40px;
    padding: 0;
    border: none;
    border-radius: var(--radius-full);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font: inherit;
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
  }

  .avatar:focus-visible,
  .author:focus-visible,
  .reply-quote:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .body {
    min-width: 0;
  }

  .meta {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .author {
    padding: 0;
    border: none;
    background: transparent;
    font: inherit;
    font-weight: 600;
    color: var(--text);
    font-size: var(--text-base);
    cursor: pointer;
  }

  .author:hover {
    text-decoration: underline;
  }

  .tag {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  /* Breaks the pattern on purpose: everything else in this header is quiet,
     so the one message whose sender cannot be confirmed is the thing the eye
     lands on. */
  .unverified {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border: 1px solid var(--danger);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--danger);
    cursor: help;
  }

  .unverified-row {
    margin-bottom: 2px;
  }

  .time {
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .time.gutter {
    align-self: start;
    padding-top: 3px;
    text-align: right;
    visibility: hidden;
  }

  .message.grouped:hover .time.gutter {
    visibility: visible;
  }

  .content.locked {
    color: var(--text-muted);
    font-style: italic;
  }

  .content {
    margin: 0;
    color: var(--text);
    font-size: var(--text-base);
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .edited {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .edit {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .edit-input {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    padding: 8px var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-base);
    line-height: 1.45;
  }

  .edit-input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .edit-actions {
    display: flex;
    gap: var(--space-2);
  }

  .status {
    grid-column: 2;
    margin-top: 2px;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .failed-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
    color: var(--danger);
  }

  .mini {
    padding: 2px var(--space-2);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .mini.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .mini.ghost {
    border-color: transparent;
    color: var(--text-muted);
  }

  .mini:hover:not(.primary) {
    border-color: var(--accent);
    color: var(--accent);
  }

  .mini:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .reactions {
    grid-column: 2;
    list-style: none;
    margin: var(--space-1) 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .reaction {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-2);
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: var(--surface);
    font: inherit;
    font-size: var(--text-xs);
    color: var(--text);
    cursor: pointer;
  }

  .reaction:disabled {
    cursor: default;
  }

  .reaction:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .reaction.me {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .reaction:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .count {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    position: absolute;
    top: -12px;
    right: var(--space-4);
    display: none;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-sm, 0 2px 6px rgba(0, 0, 0, 0.18));
  }

  .message:hover .actions,
  .message:focus-within .actions,
  .actions.open {
    display: flex;
  }

  .action {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .action:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .action:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .overlay {
    position: fixed;
    z-index: 30;
  }

  .overlay:not(.placed) {
    visibility: hidden;
  }

  .menu {
    display: flex;
    flex-direction: column;
    min-width: 180px;
    padding: var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.25));
  }

  .menu-item {
    padding: 6px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .menu-item:hover {
    background: var(--fill-1);
  }

  .menu-item.danger {
    color: var(--danger);
  }

  .menu-item:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
