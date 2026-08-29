<script lang="ts">
  import { tick } from "svelte";
  import { t } from "$lib/i18n";
  import type { OmnidiscMessage } from "$lib/omnidisc/types";
  import VirtualList from "./VirtualList.svelte";
  import MessageItem from "./MessageItem.svelte";

  const GROUP_WINDOW_MS = 5 * 60_000;

  let {
    messages,
    loading = false,
    firstUnreadId = null,
    highlightId = null,
    editingId = null,
    canReact = false,
    canPin = false,
    canReply = false,
    canEditOf = () => false,
    canDeleteOf = () => false,
    onLoadOlder,
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
    onJumpToMessage,
    onOpenProfile,
    onHighlightShown,
  }: {
    messages: OmnidiscMessage[];
    loading?: boolean;
    firstUnreadId?: string | null;
    highlightId?: string | null;
    editingId?: string | null;
    canReact?: boolean;
    canPin?: boolean;
    canReply?: boolean;
    canEditOf?: (message: OmnidiscMessage) => boolean;
    canDeleteOf?: (message: OmnidiscMessage) => boolean;
    onLoadOlder?: () => void;
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
    onJumpToMessage?: (messageId: string) => void;
    onOpenProfile?: (userId: string, anchor: HTMLElement) => void;
    onHighlightShown?: () => void;
  } = $props();

  type Row = { message: OmnidiscMessage; showHeader: boolean; daySeparator: boolean; unreadDivider: boolean };

  let list = $state<ReturnType<typeof VirtualList> | null>(null);
  let atBottom = $state(true);

  function sameDay(a: number, b: number): boolean {
    const da = new Date(a);
    const db = new Date(b);
    return da.getFullYear() === db.getFullYear() && da.getMonth() === db.getMonth() && da.getDate() === db.getDate();
  }

  let byId = $derived(new Map(messages.map((m) => [m.id, m])));

  let rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    let prev: OmnidiscMessage | null = null;
    for (const message of messages) {
      const daySeparator = !prev || !sameDay(prev.createdAt, message.createdAt);
      const showHeader =
        daySeparator ||
        !prev ||
        prev.authorId !== message.authorId ||
        message.createdAt - prev.createdAt > GROUP_WINDOW_MS;
      out.push({ message, showHeader, daySeparator, unreadDivider: message.id === firstUnreadId });
      prev = message;
    }
    return out;
  });

  const rowKey = (row: Row) => row.message.id;

  $effect(() => {
    const target = highlightId;
    if (!target) return;
    tick().then(() => {
      list?.scrollToKey(target);
      onHighlightShown?.();
    });
  });
</script>

<div class="message-list" aria-busy={loading}>
  {#if loading && messages.length === 0}
    <div class="skeleton" aria-label={$t("omnidisc.messages.loading")}>
      {#each Array(8) as _, i (i)}
        <div class="skeleton-row">
          <span class="skeleton-avatar"></span>
          <span class="skeleton-lines">
            <span class="skeleton-line short"></span>
            <span class="skeleton-line"></span>
          </span>
        </div>
      {/each}
    </div>
  {:else}
    <VirtualList
      bind:this={list}
      items={rows}
      getKey={rowKey}
      onReachTop={onLoadOlder}
      onBottomChange={(v) => (atBottom = v)}
    >
      {#snippet row(item)}
        <MessageItem
          message={item.message}
          showHeader={item.showHeader}
          daySeparator={item.daySeparator}
          unreadDivider={item.unreadDivider}
          highlighted={item.message.id === highlightId}
          editing={item.message.id === editingId}
          replyPreview={item.message.replyToId ? (byId.get(item.message.replyToId) ?? null) : null}
          canEdit={canEditOf(item.message)}
          canDelete={canDeleteOf(item.message)}
          {canPin}
          {canReact}
          {onRetry}
          {onDiscard}
          {onReact}
          onReply={canReply ? onReply : undefined}
          {onStartEdit}
          {onSubmitEdit}
          {onCancelEdit}
          {onDelete}
          {onTogglePin}
          {onCopyLink}
          {onCopyId}
          {onMarkUnread}
          onJumpToReply={onJumpToMessage}
          {onOpenProfile}
        />
      {/snippet}
      {#snippet empty()}
        <div class="empty">
          <h3>{$t("omnidisc.messages.empty_title")}</h3>
          <p>{$t("omnidisc.messages.empty_body")}</p>
        </div>
      {/snippet}
    </VirtualList>
    {#if !atBottom && messages.length > 0}
      <button type="button" class="present" onclick={() => list?.scrollToBottom()}>
        {$t("omnidisc.messages.jump_present")}
        <span aria-hidden="true">↓</span>
      </button>
    {/if}
  {/if}
</div>

<style>
  .message-list {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .present {
    position: absolute;
    right: var(--space-4);
    bottom: var(--space-3);
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    box-shadow: var(--shadow-sm, 0 2px 6px rgba(0, 0, 0, 0.2));
  }

  .present:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: var(--space-5) var(--space-4);
    gap: var(--space-1);
  }

  .empty h3 {
    margin: 0;
    font-size: var(--text-lg);
    color: var(--text);
  }

  .empty p {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .skeleton {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: var(--space-4);
    padding: var(--space-4);
    flex: 1;
  }

  .skeleton-row {
    display: grid;
    grid-template-columns: 40px 1fr;
    gap: var(--space-3);
  }

  .skeleton-avatar {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--fill-2);
  }

  .skeleton-lines {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-1);
  }

  .skeleton-line {
    height: 12px;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    width: 70%;
    animation: pulse 1.4s ease-in-out infinite;
  }

  .skeleton-line.short {
    width: 30%;
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
