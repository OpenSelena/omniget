<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    canDeleteMessage,
    canEditMessage,
    canInChannel,
    clearJumpTarget,
    deleteMessage,
    discardMessage,
    editMessage,
    effectiveNotificationLevel,
    getFirstUnreadId,
    getJumpTarget,
    getMessages,
    getNotificationLevel,
    getGroupStatus,
    getTypingNames,
    hostLabel,
    isChannelLoading,
    isEncryptedChannel,
    jumpToMessage,
    loadOlderMessages,
    markUnread,
    messageLink,
    notifyTyping,
    retryMessage,
    sendMessage,
    setNotificationLevel,
    refreshGroupStatus,
    setPinned,
    toggleReaction,
    getChannel,
    getInstance,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { NotificationLevel, OmnidiscMessage } from "$lib/omnidisc/types";
  import MessageList from "./MessageList.svelte";
  import Composer from "./Composer.svelte";
  import SearchPanel from "./SearchPanel.svelte";
  import PinsPanel from "./PinsPanel.svelte";
  import ProfilePopover from "./ProfilePopover.svelte";
  import OmnidiscPrompt from "./OmnidiscPrompt.svelte";

  let {
    instanceId,
    channelId,
    guildId = null,
    title,
    topic = "",
    kind = "text",
    canSend = false,
    membersOpen = false,
    onToggleMembers,
    headerExtra,
  }: {
    instanceId: string;
    channelId: string;
    guildId?: string | null;
    title: string;
    topic?: string;
    kind?: "text" | "dm";
    canSend?: boolean;
    membersOpen?: boolean;
    onToggleMembers?: () => void;
    headerExtra?: import("svelte").Snippet;
  } = $props();

  let panel = $state<"none" | "search" | "pins">("none");
  let notifyOpen = $state(false);
  let notifyEl = $state<HTMLElement | null>(null);
  let replyTo = $state<OmnidiscMessage | null>(null);
  let editingId = $state<string | null>(null);
  let pendingDelete = $state<OmnidiscMessage | null>(null);
  let actionError = $state<string | null>(null);
  let deleteBusy = $state(false);
  let highlightId = $state<string | null>(null);
  let profile = $state<{ userId: string; x: number; y: number } | null>(null);

  let messages = $derived(getMessages(channelId));
  let loading = $derived(isChannelLoading(channelId));
  let typingNames = $derived(getTypingNames(channelId));
  let draftKey = $derived(guildId ? `${guildId}/${channelId}` : `dm/${channelId}`);
  let firstUnreadId = $derived(getFirstUnreadId(channelId));
  let canReact = $derived(canSend && canInChannel(channelId, "ADD_REACTIONS"));
  let canPin = $derived(canInChannel(channelId, "PIN_MESSAGES"));
  let level = $derived(effectiveNotificationLevel(channelId));
  let encrypted = $derived(isEncryptedChannel(channelId));
  let groupStatus = $derived(getGroupStatus(channelId));
  let privacyText = $derived.by(() => {
    if (!encrypted) {
      const url = getInstance(instanceId)?.url ?? "";
      return $t("omnidisc.e2ee.guild_open", { host: url ? hostLabel(url) : "" });
    }
    if (!groupStatus?.ready) return $t("omnidisc.e2ee.preparing");
    const recipients = getChannel(channelId)?.recipientIds ?? [];
    return recipients.length === 1
      ? $t("omnidisc.e2ee.dm_locked", { name: title })
      : $t("omnidisc.e2ee.group_locked");
  });

  $effect(() => {
    if (encrypted) void refreshGroupStatus(channelId);
  });
  let levelExplicit = $derived(getNotificationLevel(channelId));

  $effect(() => {
    const target = getJumpTarget();
    if (!target || target.channelId !== channelId) return;
    highlightId = target.messageId;
    clearJumpTarget();
  });

  $effect(() => {
    channelId;
    replyTo = null;
    editingId = null;
    panel = "none";
    profile = null;
  });

  function fail(e: unknown) {
    actionError = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  function send(text: string) {
    const reference = replyTo?.id;
    replyTo = null;
    void sendMessage(channelId, text, reference);
  }

  function react(message: OmnidiscMessage, emoji: string) {
    toggleReaction(channelId, message.id, emoji).catch(fail);
  }

  function submitEdit(message: OmnidiscMessage, content: string) {
    editingId = null;
    editMessage(channelId, message.id, content).catch(fail);
  }

  function editLast() {
    const mine = [...messages].reverse().find((m) => canEditMessage(m));
    if (mine) editingId = mine.id;
  }

  async function confirmDelete() {
    const message = pendingDelete;
    if (!message) return;
    deleteBusy = true;
    actionError = null;
    try {
      await deleteMessage(channelId, message.id);
      pendingDelete = null;
    } catch (e) {
      fail(e);
    } finally {
      deleteBusy = false;
    }
  }

  async function copy(text: string, failKey: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      actionError = $t(failKey);
    }
  }

  function openProfile(userId: string, anchor: HTMLElement) {
    const rect = anchor.getBoundingClientRect();
    const width = typeof window !== "undefined" ? window.innerWidth : 1280;
    const height = typeof window !== "undefined" ? window.innerHeight : 800;
    profile = {
      userId,
      x: Math.min(rect.left, width - 320),
      y: Math.min(rect.bottom + 6, height - 420),
    };
  }

  function pickLevel(next: NotificationLevel | null) {
    setNotificationLevel(channelId, next);
    notifyOpen = false;
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (!notifyOpen) return;
    if (notifyEl && e.target instanceof Node && notifyEl.contains(e.target)) return;
    notifyOpen = false;
  }

  function jump(target: string, targetChannel = channelId) {
    if (targetChannel !== channelId) {
      void jumpToMessage(targetChannel, target);
      return;
    }
    void jumpToMessage(channelId, target);
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div class="chat-view">
  <header class="bar">
    <h2 class="name">
      <span class="glyph" aria-hidden="true">{kind === "dm" ? "@" : "#"}</span>
      {title}
    </h2>
    {#if topic}
      <p class="topic" title={topic}>{topic}</p>
    {/if}
    <div class="bar-actions">
      {@render headerExtra?.()}
      <button
        type="button"
        class="icon-button"
        class:on={panel === "search"}
        onclick={() => (panel = panel === "search" ? "none" : "search")}
        aria-pressed={panel === "search"}
        title={$t("omnidisc.search.title")}
        aria-label={$t("omnidisc.search.title")}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg>
      </button>
      <button
        type="button"
        class="icon-button"
        class:on={panel === "pins"}
        onclick={() => (panel = panel === "pins" ? "none" : "pins")}
        aria-pressed={panel === "pins"}
        title={$t("omnidisc.pins.title")}
        aria-label={$t("omnidisc.pins.title")}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 4h6l-1 6 4 3v2H6v-2l4-3z" /><path d="M12 15v5" /></svg>
      </button>
      <div class="notify" bind:this={notifyEl}>
        <button
          type="button"
          class="icon-button"
          class:on={notifyOpen}
          onclick={() => (notifyOpen = !notifyOpen)}
          aria-expanded={notifyOpen}
          title={$t("omnidisc.notify.title")}
          aria-label={$t("omnidisc.notify.title")}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" /><path d="M13.7 21a2 2 0 0 1-3.4 0" /></svg>
        </button>
        {#if notifyOpen}
          <div class="menu" role="menu" aria-label={$t("omnidisc.notify.title")}>
            <p class="menu-hint">{$t("omnidisc.notify.channel_hint")}</p>
            <button type="button" class="menu-item" class:sel={levelExplicit === null} role="menuitem" onclick={() => pickLevel(null)}>
              {$t("omnidisc.notify.inherit", { level: $t(`omnidisc.notify.level_${level}`) })}
            </button>
            <button type="button" class="menu-item" class:sel={levelExplicit === "all"} role="menuitem" onclick={() => pickLevel("all")}>{$t("omnidisc.notify.level_all")}</button>
            <button type="button" class="menu-item" class:sel={levelExplicit === "mentions"} role="menuitem" onclick={() => pickLevel("mentions")}>{$t("omnidisc.notify.level_mentions")}</button>
            <button type="button" class="menu-item" class:sel={levelExplicit === "nothing"} role="menuitem" onclick={() => pickLevel("nothing")}>{$t("omnidisc.notify.level_nothing")}</button>
          </div>
        {/if}
      </div>
      {#if onToggleMembers}
        <button
          type="button"
          class="icon-button"
          class:on={membersOpen}
          onclick={onToggleMembers}
          aria-pressed={membersOpen}
          aria-label={membersOpen ? $t("omnidisc.members.hide") : $t("omnidisc.members.show")}
          title={membersOpen ? $t("omnidisc.members.hide") : $t("omnidisc.members.show")}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </button>
      {/if}
    </div>
  </header>

  {#if actionError}
    <p class="action-error" role="alert">
      {actionError}
      <button type="button" class="dismiss" onclick={() => (actionError = null)}>{$t("common.close")}</button>
    </p>
  {/if}

  <div class="body">
    <div class="stream">
      <MessageList
        {messages}
        {loading}
        {firstUnreadId}
        {highlightId}
        {editingId}
        {canReact}
        canPin={canPin}
        canReply={canSend}
        canEditOf={canEditMessage}
        canDeleteOf={canDeleteMessage}
        onLoadOlder={() => {
          void loadOlderMessages(channelId);
        }}
        onRetry={(m) => {
          void retryMessage(channelId, m.id);
        }}
        onDiscard={(m) => discardMessage(channelId, m.id)}
        onReact={react}
        onReply={(m) => (replyTo = m)}
        onStartEdit={(m) => (editingId = m.id)}
        onSubmitEdit={submitEdit}
        onCancelEdit={() => (editingId = null)}
        onDelete={(m) => {
          actionError = null;
          pendingDelete = m;
        }}
        onTogglePin={(m) => {
          setPinned(channelId, m.id, !m.pinned).catch(fail);
        }}
        onCopyLink={(m) => void copy(messageLink(channelId, m.id), "omnidisc.messages.copy_failed")}
        onCopyId={(m) => void copy(m.id, "omnidisc.messages.copy_failed")}
        onMarkUnread={(m) => markUnread(channelId, m.id)}
        onJumpToMessage={(id) => jump(id)}
        onOpenProfile={openProfile}
        onHighlightShown={() => {
          setTimeout(() => (highlightId = null), 1800);
        }}
      />
      <p class="privacy" class:locked={encrypted}>
        {#if encrypted}
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><rect x="4" y="10" width="16" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /></svg>
          <span>{privacyText}</span>
        {:else}
          <span>{privacyText}</span>
        {/if}
      </p>
      <Composer
        {channelId}
        canAttach={canSend && canInChannel(channelId, "ATTACH_FILES")}
        {draftKey}
        placeholder={kind === "dm"
          ? $t("omnidisc.dm.placeholder", { name: title })
          : $t("omnidisc.composer.placeholder", { channel: title })}
        disabled={!canSend}
        {typingNames}
        {replyTo}
        onSend={send}
        onTyping={() => notifyTyping(channelId)}
        onCancelReply={() => (replyTo = null)}
        onEditLast={editLast}
      />
    </div>

    {#if panel === "search"}
      <SearchPanel
        {instanceId}
        {guildId}
        {channelId}
        onJump={(ch, id) => jump(id, ch)}
        onClose={() => (panel = "none")}
      />
    {:else if panel === "pins"}
      <PinsPanel {instanceId} {channelId} {canPin} onJump={(id) => jump(id)} onClose={() => (panel = "none")} />
    {/if}
  </div>
</div>

{#if profile}
  <ProfilePopover
    {instanceId}
    userId={profile.userId}
    {guildId}
    x={profile.x}
    y={profile.y}
    onClose={() => (profile = null)}
  />
{/if}

<OmnidiscPrompt
  open={pendingDelete !== null}
  title={$t("omnidisc.messages.delete_title")}
  body={$t("omnidisc.messages.delete_body")}
  submitLabel={$t("omnidisc.messages.delete_confirm")}
  busy={deleteBusy}
  error={actionError}
  onSubmit={() => void confirmDelete()}
  onClose={() => {
    pendingDelete = null;
    actionError = null;
  }}
>
  {#if pendingDelete}
    <blockquote class="preview">
      <strong>{pendingDelete.authorName}</strong>
      <span>{pendingDelete.content.slice(0, 240) || $t("omnidisc.messages.delete_no_text")}</span>
    </blockquote>
  {/if}
</OmnidiscPrompt>

<style>
  .privacy {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: var(--space-2) var(--space-4) 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .privacy.locked {
    color: var(--success);
  }

  .chat-view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 48px;
    padding: 0 var(--space-4);
    border-bottom: none;
  }

  .name {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 0 0 auto;
  }

  .glyph {
    color: var(--text-muted);
  }

  .topic {
    flex: 1;
    margin: 0;
    padding-left: var(--space-3);
    border-left: none;
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .notify {
    position: relative;
  }

  .icon-button {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .icon-button:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .icon-button.on {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .icon-button:focus-visible,
  .menu-item:focus-visible,
  .dismiss:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .menu {
    position: absolute;
    right: 0;
    top: 36px;
    z-index: 40;
    min-width: 210px;
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.25));
  }

  .menu-hint {
    margin: var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-muted);
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

  .menu-item.sel {
    color: var(--accent);
    font-weight: 600;
  }

  .action-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-2) var(--space-4);
    background: var(--danger-soft, var(--surface));
    color: var(--danger);
    font-size: var(--text-xs);
  }

  .dismiss {
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
  }

  .stream {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .preview {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--border-hi);
    background: var(--fill-1);
    border-radius: var(--radius-sm);
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-sm);
    color: var(--text);
    overflow-wrap: anywhere;
  }

  @media (max-width: 1100px) {
    .body :global(.search-panel),
    .body :global(.pins-panel) {
      display: none;
    }
  }
</style>
