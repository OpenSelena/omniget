<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { isMac, modKey } from "$lib/platform";
  import { getSettings } from "$lib/stores/settings-store.svelte";
  import { getChannel, getGuild } from "$lib/stores/omnidisc-store.svelte";
  import {
    getVoiceSession,
    getVoiceState,
    getVoiceError,
    getMicError,
    getOutputError,
    getVoiceQuality,
    getVoiceStats,
    isMuted,
    isDeafened,
    isVoiceBusy,
    isE2ee,
    toggleMute,
    toggleDeafen,
    leaveVoice,
    retryVoice,
  } from "$lib/stores/omnidisc-voice-store.svelte";
  import { isPublishing, stopStream, isStreamBusy } from "$lib/stores/omnidisc-stream-store.svelte";
  import ShareScreenDialog from "$components/omnidisc/ShareScreenDialog.svelte";

  let shareOpen = $state(false);
  let publishing = $derived(isPublishing());
  let streamBusy = $derived(isStreamBusy());

  let session = $derived(getVoiceSession());
  let connState = $derived(getVoiceState());
  let channel = $derived(getChannel(session?.channelId ?? null));
  let guild = $derived(getGuild(session?.guildId ?? null));
  let muted = $derived(isMuted());
  let deafened = $derived(isDeafened());
  let busy = $derived(isVoiceBusy());
  let quality = $derived(getVoiceQuality());
  let stats = $derived(getVoiceStats());
  let error = $derived(getVoiceError());
  let encrypted = $derived(isE2ee());
  let micError = $derived(getMicError());
  let outputError = $derived(getOutputError());
  let ping = $derived(stats?.rtt_ms != null ? Math.round(stats.rtt_ms) : null);
  let pttKey = $derived(getSettings()?.omnidisc?.voice?.ptt_key ?? "");
  let pttLabel = $derived(
    pttKey
      ? pttKey
          .split("+")
          .map((part) => (part === "CmdOrCtrl" ? modKey() : part))
          .join(isMac() ? "" : "+")
      : "",
  );

  let stateLabel = $derived.by(() => {
    switch (connState) {
      case "connecting":
        return $t("omnidisc.voice.connecting");
      case "reconnecting":
        return $t("omnidisc.voice.reconnecting");
      case "failed":
        return $t("omnidisc.voice.failed");
      default:
        return $t("omnidisc.voice.connected_to");
    }
  });

  let qualityLabel = $derived.by(() => {
    if (connState === "reconnecting") return $t("omnidisc.voice.quality_lost");
    switch (quality) {
      case "excellent":
        return $t("omnidisc.voice.quality_excellent");
      case "good":
        return $t("omnidisc.voice.quality_good");
      case "poor":
        return $t("omnidisc.voice.quality_poor");
      case "lost":
        return $t("omnidisc.voice.quality_lost");
      default:
        return $t("omnidisc.voice.quality_unknown");
    }
  });

  let dotClass = $derived.by(() => {
    if (connState === "failed") return "bad";
    if (connState === "connecting" || connState === "reconnecting") return "wait";
    if (quality === "poor") return "warn";
    if (quality === "lost") return "bad";
    if (quality === "unknown") return "wait";
    return "ok";
  });

  function translate(code: string | null): string {
    return code ? translateBackendError(code, $t) : "";
  }
</script>

{#if session && connState !== "idle"}
  <section class="voice-bar" class:failed={connState === "failed"} aria-label={$t("omnidisc.voice.title")} aria-live="polite">
    <div class="status-row">
      <span class="dot {dotClass}" title={qualityLabel} aria-hidden="true"></span>
      <div class="status-text">
        <span class="state">{stateLabel}</span>
        <span class="where">
          <span
            class="lock"
            class:open={!encrypted}
            title={encrypted ? $t("omnidisc.voice.e2ee_on_desc") : $t("omnidisc.voice.e2ee_off_desc")}
          >
            <svg viewBox="0 0 24 24" width="11" height="11" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="5" y="11" width="14" height="9" rx="2" />
              {#if encrypted}
                <path d="M8 11V8a4 4 0 0 1 8 0v3" />
              {:else}
                <path d="M8 11V8a4 4 0 0 1 7.5-2" />
              {/if}
            </svg>
            <span class="sr-only">{encrypted ? $t("omnidisc.voice.e2ee_on") : $t("omnidisc.voice.e2ee_off")}</span>
          </span>
          {#if channel}{channel.name}{/if}{#if guild} · {guild.name}{/if}
        </span>
        {#if pttLabel && connState === "connected" && !muted}
          <span class="where">{$t("omnidisc.voice.ptt_hold", { key: pttLabel })}</span>
        {/if}
      </div>
      {#if ping !== null && connState === "connected"}
        <span class="ping" title={qualityLabel}>{$t("omnidisc.voice.ping", { ms: ping })}</span>
      {/if}
      <button
        type="button"
        class="icon-btn small"
        onclick={() => goto("/omnidisc/settings?view=voice")}
        aria-label={$t("omnidisc.voice.open_settings")}
        title={$t("omnidisc.voice.open_settings")}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
        </svg>
      </button>
    </div>

    {#if connState === "failed"}
      <p class="note bad" role="alert">{translate(error ?? "ERR_VOICE_DISCONNECTED")}</p>
      <div class="actions">
        <button type="button" class="btn primary" onclick={() => void retryVoice()} disabled={busy}>{$t("omnidisc.voice.retry")}</button>
        <button type="button" class="btn" onclick={() => void leaveVoice()} disabled={busy}>{$t("omnidisc.voice.leave_short")}</button>
      </div>
    {:else}
      {#if micError}
        <p class="note warn" role="status">
          {$t("omnidisc.voice.listen_only")}
          <button type="button" class="link" onclick={() => goto("/omnidisc/settings?view=voice")}>{$t("omnidisc.voice.open_settings")}</button>
        </p>
      {/if}
      {#if outputError}
        <p class="note warn" role="status">
          {$t("omnidisc.voice.no_output")}
          <button type="button" class="link" onclick={() => goto("/omnidisc/settings?view=voice")}>{$t("omnidisc.voice.open_settings")}</button>
        </p>
      {/if}
      <div class="share-row">
        {#if publishing}
          <button type="button" class="share stop" onclick={() => void stopStream()} disabled={streamBusy}>
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="18" height="13" rx="2" /><path d="M9 10l6 4M15 10l-6 4" /></svg>
            {$t("omnidisc.stream.stop_sharing")}
          </button>
        {:else}
          <button type="button" class="share" onclick={() => (shareOpen = true)} disabled={connState !== "connected"}>
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="18" height="13" rx="2" /><path d="M12 8v5M9.5 10.5 12 8l2.5 2.5" /></svg>
            {$t("omnidisc.stream.share_screen")}
          </button>
        {/if}
      </div>

      <div class="actions">
        <button
          type="button"
          class="icon-btn"
          class:on={muted}
          aria-pressed={muted}
          aria-label={muted ? $t("omnidisc.voice.unmute") : $t("omnidisc.voice.mute")}
          title={muted ? $t("omnidisc.voice.unmute") : $t("omnidisc.voice.mute")}
          disabled={busy || !!micError}
          onclick={() => void toggleMute()}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <rect x="9" y="3" width="6" height="11" rx="3" />
            <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
            {#if muted}<path d="M4 4l16 16" />{/if}
          </svg>
        </button>
        <button
          type="button"
          class="icon-btn"
          class:on={deafened}
          aria-pressed={deafened}
          aria-label={deafened ? $t("omnidisc.voice.undeafen") : $t("omnidisc.voice.deafen")}
          title={deafened ? $t("omnidisc.voice.undeafen") : $t("omnidisc.voice.deafen")}
          disabled={busy}
          onclick={() => void toggleDeafen()}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M4 14v-2a8 8 0 0 1 16 0v2" />
            <rect x="3" y="14" width="4" height="6" rx="1" />
            <rect x="17" y="14" width="4" height="6" rx="1" />
            {#if deafened}<path d="M4 4l16 16" />{/if}
          </svg>
        </button>
        <button
          type="button"
          class="icon-btn leave"
          aria-label={$t("omnidisc.voice.leave")}
          title={$t("omnidisc.voice.leave")}
          disabled={busy}
          onclick={() => void leaveVoice()}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M10.7 13.3a15 15 0 0 1-2-2.6l1.4-1.4a1 1 0 0 0 .2-1L9.5 4.8a1 1 0 0 0-1-.8H5a1 1 0 0 0-1 1c0 3.5 1.3 6.8 3.5 9.4" />
            <path d="M13.3 10.7c.9.8 1.8 1.5 2.7 2l1.4-1.4a1 1 0 0 1 1-.2l3.5.8a1 1 0 0 1 .8 1V16a1 1 0 0 1-1 1 16 16 0 0 1-9.4-3.5" />
            <path d="M4 20L20 4" />
          </svg>
        </button>
      </div>
    {/if}
  </section>
{/if}

{#if shareOpen}
  <ShareScreenDialog onClose={() => (shareOpen = false)} />
{/if}

<style>
  .share-row {
    display: flex;
  }

  .share {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-mut);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .share:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .share.stop {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
    color: var(--danger);
  }

  .share:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .share:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .voice-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--surface);
  }

  .voice-bar.failed {
    border-top-color: color-mix(in srgb, var(--danger) 40%, var(--border));
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: 0 0 8px;
    border-radius: var(--radius-full);
    background: var(--text-dim);
  }

  .dot.ok {
    background: var(--success);
  }

  .dot.warn {
    background: var(--warning);
  }

  .dot.bad {
    background: var(--danger);
  }

  .dot.wait {
    background: var(--text-muted);
  }

  .status-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .state {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--success);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .failed .state {
    color: var(--danger);
  }

  .where {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lock {
    display: inline-flex;
    flex: 0 0 auto;
    color: var(--text-secondary);
  }

  .lock.open {
    color: var(--text-muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }

  .ping {
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .icon-btn {
    flex: 1;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-mut);
    color: var(--text);
    cursor: pointer;
  }

  .icon-btn.small {
    flex: 0 0 26px;
    width: 26px;
    height: 26px;
    border: none;
    background: transparent;
    color: var(--text-muted);
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .icon-btn.on {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
    color: var(--danger);
  }

  .icon-btn.leave {
    color: var(--danger);
  }

  .icon-btn.leave:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .icon-btn:focus-visible,
  .btn:focus-visible,
  .link:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .note {
    margin: 0;
    font-size: var(--text-xs);
    line-height: 1.45;
    color: var(--text);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
  }

  .note.warn {
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }

  .note.bad {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }

  .link {
    border: none;
    background: transparent;
    padding: 0;
    font: inherit;
    font-size: var(--text-xs);
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
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

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
