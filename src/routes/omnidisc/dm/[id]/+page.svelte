<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import {
    selectChannel,
    getChannel,
    dmTitle,
    getSelectedInstance,
  } from "$lib/stores/omnidisc-store.svelte";
  import ChatView from "$components/omnidisc/ChatView.svelte";
  import {
    getVoiceSession,
    getVoiceState,
    isVoiceBusy,
    joinVoice,
    leaveVoice,
  } from "$lib/stores/omnidisc-voice-store.svelte";

  let channelId = $derived(page.params.id ?? "");
  let channel = $derived(getChannel(channelId));
  let instance = $derived(getSelectedInstance());
  let title = $derived(channel ? dmTitle(channel) : "");
  let canSend = $derived(instance?.status === "connected");
  let session = $derived(getVoiceSession());
  let connState = $derived(getVoiceState());
  let voiceBusy = $derived(isVoiceBusy());
  let inThisCall = $derived(session?.channelId === channelId && connState !== "idle");
  let inAnotherCall = $derived(!!session && session.channelId !== channelId && connState !== "idle");

  $effect(() => {
    selectChannel(null, channelId);
  });
</script>

{#if !channel || !instance}
  <div class="not-found">
    <h3>{$t("omnidisc.dm.not_found")}</h3>
    <p>{$t("omnidisc.dm.not_found_hint")}</p>
  </div>
{:else}
  <ChatView
    instanceId={instance.id}
    {channelId}
    title={title || $t("omnidisc.dm.title")}
    kind="dm"
    {canSend}
  >
    {#snippet headerExtra()}
      <button
        type="button"
        class="icon-button"
        class:leave={inThisCall}
        disabled={!canSend || voiceBusy || inAnotherCall}
        onclick={() => (inThisCall ? void leaveVoice() : void joinVoice(channelId))}
        aria-label={inThisCall ? $t("omnidisc.call.leave") : $t("omnidisc.call.start")}
        title={inThisCall ? $t("omnidisc.call.leave") : $t("omnidisc.call.start_hint")}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M10.7 13.3a15 15 0 0 1-2-2.6l1.4-1.4a1 1 0 0 0 .2-1L9.5 4.8a1 1 0 0 0-1-.8H5a1 1 0 0 0-1 1c0 3.5 1.3 6.8 3.5 9.4" />
          <path d="M13.3 10.7c.9.8 1.8 1.5 2.7 2l1.4-1.4a1 1 0 0 1 1-.2l3.5.8a1 1 0 0 1 .8 1V16a1 1 0 0 1-1 1 16 16 0 0 1-9.4-3.5" />
          {#if inThisCall}<path d="M4 20L20 4" />{/if}
        </svg>
      </button>
    {/snippet}
  </ChatView>
{/if}

<style>
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

  .icon-button:hover:not(:disabled) {
    background: var(--fill-1);
    color: var(--text);
  }

  .icon-button.leave {
    color: var(--danger);
  }

  .icon-button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .icon-button:focus-visible {
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
