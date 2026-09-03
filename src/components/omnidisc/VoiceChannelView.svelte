<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { getVoiceMembers, getInstance, getGuild, getChannel, userName } from "$lib/stores/omnidisc-store.svelte";
  import {
    getVoiceSession,
    getVoiceState,
    getVoiceError,
    isVoiceBusy,
    isSpeaking,
    joinVoice,
    leaveVoice,
    clearVoiceError,
  } from "$lib/stores/omnidisc-voice-store.svelte";
  import {
    isPublishing,
    isWatching,
    watchStream,
    unwatchStream,
    setStreamVolume,
    isStreamBusy,
    refreshStreamStats,
    getStreamStats,
    getMediaCapabilities,
    isStreamer,
    getStreamPreview,
  } from "$lib/stores/omnidisc-stream-store.svelte";
  import type { OmnidiscChannel } from "$lib/omnidisc/types";

  let { channel }: { channel: OmnidiscChannel } = $props();

  let guild = $derived(getGuild(channel.guildId ?? null));
  let instance = $derived(getInstance(guild?.instanceId ?? null));
  let me = $derived(instance?.userId ?? null);
  let members = $derived(getVoiceMembers(channel.id));
  let session = $derived(getVoiceSession());
  let connState = $derived(getVoiceState());
  let here = $derived(session?.channelId === channel.id && connState !== "idle");
  let elsewhere = $derived(!!session && session.channelId !== channel.id && connState !== "idle");
  let busy = $derived(isVoiceBusy());
  let error = $derived(getVoiceError());
  let canJoin = $derived(instance?.status === "connected");
  let currentName = $derived(getChannel(session?.channelId ?? null)?.name ?? "");
  let confirmSwitch = $state(false);
  let publishing = $derived(isPublishing());
  let streamBusy = $derived(isStreamBusy());
  let canWatch = $derived(getMediaCapabilities().stream_viewer);
  let inspectorOpen = $state(false);
  let volumes = $state<Record<string, number>>({});

  let stats = $derived(getStreamStats());

  $effect(() => {
    if (!inspectorOpen) return;
    const tick = () => void refreshStreamStats();
    tick();
    const timer = setInterval(tick, 2000);
    return () => clearInterval(timer);
  });

  function initial(name: string): string {
    return name.trim().charAt(0).toUpperCase() || "?";
  }

  function isLive(userId: string): boolean {
    const m = members.find((x) => x.userId === userId);
    if (!m) return false;
    return m.streaming || isStreamer(userId) || (userId === me && publishing);
  }

  async function toggleWatch(userId: string) {
    if (isWatching(userId)) await unwatchStream(userId);
    else await watchStream(userId);
  }

  function volumeOf(userId: string): number {
    return volumes[userId] ?? 100;
  }

  function onVolume(userId: string, value: number) {
    volumes = { ...volumes, [userId]: value };
    void setStreamVolume(userId, value / 100);
  }

  function watchStatsFor(userId: string) {
    return stats?.watching.find((w) => w.user_id === userId) ?? null;
  }

  async function join() {
    if (elsewhere && !confirmSwitch) {
      confirmSwitch = true;
      return;
    }
    confirmSwitch = false;
    clearVoiceError();
    await joinVoice(channel.id);
  }
</script>

<div class="voice-view">
  {#if error && !here}
    <p class="error" role="alert">{translateBackendError(error, $t)}</p>
  {/if}

  {#if members.length === 0}
    <div class="empty">
      <h3>{$t("omnidisc.voice.nobody")}</h3>
      <p>{$t("omnidisc.voice.nobody_hint")}</p>
    </div>
  {:else}
    <p class="count">{$t("omnidisc.voice.participants", { count: members.length })}</p>
    <ul class="grid" aria-label={$t("omnidisc.voice.participants", { count: members.length })}>
      {#each members as m (m.userId)}
        {@const name = instance ? userName(instance.id, m.userId) : m.userId}
        {@const talking = isSpeaking(m.userId)}
        {@const live = isLive(m.userId)}
        {@const watching = isWatching(m.userId)}
        {@const ws = watchStatsFor(m.userId)}
        <li class="tile" class:speaking={talking} class:me={m.userId === me} class:live>
          {#if live}<span class="live-badge">{$t("omnidisc.stream.live")}</span>{/if}
          <span class="avatar" aria-hidden="true">{initial(name)}</span>
          <span class="name">{name}{#if m.userId === me} <span class="you">({$t("omnidisc.voice.you")})</span>{/if}</span>
          {#if live && m.userId !== me}
            <div class="watch">
              <button
                type="button"
                class="btn small"
                onclick={() => void toggleWatch(m.userId)}
                disabled={streamBusy || !canWatch}
                title={canWatch ? undefined : $t("omnidisc.stream.watch_unsupported")}
              >
                {watching ? $t("omnidisc.stream.stop_watching") : $t("omnidisc.stream.watch")}
              </button>
              {#if !canWatch}
                <span class="watch-note">{$t("omnidisc.stream.watch_unsupported")}</span>
              {/if}
              {#if watching}
                {#if ws && ws.width > 0}
                  <span class="res-badge">{ws.width}×{ws.height} · {Math.round(ws.fps_received)}fps{#if ws.codec} · {ws.codec.replace("video/", "").toUpperCase()}{/if}</span>
                {/if}
                <label class="vol">
                  <span class="sr-only">{$t("omnidisc.stream.volume")}</span>
                  <input type="range" min="0" max="200" step="5" value={volumeOf(m.userId)} oninput={(e) => onVolume(m.userId, Number(e.currentTarget.value))} />
                  <span class="vol-val">{volumeOf(m.userId)}%</span>
                </label>
              {/if}
            </div>
          {:else if live && m.userId === me}
            {@const preview = getStreamPreview()}
            {#if preview}
              <img class="self-preview" src={preview} alt={$t("omnidisc.stream.preview_alt")} />
            {/if}
            <span class="res-badge you-live">{$t("omnidisc.stream.you_live")}</span>
          {/if}
          <span class="badges">
            {#if talking}<span class="sr-only">{$t("omnidisc.voice.speaking")}</span>{/if}
            {#if m.selfDeaf || m.serverDeaf}
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label={$t("omnidisc.voice.user_deafened", { name })}><path d="M4 14v-2a8 8 0 0 1 16 0v2" /><rect x="3" y="14" width="4" height="6" rx="1" /><rect x="17" y="14" width="4" height="6" rx="1" /><path d="M4 4l16 16" /></svg>
            {:else if m.selfMute || m.serverMute}
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label={$t("omnidisc.voice.user_muted", { name })}><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5 11a7 7 0 0 0 14 0M12 18v3" /><path d="M4 4l16 16" /></svg>
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if here}
    <details class="inspector" bind:open={inspectorOpen}>
      <summary>{$t("omnidisc.stream.inspector")}</summary>
      <div class="inspector-body">
        {#if stats?.publishing}
          {@const p = stats.publishing}
          <p class="ins-line"><strong>{$t("omnidisc.stream.sending")}</strong> {p.width}×{p.height} · {Math.round(p.fps_sent || p.fps_encoded)}fps · {Math.round(p.bitrate_kbps)}/{p.configured_kbps} kbps · {(p.codec ?? "h264").toUpperCase()} · {p.encoder}{#if p.hardware === false} ({$t("omnidisc.stream.software")}){/if}</p>
          <p class="ins-line muted">{$t("omnidisc.stream.rtt")}: {p.rtt_ms != null ? Math.round(p.rtt_ms) + "ms" : "—"} · {$t("omnidisc.stream.loss")}: {p.packet_loss != null ? Math.round(p.packet_loss * 100) + "%" : "—"} · {$t("omnidisc.stream.encode")}: {p.encode_ms.toFixed(1)}ms</p>
          {#if p.hardware === false}<p class="ins-line warn">{$t("omnidisc.stream.software_warn")}</p>{/if}
        {/if}
        {#each stats?.watching ?? [] as w (w.user_id)}
          <p class="ins-line muted">{$t("omnidisc.stream.receiving")}: {w.width}×{w.height} · {Math.round(w.fps_received)}fps · {w.decoder || "—"} · {$t("omnidisc.stream.loss")} {w.packet_loss != null ? Math.round(w.packet_loss) : 0}</p>
        {/each}
        {#if !stats?.publishing && (stats?.watching?.length ?? 0) === 0}
          <p class="ins-line muted">{$t("omnidisc.stream.no_active")}</p>
        {/if}
      </div>
    </details>
  {/if}

  <div class="cta">
    {#if here}
      <button type="button" class="btn" onclick={() => void leaveVoice()} disabled={busy}>{$t("omnidisc.voice.leave")}</button>
    {:else if confirmSwitch}
      <p class="switch">
        <strong>{$t("omnidisc.voice.switch_title", { channel: channel.name })}</strong>
        {$t("omnidisc.voice.switch_body", { current: currentName })}
      </p>
      <div class="switch-actions">
        <button type="button" class="btn primary" onclick={join} disabled={busy}>{$t("omnidisc.voice.switch_confirm")}</button>
        <button type="button" class="btn" onclick={() => (confirmSwitch = false)}>{$t("common.cancel")}</button>
      </div>
    {:else}
      <button type="button" class="btn primary" onclick={join} disabled={busy || !canJoin} aria-busy={busy && connState === "connecting"}>
        {#if busy && connState === "connecting"}{$t("omnidisc.voice.connecting")}{:else}{$t("omnidisc.voice.join_channel", { channel: channel.name })}{/if}
      </button>
    {/if}
  </div>
</div>

<style>
  .voice-view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-5);
    overflow-y: auto;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    text-align: center;
    max-width: 360px;
  }

  .empty h3 {
    margin: 0;
    font-size: var(--text-lg);
    color: var(--text);
  }

  .empty p,
  .count {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-3);
    max-width: 640px;
  }

  .tile {
    position: relative;
    width: 140px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-2);
    border-radius: var(--radius-md);
    background: var(--surface);
    border: none;
  }

  .tile.live {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }

  .live-badge {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--on-accent);
    background: var(--danger);
    padding: 1px 6px;
    border-radius: var(--radius-sm);
  }

  .watch {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
  }

  .btn.small {
    padding: 4px var(--space-3);
    font-size: var(--text-xs);
    border-radius: var(--radius-sm);
  }

  .watch-note {
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-align: center;
    line-height: 1.4;
  }

  .res-badge {
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .you-live {
    color: var(--danger);
  }

  .self-preview {
    width: 100%;
    border-radius: var(--radius-sm);
    border: none;
    display: block;
  }

  .vol {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
  }

  .vol input {
    flex: 1;
    accent-color: var(--accent);
    min-width: 0;
  }

  .vol-val {
    font-size: var(--text-xs);
    color: var(--text-muted);
    min-width: 34px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .inspector {
    width: min(560px, 100%);
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .inspector summary {
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .inspector-body {
    padding: 0 var(--space-3) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .ins-line {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text);
    line-height: 1.5;
    font-variant-numeric: tabular-nums;
  }

  .ins-line.muted {
    color: var(--text-muted);
  }

  .ins-line.warn {
    color: var(--warning);
  }

  .avatar {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-full);
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-lg);
    font-weight: 600;
    box-shadow: 0 0 0 3px transparent;
    transition: box-shadow 120ms ease;
  }

  .tile.speaking .avatar {
    box-shadow: 0 0 0 3px var(--success);
  }

  .name {
    font-size: var(--text-sm);
    color: var(--text);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .you {
    color: var(--text-muted);
  }

  .badges {
    height: 14px;
    color: var(--text-muted);
    display: flex;
    gap: 4px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .cta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .switch {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    text-align: center;
    max-width: 320px;
  }

  .switch strong {
    display: block;
    color: var(--text);
  }

  .switch-actions {
    display: flex;
    gap: var(--space-2);
  }

  .btn {
    padding: 8px var(--space-4);
    border-radius: var(--radius-md);
    border: none;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .error {
    margin: 0;
    max-width: 420px;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: 1.5;
  }

  @media (prefers-reduced-motion: reduce) {
    .avatar {
      transition: none;
    }
  }
</style>
