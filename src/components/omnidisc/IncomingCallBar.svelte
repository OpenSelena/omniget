<script lang="ts">
  import { t } from "$lib/i18n";
  import { getChannel, dmTitle, userName } from "$lib/stores/omnidisc-store.svelte";
  import {
    getIncomingCall,
    ringSecondsLeft,
    acceptIncomingCall,
    dismissIncomingCall,
    isVoiceBusy,
  } from "$lib/stores/omnidisc-voice-store.svelte";

  let call = $derived(getIncomingCall());
  let busy = $derived(isVoiceBusy());
  let seconds = $derived(ringSecondsLeft());
  let channel = $derived(getChannel(call?.channelId ?? null));
  let caller = $derived(call ? userName(call.instanceId, call.fromUserId) : "");
  let group = $derived((channel?.recipientIds?.length ?? 0) > 2);
  let where = $derived(channel ? dmTitle(channel) : "");
</script>

{#if call}
  <section class="ring" aria-label={$t("omnidisc.call.incoming", { name: caller })} aria-live="assertive">
    <div class="who">
      <span class="pulse" aria-hidden="true"></span>
      <div class="lines">
        <span class="headline">
          {group
            ? $t("omnidisc.call.incoming_group", { name: caller, channel: where })
            : $t("omnidisc.call.incoming", { name: caller })}
        </span>
        <span class="countdown">{$t("omnidisc.call.expires_in", { seconds })}</span>
      </div>
    </div>
    <div class="actions">
      <button type="button" class="btn answer" disabled={busy} onclick={() => void acceptIncomingCall()}>
        {busy ? $t("omnidisc.call.joining") : $t("omnidisc.call.accept")}
      </button>
      <button type="button" class="btn" disabled={busy} onclick={dismissIncomingCall}>
        {$t("omnidisc.call.decline")}
      </button>
    </div>
  </section>
{/if}

<style>
  .ring {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid color-mix(in srgb, var(--accent) 40%, var(--border));
    background: color-mix(in srgb, var(--accent) 10%, var(--surface));
  }

  .who {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .pulse {
    width: 8px;
    height: 8px;
    flex: 0 0 8px;
    border-radius: var(--radius-full);
    background: var(--accent);
    animation: ring-pulse 1.4s ease-in-out infinite;
  }

  .lines {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .headline {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .countdown {
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .btn {
    flex: 1;
    height: 30px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-mut);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .btn.answer {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @keyframes ring-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
</style>
