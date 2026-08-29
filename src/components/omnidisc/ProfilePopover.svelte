<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    acceptFriend,
    addFriend,
    banMember,
    blockUser,
    canInGuild,
    getGuild,
    getInstance,
    getMember,
    getNote,
    getRoles,
    getUser,
    isGuildOwner,
    kickMember,
    loadNotes,
    mutualGuilds,
    openDm,
    relationshipWith,
    removeFriend,
    saveNote,
    updateMember,
    userName,
  } from "$lib/stores/omnidisc-store.svelte";
  import OmnidiscPrompt from "./OmnidiscPrompt.svelte";

  let {
    instanceId,
    userId,
    guildId = null,
    x,
    y,
    onClose,
  }: {
    instanceId: string;
    userId: string;
    guildId?: string | null;
    x: number;
    y: number;
    onClose: () => void;
  } = $props();

  type Action = "kick" | "ban" | "timeout" | "nickname" | null;

  let panel = $state<HTMLElement | null>(null);
  let note = $state("");
  let noteLoaded = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let action = $state<Action>(null);
  let reason = $state("");
  let nickname = $state("");
  let timeoutMinutes = $state(60);

  let instance = $derived(getInstance(instanceId));
  let user = $derived(getUser(instanceId, userId));
  let member = $derived(getMember(guildId, userId));
  let guild = $derived(getGuild(guildId));
  let me = $derived(instance?.userId ?? null);
  let isMe = $derived(me === userId);
  let relationship = $derived(relationshipWith(instanceId, userId));
  let mutual = $derived(mutualGuilds(instanceId, userId));
  let roleNames = $derived(
    (member?.roleIds ?? [])
      .map((id) => getRoles(guildId).find((r) => r.id === id))
      .filter((r) => r && !r.isEveryone)
      .map((r) => r?.name ?? ""),
  );
  let displayName = $derived(user?.displayName ?? userName(instanceId, userId));
  let canKick = $derived(!isMe && !!guildId && canInGuild(guildId, "KICK_MEMBERS"));
  let canBan = $derived(!isMe && !!guildId && canInGuild(guildId, "BAN_MEMBERS"));
  let canTimeout = $derived(!isMe && !!guildId && canInGuild(guildId, "MODERATE_MEMBERS"));
  let canNickname = $derived(!isMe && !!guildId && canInGuild(guildId, "MANAGE_NICKNAMES"));
  let targetIsOwner = $derived(!!guild && guild.ownerId === userId);
  let moderationAllowed = $derived(!targetIsOwner || isGuildOwner(guildId));

  $effect(() => {
    if (noteLoaded || !instanceId) return;
    noteLoaded = true;
    loadNotes(instanceId)
      .then(() => {
        note = getNote(instanceId, userId);
      })
      .catch(() => {
        note = "";
      });
  });

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function run(fn: () => Promise<void>) {
    busy = true;
    error = null;
    try {
      await fn();
      action = null;
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function message() {
    await run(async () => {
      const channel = await openDm(instanceId, userId);
      if (channel) {
        onClose();
        await goto(`/omnidisc/dm/${channel.id}`);
      }
    });
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (action) return;
    if (panel && e.target instanceof Node && panel.contains(e.target)) return;
    onClose();
  }

  function confirmTitle(): string {
    switch (action) {
      case "kick":
        return $t("omnidisc.profile.kick_title", { name: displayName });
      case "ban":
        return $t("omnidisc.profile.ban_title", { name: displayName });
      case "timeout":
        return $t("omnidisc.profile.timeout_title", { name: displayName });
      case "nickname":
        return $t("omnidisc.profile.nickname_title", { name: displayName });
      default:
        return "";
    }
  }

  function confirmSubmitLabel(): string {
    switch (action) {
      case "kick":
        return $t("omnidisc.profile.kick_confirm", { name: displayName });
      case "ban":
        return $t("omnidisc.profile.ban_confirm", { name: displayName });
      case "timeout":
        return $t("omnidisc.profile.timeout_confirm", { name: displayName });
      case "nickname":
        return $t("omnidisc.profile.nickname_confirm");
      default:
        return "";
    }
  }

  async function submitAction() {
    if (!guildId) return;
    const current = action;
    await run(async () => {
      if (current === "kick") await kickMember(guildId, userId, reason.trim() || undefined);
      else if (current === "ban") await banMember(guildId, userId, reason.trim() || undefined);
      else if (current === "timeout") {
        const until = new Date(Date.now() + timeoutMinutes * 60_000).toISOString();
        await updateMember(guildId, userId, { muted_until: until, reason: reason.trim() || undefined });
      } else if (current === "nickname") {
        await updateMember(guildId, userId, { nick: nickname.trim() || null });
      }
      reason = "";
      onClose();
    });
  }

  async function copyId() {
    try {
      await navigator.clipboard.writeText(userId);
    } catch {
      error = $t("omnidisc.profile.copy_failed");
    }
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydown={(e) => { if (e.key === "Escape" && !action) onClose(); }} />

<div
  class="profile"
  role="dialog"
  aria-label={$t("omnidisc.profile.title", { name: displayName })}
  bind:this={panel}
  style:left={`${x}px`}
  style:top={`${y}px`}
>
  <div class="banner" style:background={user?.accentColor !== undefined ? `#${user.accentColor.toString(16).padStart(6, "0")}` : "var(--accent-soft)"}></div>
  <div class="head">
    <span class="avatar" aria-hidden="true">{displayName.trim().charAt(0).toUpperCase() || "?"}</span>
    <div class="names">
      <strong class="display">{member?.nick || displayName}</strong>
      <span class="handle">@{user?.username ?? "…"}</span>
      {#if user?.pronouns}
        <span class="pronouns">{user.pronouns}</span>
      {/if}
    </div>
  </div>

  {#if user?.bio}
    <p class="bio">{user.bio}</p>
  {/if}

  {#if roleNames.length > 0}
    <div class="section">
      <h4>{$t("omnidisc.profile.roles")}</h4>
      <ul class="chips">
        {#each roleNames as name (name)}
          <li class="chip">{name}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if mutual.length > 0}
    <div class="section">
      <h4>{$t("omnidisc.profile.mutual_servers", { count: mutual.length })}</h4>
      <ul class="chips">
        {#each mutual.slice(0, 6) as g (g.id)}
          <li class="chip">{g.name}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if !isMe}
    <div class="section">
      <label class="note-label" for="od-profile-note">{$t("omnidisc.profile.note")}</label>
      <textarea
        id="od-profile-note"
        class="note"
        rows="2"
        bind:value={note}
        placeholder={$t("omnidisc.profile.note_placeholder")}
        onblur={() => void run(() => saveNote(instanceId, userId, note))}
      ></textarea>
    </div>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if !isMe}
    <div class="actions">
      <button type="button" class="btn primary" onclick={() => void message()} disabled={busy}>
        {$t("omnidisc.profile.message")}
      </button>
      {#if relationship?.kind === "incoming_request"}
        <button type="button" class="btn" onclick={() => void run(() => acceptFriend(instanceId, userId))} disabled={busy}>
          {$t("omnidisc.friends.accept")}
        </button>
      {:else if relationship?.kind === "friend"}
        <button type="button" class="btn" onclick={() => void run(() => removeFriend(instanceId, userId))} disabled={busy}>
          {$t("omnidisc.friends.remove")}
        </button>
      {:else if relationship?.kind === "outgoing_request"}
        <span class="pending-note">{$t("omnidisc.friends.request_sent")}</span>
      {:else if relationship?.kind !== "blocked" && user}
        <button type="button" class="btn" onclick={() => void run(() => addFriend(instanceId, user.username))} disabled={busy}>
          {$t("omnidisc.friends.add")}
        </button>
      {/if}
      <button type="button" class="btn" onclick={() => void copyId()}>{$t("omnidisc.profile.copy_id")}</button>
      {#if relationship?.kind === "blocked"}
        <button type="button" class="btn" onclick={() => void run(() => removeFriend(instanceId, userId))} disabled={busy}>
          {$t("omnidisc.friends.unblock")}
        </button>
      {:else}
        <button type="button" class="btn" onclick={() => void run(() => blockUser(instanceId, userId))} disabled={busy}>
          {$t("omnidisc.friends.block")}
        </button>
      {/if}
    </div>
  {/if}

  {#if moderationAllowed && (canNickname || canTimeout || canKick || canBan)}
    <div class="actions moderation">
      <h4>{$t("omnidisc.profile.moderation")}</h4>
      {#if canNickname}
        <button type="button" class="btn" onclick={() => { nickname = member?.nick ?? ""; action = "nickname"; }}>{$t("omnidisc.profile.nickname")}</button>
      {/if}
      {#if canTimeout}
        <button type="button" class="btn" onclick={() => (action = "timeout")}>{$t("omnidisc.profile.timeout")}</button>
      {/if}
      {#if canKick}
        <button type="button" class="btn" onclick={() => (action = "kick")}>{$t("omnidisc.profile.kick")}</button>
      {/if}
      {#if canBan}
        <button type="button" class="btn danger" onclick={() => (action = "ban")}>{$t("omnidisc.profile.ban")}</button>
      {/if}
    </div>
  {/if}
</div>

<OmnidiscPrompt
  open={action !== null}
  title={confirmTitle()}
  body={action === "ban" ? $t("omnidisc.profile.ban_body", { name: displayName }) : action === "kick" ? $t("omnidisc.profile.kick_body", { name: displayName }) : undefined}
  submitLabel={confirmSubmitLabel()}
  busy={busy}
  error={error}
  onSubmit={() => void submitAction()}
  onClose={() => { action = null; error = null; }}
>
  {#if action === "nickname"}
    <label class="field">
      <span class="field-label">{$t("omnidisc.profile.nickname_label")}</span>
      <input class="field-input" type="text" bind:value={nickname} maxlength="32" placeholder={displayName} />
    </label>
  {:else if action === "timeout"}
    <label class="field">
      <span class="field-label">{$t("omnidisc.profile.timeout_label")}</span>
      <select class="field-input" bind:value={timeoutMinutes}>
        <option value={5}>{$t("omnidisc.profile.timeout_5m")}</option>
        <option value={60}>{$t("omnidisc.profile.timeout_1h")}</option>
        <option value={1440}>{$t("omnidisc.profile.timeout_1d")}</option>
        <option value={10080}>{$t("omnidisc.profile.timeout_1w")}</option>
      </select>
    </label>
    <label class="field">
      <span class="field-label">{$t("omnidisc.profile.reason_label")}</span>
      <input class="field-input" type="text" bind:value={reason} maxlength="200" />
    </label>
  {:else}
    <label class="field">
      <span class="field-label">{$t("omnidisc.profile.reason_label")}</span>
      <input class="field-input" type="text" bind:value={reason} maxlength="200" placeholder={$t("omnidisc.profile.reason_placeholder")} />
    </label>
  {/if}
</OmnidiscPrompt>

<style>
  .profile {
    position: fixed;
    z-index: 60;
    width: 300px;
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-bottom: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.25));
  }

  .banner {
    height: 56px;
    border-radius: var(--radius-md) var(--radius-md) 0 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-3);
    margin-top: -22px;
  }

  .avatar {
    width: 52px;
    height: 52px;
    flex: 0 0 52px;
    border-radius: var(--radius-full);
    border: 3px solid var(--surface);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .names {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding-top: 20px;
  }

  .display {
    font-size: var(--text-base);
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .handle,
  .pronouns {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .bio {
    margin: 0 var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-secondary, var(--text));
    line-height: 1.45;
    white-space: pre-wrap;
  }

  .section {
    padding: 0 var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .section h4,
  .moderation h4 {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chip {
    padding: 2px var(--space-2);
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    font-size: var(--text-xs);
    color: var(--text-secondary, var(--text));
  }

  .note-label {
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .note {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    padding: 6px var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .note:focus-visible,
  .field-input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    padding: 0 var(--space-3);
  }

  .moderation {
    flex-direction: column;
    align-items: stretch;
    border-top: 1px solid var(--border);
    padding-top: var(--space-2);
  }

  .btn {
    flex: 1 1 auto;
    padding: 6px var(--space-2);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: center;
  }

  .btn:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .btn.danger {
    color: var(--danger);
    border-color: var(--danger);
  }

  .btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .pending-note {
    flex: 1 1 auto;
    font-size: var(--text-xs);
    color: var(--text-muted);
    align-self: center;
  }

  .error {
    margin: 0 var(--space-3);
    font-size: var(--text-xs);
    color: var(--danger);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text);
  }

  .field-input {
    width: 100%;
    box-sizing: border-box;
    padding: 8px var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }
</style>
