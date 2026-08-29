<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    getSelectedInstance,
    getGuilds,
    getGuild,
    getDms,
    getSelectedGuildId,
    getSelectedChannelId,
    canManageGuild,
    canInGuild,
    createChannel,
    createInviteLink,
    isUnread,
    getMentionCount,
    dmTitle,
    getChannel,
    getVoiceMembers,
    getVoiceMemberCount,
    userName,
  } from "$lib/stores/omnidisc-store.svelte";
  import {
    getVoiceSession,
    getVoiceState,
    isVoiceBusy,
    isSpeaking,
    joinVoice,
  } from "$lib/stores/omnidisc-voice-store.svelte";
  import { isStreamer } from "$lib/stores/omnidisc-stream-store.svelte";
  import type { ChannelKind, OmnidiscChannel } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "./OmnidiscPrompt.svelte";
  import VoiceBar from "./VoiceBar.svelte";
  import IncomingCallBar from "./IncomingCallBar.svelte";

  let instance = $derived(getSelectedInstance());
  let guilds = $derived(getGuilds(instance?.id ?? null));
  let homeMode = $derived(!page.url.pathname.startsWith("/omnidisc/g/"));
  let guild = $derived(homeMode ? null : (getGuild(getSelectedGuildId()) ?? guilds[0] ?? null));
  let dms = $derived(getDms(instance?.id ?? null));
  let selectedChannelId = $derived(getSelectedChannelId());
  let isOwner = $derived(canManageGuild(guild?.id ?? null));
  let canOpenSettings = $derived(!!guild && (isOwner || canInGuild(guild.id, "MANAGE_GUILD") || canInGuild(guild.id, "MANAGE_ROLES") || canInGuild(guild.id, "MANAGE_CHANNELS") || canInGuild(guild.id, "VIEW_AUDIT_LOG")));
  let connected = $derived(instance?.status === "connected" || instance?.id === "demo");
  let voiceSession = $derived(getVoiceSession());
  let voiceState = $derived(getVoiceState());
  let voiceBusy = $derived(isVoiceBusy());
  let currentVoiceName = $derived(getChannel(voiceSession?.channelId ?? null)?.name ?? "");
  let switchTarget = $state<string | null>(null);

  let createOpen = $state(false);
  let createBusy = $state(false);
  let createError = $state<string | null>(null);
  let channelName = $state("");
  let channelKind = $state<ChannelKind>("text");

  let inviteState = $state<"idle" | "busy" | "copied" | "failed">("idle");
  let inviteError = $state<string | null>(null);
  let inviteTimer: ReturnType<typeof setTimeout> | null = null;

  interface ChannelGroup {
    id: string;
    category: string;
    channels: OmnidiscChannel[];
  }

  let groups = $derived.by(() => {
    if (!guild) return [] as ChannelGroup[];
    const map = new Map<string, ChannelGroup>();
    for (const channel of guild.channels) {
      if (channel.kind === "category") {
        if (!map.has(channel.id)) map.set(channel.id, { id: channel.id, category: channel.name, channels: [] });
        continue;
      }
      const key = channel.parentId ?? "";
      const group = map.get(key) ?? { id: key, category: channel.category ?? "", channels: [] };
      group.channels.push(channel);
      map.set(key, group);
    }
    return [...map.values()];
  });

  const COLLAPSED_KEY = "omnidisc.collapsed-categories";

  function loadCollapsed(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "{}") as Record<string, boolean>;
    } catch {
      return {};
    }
  }

  let collapsed = $state<Record<string, boolean>>(loadCollapsed());

  function toggleCategory(id: string) {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
    try {
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify(collapsed));
    } catch {
      /* per-viewer convenience only */
    }
  }

  function visibleChannels(group: ChannelGroup): OmnidiscChannel[] {
    if (!collapsed[group.id]) return group.channels;
    return group.channels.filter(
      (c) => c.id === selectedChannelId || isUnread(c.id) || getMentionCount(c.id) > 0,
    );
  }

  function openCreate() {
    createError = null;
    channelName = "";
    channelKind = "text";
    createOpen = true;
  }

  async function submitCreate() {
    if (!guild) return;
    createBusy = true;
    createError = null;
    try {
      const channel = await createChannel(guild.id, channelName.trim(), channelKind);
      createOpen = false;
      if (channel && channel.kind === "text") goto(`/omnidisc/g/${guild.id}/${channel.id}`);
    } catch (e) {
      createError = translateBackendError(typeof e === "string" ? e : String(e), $t);
    } finally {
      createBusy = false;
    }
  }

  function initial(name: string): string {
    return name.trim().charAt(0).toUpperCase() || "?";
  }

  async function clickVoice(channel: OmnidiscChannel) {
    if (!guild) return;
    const inVoice = !!voiceSession && voiceState !== "idle";
    goto(`/omnidisc/g/${guild.id}/${channel.id}`);
    if (inVoice && voiceSession?.channelId !== channel.id) {
      switchTarget = channel.id;
      return;
    }
    if (inVoice) return;
    switchTarget = null;
    await joinVoice(channel.id);
  }

  async function confirmSwitch() {
    const target = switchTarget;
    switchTarget = null;
    if (target) await joinVoice(target);
  }

  async function invite() {
    if (!guild || inviteState === "busy") return;
    inviteState = "busy";
    inviteError = null;
    try {
      const link = await createInviteLink(guild.id, selectedChannelId ?? undefined);
      if (!link) throw new Error("ERR_SERVER");
      await navigator.clipboard.writeText(link);
      inviteState = "copied";
    } catch (e) {
      inviteState = "failed";
      inviteError = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
    }
    if (inviteTimer) clearTimeout(inviteTimer);
    inviteTimer = setTimeout(() => {
      inviteState = "idle";
      inviteError = null;
    }, 4000);
  }
</script>

<aside class="channel-list" aria-label={$t("omnidisc.channels.title")}>
  <header class="head">
    <div class="head-text">
      <h2 class="title">{homeMode ? (instance?.name || $t("omnidisc.title")) : (guild?.name || instance?.name || $t("omnidisc.title"))}</h2>
      {#if instance?.name && guild}
        <span class="sub">{instance.name}</span>
      {/if}
    </div>
    {#if guild && canOpenSettings}
      <a class="head-action" href={`/omnidisc/g/${guild.id}/settings`} title={$t("omnidisc.guild_settings.title", { guild: guild.name })} aria-label={$t("omnidisc.guild_settings.title", { guild: guild.name })}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1A1.6 1.6 0 0 0 9 19.4a1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1A1.6 1.6 0 0 0 4.6 9a1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
      </a>
    {/if}
    {#if guild && connected}
      <button type="button" class="head-action" onclick={invite} disabled={inviteState === "busy"} title={$t("omnidisc.guild.invite")} aria-label={$t("omnidisc.guild.invite")}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M19 8v6M22 11h-6" />
        </svg>
      </button>
    {/if}
  </header>

  {#if inviteState === "copied"}
    <p class="toast ok" role="status">{$t("omnidisc.guild.invite_copied")}</p>
  {:else if inviteState === "failed"}
    <p class="toast bad" role="alert">{inviteError ?? $t("omnidisc.guild.invite_failed")}</p>
  {/if}

  <nav class="groups">
    {#if homeMode && instance}
      <h3 class="category">{$t("omnidisc.channels.dms")}</h3>
      {#if dms.length === 0}
        <p class="empty-body inset">{$t("omnidisc.channels.no_dms")}</p>
      {:else}
        <ul class="channels">
          {#each dms as dm (dm.id)}
            <li>
              <a
                class="channel"
                class:active={dm.id === selectedChannelId}
                class:unread={isUnread(dm.id)}
                href={`/omnidisc/dm/${dm.id}`}
                aria-current={dm.id === selectedChannelId ? "page" : undefined}
              >
                <span class="glyph at" aria-hidden="true">@</span>
                <span class="name">{dmTitle(dm)}</span>
                {#if getMentionCount(dm.id) > 0}
                  <span class="pill">{getMentionCount(dm.id)}</span>
                {:else if isUnread(dm.id)}
                  <span class="dot" aria-hidden="true"></span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    {#if !guild}
      {#if !homeMode && instance && guilds.length === 0}
        <div class="empty">
          <p class="empty-title">{$t("omnidisc.channels.no_guilds")}</p>
          <p class="empty-body">{$t("omnidisc.channels.no_guilds_hint")}</p>
        </div>
      {/if}
    {:else if guild.channels.length === 0}
      <div class="empty">
        <p class="empty-title">{$t("omnidisc.channels.empty")}</p>
        <p class="empty-body">{$t("omnidisc.channels.empty_hint")}</p>
      </div>
    {:else}
      {#each groups as group (group.id)}
        {#if group.category}
          <button
            type="button"
            class="category cat-toggle"
            aria-expanded={!collapsed[group.id]}
            onclick={() => toggleCategory(group.id)}
          >
            <svg class="chev" class:closed={collapsed[group.id]} viewBox="0 0 24 24" width="10" height="10" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
            {group.category}
          </button>
        {/if}
        <ul class="channels">
          {#each visibleChannels(group) as channel (channel.id)}
            <li>
              {#if channel.kind === "voice"}
                {@const count = getVoiceMemberCount(channel.id)}
                {@const inHere = voiceSession?.channelId === channel.id && voiceState !== "idle"}
                <button
                  type="button"
                  class="channel voice"
                  class:active={channel.id === selectedChannelId}
                  class:joined={inHere}
                  aria-current={channel.id === selectedChannelId ? "page" : undefined}
                  disabled={!connected || voiceBusy}
                  title={inHere ? channel.name : $t("omnidisc.voice.join_channel", { channel: channel.name })}
                  onclick={() => void clickVoice(channel)}
                >
                  <svg class="glyph" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M11 5L6 9H2v6h4l5 4V5z" />
                    <path d="M15.5 8.5a5 5 0 0 1 0 7" />
                  </svg>
                  <span class="name">{channel.name}</span>
                  {#if count > 0}
                    <span class="count" aria-label={$t("omnidisc.voice.participants", { count })}>{count}</span>
                  {/if}
                </button>
                {#if switchTarget === channel.id}
                  <div class="switch" role="group" aria-label={$t("omnidisc.voice.switch_title", { channel: channel.name })}>
                    <p class="switch-text">
                      <strong>{$t("omnidisc.voice.switch_title", { channel: channel.name })}</strong>
                      {$t("omnidisc.voice.switch_body", { current: currentVoiceName })}
                    </p>
                    <div class="switch-actions">
                      <button type="button" class="mini primary" onclick={() => void confirmSwitch()} disabled={voiceBusy}>{$t("omnidisc.voice.switch_confirm")}</button>
                      <button type="button" class="mini" onclick={() => (switchTarget = null)}>{$t("common.cancel")}</button>
                    </div>
                  </div>
                {/if}
                {#if count > 0}
                  <ul class="voice-members">
                    {#each getVoiceMembers(channel.id) as m (m.userId)}
                      {@const name = instance ? userName(instance.id, m.userId) : m.userId}
                      {@const talking = isSpeaking(m.userId)}
                      <li class="voice-member" class:speaking={talking}>
                        <span class="mini-avatar" aria-hidden="true">{initial(name)}</span>
                        <span class="member-name">{name}</span>
                        {#if m.streaming || isStreamer(m.userId)}<span class="live-mini">{$t("omnidisc.stream.live")}</span>{/if}
                        {#if talking}<span class="sr-only">{$t("omnidisc.voice.speaking")}</span>{/if}
                        {#if m.selfDeaf || m.serverDeaf}
                          <svg class="flag" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label={$t("omnidisc.voice.user_deafened", { name })}><path d="M4 14v-2a8 8 0 0 1 16 0v2" /><rect x="3" y="14" width="4" height="6" rx="1" /><rect x="17" y="14" width="4" height="6" rx="1" /><path d="M4 4l16 16" /></svg>
                        {:else if m.selfMute || m.serverMute}
                          <svg class="flag" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label={$t("omnidisc.voice.user_muted", { name })}><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5 11a7 7 0 0 0 14 0M12 18v3" /><path d="M4 4l16 16" /></svg>
                        {/if}
                      </li>
                    {/each}
                  </ul>
                {/if}
              {:else}
                <a
                  class="channel"
                  class:active={channel.id === selectedChannelId}
                  class:unread={isUnread(channel.id)}
                  href={`/omnidisc/g/${guild.id}/${channel.id}`}
                  aria-current={channel.id === selectedChannelId ? "page" : undefined}
                >
                  <span class="glyph hash" aria-hidden="true">#</span>
                  <span class="name">{channel.name}</span>
                  {#if getMentionCount(channel.id) > 0}
                    <span class="pill">{getMentionCount(channel.id)}</span>
                  {:else if isUnread(channel.id)}
                    <span class="dot" aria-hidden="true"></span>
                  {/if}
                </a>
              {/if}
            </li>
          {/each}
        </ul>
      {/each}
    {/if}

    {#if guild && isOwner && connected}
      <button type="button" class="create" onclick={openCreate}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        {$t("omnidisc.guild.create_channel")}
      </button>
    {/if}
  </nav>
  <IncomingCallBar />
  <VoiceBar />
</aside>

<OmnidiscPrompt
  open={createOpen}
  title={$t("omnidisc.guild.create_channel_title", { guild: guild?.name ?? "" })}
  submitLabel={$t("omnidisc.guild.create_channel_submit")}
  busy={createBusy}
  error={createError}
  canSubmit={channelName.trim().length > 0}
  onSubmit={submitCreate}
  onClose={() => (createOpen = false)}
>
  <label class="field">
    <span class="label">{$t("omnidisc.guild.channel_name_label")}</span>
    <input class="input" type="text" bind:value={channelName} placeholder={$t("omnidisc.guild.channel_name_placeholder")} maxlength="100" spellcheck="false" />
  </label>
  <fieldset class="field kind">
    <legend class="label">{$t("omnidisc.guild.channel_kind_label")}</legend>
    <div class="kind-row">
      <label class="kind-opt" class:on={channelKind === "text"}>
        <input type="radio" name="od-channel-kind" value="text" bind:group={channelKind} />
        <span class="hash">#</span>
        {$t("omnidisc.guild.channel_kind_text")}
      </label>
      <label class="kind-opt" class:on={channelKind === "voice"}>
        <input type="radio" name="od-channel-kind" value="voice" bind:group={channelKind} />
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M11 5L6 9H2v6h4l5 4V5z" /></svg>
        {$t("omnidisc.guild.channel_kind_voice")}
      </label>
    </div>
  </fieldset>
</OmnidiscPrompt>

<style>
  .channel-list {
    width: 232px;
    flex: 0 0 232px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--surface-mut);
    border-right: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-4);
    border-bottom: none;
    min-height: 48px;
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .head-action {
    width: 30px;
    height: 30px;
    flex: 0 0 30px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .head-action:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .head-action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .head-action:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .toast {
    margin: var(--space-2) var(--space-2) 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    line-height: 1.4;
  }

  .toast.ok {
    background: color-mix(in srgb, var(--success) 15%, transparent);
    color: var(--text);
  }

  .toast.bad {
    background: color-mix(in srgb, var(--danger) 15%, transparent);
    color: var(--text);
  }

  .groups {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
  }

  .category {
    margin: var(--space-3) var(--space-2) var(--space-1);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .cat-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    width: calc(100% - var(--space-2) * 2);
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-size: var(--text-xs);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .cat-toggle:hover {
    color: var(--text);
  }

  .cat-toggle:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .chev {
    transition: transform 120ms ease;
    flex-shrink: 0;
  }

  .chev.closed {
    transform: rotate(-90deg);
  }

  @media (prefers-reduced-motion: reduce) {
    .chev {
      transition: none;
    }
  }

  .channels {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .channel {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }

  .channel:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .channel.unread {
    color: var(--text);
    font-weight: 600;
  }

  .channel.active {
    background: var(--fill-2);
    color: var(--text);
    font-weight: 600;
  }

  .channel:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .channel.voice {
    width: 100%;
    border: none;
    background: transparent;
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .channel.voice:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .channel.voice:hover:not(:disabled) {
    background: var(--fill-1);
    color: var(--text);
  }

  .channel.joined {
    color: var(--text);
  }

  .channel.joined .glyph {
    color: var(--success);
    opacity: 1;
  }

  .count {
    min-width: 18px;
    padding: 0 6px;
    border-radius: var(--radius-full);
    background: var(--fill-2);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    text-align: center;
    line-height: 18px;
    font-variant-numeric: tabular-nums;
  }

  .voice-members {
    list-style: none;
    margin: 0 0 var(--space-1);
    padding: 0 0 0 28px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .voice-member {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .voice-member.speaking {
    color: var(--text);
  }

  .mini-avatar {
    width: 18px;
    height: 18px;
    flex: 0 0 18px;
    border-radius: var(--radius-full);
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 10px;
    font-weight: 600;
    box-shadow: 0 0 0 2px transparent;
    transition: box-shadow 120ms ease;
  }

  .voice-member.speaking .mini-avatar {
    box-shadow: 0 0 0 2px var(--success);
  }

  .member-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .live-mini {
    flex: 0 0 auto;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--on-accent);
    background: var(--danger);
    padding: 0 4px;
    border-radius: var(--radius-sm);
  }

  .flag {
    flex: 0 0 12px;
    color: var(--text-muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .switch {
    margin: var(--space-1) 0 var(--space-2) 28px;
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--surface);
    border: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .switch-text {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.4;
  }

  .switch-text strong {
    display: block;
    color: var(--text);
  }

  .switch-actions {
    display: flex;
    gap: var(--space-1);
  }

  .mini {
    flex: 1;
    padding: 4px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: var(--surface-mut);
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

  .mini:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .mini:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @media (prefers-reduced-motion: reduce) {
    .mini-avatar {
      transition: none;
    }
  }

  .glyph {
    flex: 0 0 16px;
    width: 16px;
    text-align: center;
    opacity: 0.8;
  }

  .hash,
  .at {
    font-weight: 600;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: 0 0 8px;
    border-radius: var(--radius-full);
    background: var(--text);
  }

  .pill {
    min-width: 18px;
    padding: 0 6px;
    border-radius: var(--radius-full);
    background: var(--danger);
    color: var(--on-accent);
    font-size: var(--text-xs);
    font-weight: 600;
    text-align: center;
    line-height: 18px;
  }

  .create {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    margin-top: var(--space-3);
    padding: 6px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .create:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .create:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .empty {
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .empty-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .empty-body {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .empty-body.inset {
    padding: 0 var(--space-2) var(--space-2);
  }

  .kind {
    margin: 0;
    padding: 0;
    border: none;
  }

  .kind-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  .kind-opt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 8px var(--space-3);
    border-radius: var(--radius-md);
    border: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .kind-opt.on {
    border-color: var(--accent);
    color: var(--text);
    background: var(--accent-soft);
  }

  .kind-opt input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .kind-opt:focus-within {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
