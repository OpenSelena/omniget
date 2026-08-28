<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import {
    selectChannel,
    getGuild,
    canInChannel,
    isMemberListOpen,
    toggleMemberList,
    getInstance,
  } from "$lib/stores/omnidisc-store.svelte";
  import ChatView from "$components/omnidisc/ChatView.svelte";
  import VoiceChannelView from "$components/omnidisc/VoiceChannelView.svelte";

  let guildId = $derived(page.params.guild ?? "");
  let channelId = $derived(page.params.channel ?? "");

  $effect(() => {
    selectChannel(guildId, channelId);
  });

  let guild = $derived(getGuild(guildId));
  let instance = $derived(getInstance(guild?.instanceId ?? null));
  let channel = $derived(guild?.channels.find((c) => c.id === channelId) ?? null);
  let membersOpen = $derived(isMemberListOpen());
  let connected = $derived(instance?.status === "connected" || instance?.id === "demo");
  let canSend = $derived(connected && canInChannel(channelId, "SEND_MESSAGES"));
  let canManageChannel = $derived(canInChannel(channelId, "MANAGE_CHANNELS"));
</script>

{#if !channel || !instance}
  <div class="not-found">
    <h3>{$t("omnidisc.channels.not_found")}</h3>
    <p>{$t("omnidisc.channels.not_found_hint")}</p>
  </div>
{:else if channel.kind === "voice"}
  <div class="voice-wrap">
    <header class="bar">
      <h2 class="name">{channel.name}</h2>
    </header>
    <VoiceChannelView {channel} />
  </div>
{:else}
  <ChatView
    instanceId={instance.id}
    {channelId}
    {guildId}
    title={channel.name}
    topic={channel.topic ?? ""}
    kind="text"
    {canSend}
    {membersOpen}
    onToggleMembers={toggleMemberList}
  >
    {#snippet headerExtra()}
      {#if canManageChannel}
        <a
          class="icon-link"
          href={`/omnidisc/g/${guildId}/settings?channel=${channelId}`}
          title={$t("omnidisc.channel_settings.title")}
          aria-label={$t("omnidisc.channel_settings.title")}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1A1.6 1.6 0 0 0 9 19.4a1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1A1.6 1.6 0 0 0 4.6 9a1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
        </a>
      {/if}
    {/snippet}
  </ChatView>
{/if}

<style>
  .voice-wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    min-height: 48px;
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }

  .name {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
  }

  .icon-link {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }

  .icon-link:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .icon-link:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .not-found {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    padding: var(--space-5);
    text-align: center;
  }

  .not-found h3 {
    margin: 0;
    font-size: var(--text-lg);
    color: var(--text);
  }

  .not-found p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>
