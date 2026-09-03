<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    canInGuild,
    deleteGuild,
    getGuild,
    getInstance,
    getMembers,
    getNotificationLevel,
    isGuildOwner,
    leaveGuild,
    setNotificationLevel,
    transferGuild,
    updateGuild,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { NotificationLevel } from "$lib/omnidisc/types";
  import GuildRoles from "$components/omnidisc/settings/GuildRoles.svelte";
  import GuildChannels from "$components/omnidisc/settings/GuildChannels.svelte";
  import GuildInvites from "$components/omnidisc/settings/GuildInvites.svelte";
  import GuildBans from "$components/omnidisc/settings/GuildBans.svelte";
  import GuildAuditLog from "$components/omnidisc/settings/GuildAuditLog.svelte";
  import OmnidiscPrompt from "$components/omnidisc/OmnidiscPrompt.svelte";

  type View = "overview" | "roles" | "channels" | "invites" | "bans" | "audit";
  type Danger = "transfer" | "leave" | "delete" | null;

  let guildId = $derived(page.params.guild ?? "");
  let guild = $derived(getGuild(guildId));
  let instance = $derived(getInstance(guild?.instanceId ?? null));
  let members = $derived(getMembers(guildId));
  let owner = $derived(isGuildOwner(guildId));
  let canManage = $derived(canInGuild(guildId, "MANAGE_GUILD"));
  let canRoles = $derived(canInGuild(guildId, "MANAGE_ROLES"));
  let canChannels = $derived(canInGuild(guildId, "MANAGE_CHANNELS"));
  let canInvite = $derived(canInGuild(guildId, "CREATE_INVITES"));
  let canBan = $derived(canInGuild(guildId, "BAN_MEMBERS"));
  let canAudit = $derived(canInGuild(guildId, "VIEW_AUDIT_LOG"));

  let view = $state<View>((page.url.searchParams.get("channel") ? "channels" : "overview") as View);
  let initialChannelId = page.url.searchParams.get("channel");

  let name = $state("");
  let description = $state("");
  let seeded = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let saved = $state(false);
  let danger = $state<Danger>(null);
  let typed = $state("");
  let transferTo = $state("");

  $effect(() => {
    if (!guild || seeded === guild.id) return;
    seeded = guild.id;
    name = guild.name;
    description = guild.description ?? "";
  });

  let guildLevel = $derived(getNotificationLevel(guildId) ?? "all");

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function saveOverview() {
    if (!guild) return;
    busy = true;
    error = null;
    saved = false;
    try {
      const patch: Record<string, unknown> = {};
      if (name.trim() && name.trim() !== guild.name) patch.name = name.trim();
      if ((description ?? "") !== (guild.description ?? "")) patch.description = description.trim() || null;
      if (Object.keys(patch).length > 0) await updateGuild(guild.id, patch);
      saved = true;
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function runDanger() {
    if (!guild) return;
    busy = true;
    error = null;
    try {
      if (danger === "delete") {
        await deleteGuild(guild.id);
        danger = null;
        await goto("/omnidisc");
      } else if (danger === "leave") {
        await leaveGuild(guild.id);
        danger = null;
        await goto("/omnidisc");
      } else if (danger === "transfer" && transferTo) {
        await transferGuild(guild.id, transferTo);
        danger = null;
      }
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  const VIEWS: { id: View; allowed: () => boolean }[] = [
    { id: "overview", allowed: () => true },
    { id: "roles", allowed: () => canRoles },
    { id: "channels", allowed: () => canChannels },
    { id: "invites", allowed: () => canInvite },
    { id: "bans", allowed: () => canBan },
    { id: "audit", allowed: () => canAudit },
  ];
</script>

{#if !guild || !instance}
  <div class="not-found">
    <h3>{$t("omnidisc.guild_settings.not_found_title")}</h3>
    <p>{$t("omnidisc.guild_settings.not_found_body")}</p>
  </div>
{:else}
  <div class="settings">
    <header class="bar">
      <a class="back" href={`/omnidisc/g/${guild.id}/${guild.channels.find((c) => c.kind === "text")?.id ?? ""}`}>
        <span aria-hidden="true">‹</span>
        {$t("omnidisc.guild_settings.back")}
      </a>
      <h2 class="title">{$t("omnidisc.guild_settings.title", { guild: guild.name })}</h2>
    </header>

    <div class="layout">
      <nav class="sub-nav" aria-label={$t("omnidisc.guild_settings.title", { guild: guild.name })}>
        {#each VIEWS as item (item.id)}
          {#if item.allowed()}
            <button type="button" class="sub-item" class:on={view === item.id} onclick={() => (view = item.id)}>
              {$t(`omnidisc.guild_settings.tab_${item.id}`)}
            </button>
          {/if}
        {/each}
      </nav>

      <div class="content">
        {#if view === "overview"}
          <section class="block">
            <h3 class="block-title">{$t("omnidisc.guild_settings.overview")}</h3>
            <label class="field">
              <span class="field-label">{$t("omnidisc.guild.name_label")}</span>
              <input class="field-input" type="text" bind:value={name} maxlength="100" disabled={!canManage || busy} />
            </label>
            <label class="field">
              <span class="field-label">{$t("omnidisc.guild_settings.description")}</span>
              <textarea class="field-input" rows="2" maxlength="300" bind:value={description} disabled={!canManage || busy}></textarea>
            </label>
            {#if canManage}
              <button type="button" class="primary" onclick={() => void saveOverview()} disabled={busy}>
                {busy ? $t("omnidisc.guild.working") : $t("omnidisc.guild_settings.save")}
              </button>
            {:else}
              <p class="hint">{$t("omnidisc.guild_settings.read_only")}</p>
            {/if}
            {#if error}
              <p class="error" role="alert">{error}</p>
            {:else if saved}
              <p class="ok" role="status">{$t("omnidisc.guild_settings.saved")}</p>
            {/if}
          </section>

          <section class="block">
            <h3 class="block-title">{$t("omnidisc.notify.guild_title")}</h3>
            <p class="hint">{$t("omnidisc.notify.guild_hint")}</p>
            <div class="segmented" role="radiogroup" aria-label={$t("omnidisc.notify.guild_title")}>
              {#each ["all", "mentions", "nothing"] as level (level)}
                <button
                  type="button"
                  class="seg"
                  class:on={guildLevel === level}
                  aria-pressed={guildLevel === level}
                  onclick={() => setNotificationLevel(guildId, level as NotificationLevel)}
                >
                  {$t(`omnidisc.notify.level_${level}`)}
                </button>
              {/each}
            </div>
          </section>

          <section class="block danger-zone">
            <h3 class="block-title">{$t("omnidisc.guild_settings.danger")}</h3>
            <div class="danger-row">
              <div class="danger-text">
                <strong>{$t("omnidisc.guild_settings.leave")}</strong>
                <span>{owner ? $t("omnidisc.guild_settings.leave_owner_hint") : $t("omnidisc.guild_settings.leave_hint")}</span>
              </div>
              <button type="button" class="ghost" onclick={() => (danger = "leave")} disabled={owner}>{$t("omnidisc.guild_settings.leave")}</button>
            </div>
            {#if owner}
              <div class="danger-row">
                <div class="danger-text">
                  <strong>{$t("omnidisc.guild_settings.transfer")}</strong>
                  <span>{$t("omnidisc.guild_settings.transfer_hint")}</span>
                </div>
                <button type="button" class="ghost" onclick={() => { transferTo = ""; danger = "transfer"; }}>{$t("omnidisc.guild_settings.transfer")}</button>
              </div>
              <div class="danger-row">
                <div class="danger-text">
                  <strong>{$t("omnidisc.guild_settings.delete")}</strong>
                  <span>{$t("omnidisc.guild_settings.delete_hint")}</span>
                </div>
                <button type="button" class="ghost danger" onclick={() => { typed = ""; danger = "delete"; }}>{$t("omnidisc.guild_settings.delete")}</button>
              </div>
            {/if}
          </section>
        {:else if view === "roles"}
          <GuildRoles {guildId} canManage={canRoles} />
        {:else if view === "channels"}
          <GuildChannels {guildId} instanceId={instance.id} canManage={canChannels} {initialChannelId} />
        {:else if view === "invites"}
          <GuildInvites {guildId} {canInvite} />
        {:else if view === "bans"}
          <GuildBans {guildId} instanceId={instance.id} {canBan} />
        {:else if view === "audit"}
          <GuildAuditLog {guildId} instanceId={instance.id} />
        {/if}
      </div>
    </div>
  </div>

  <OmnidiscPrompt
    open={danger !== null}
    title={danger === "delete"
      ? $t("omnidisc.guild_settings.delete_title", { guild: guild.name })
      : danger === "leave"
        ? $t("omnidisc.guild_settings.leave_title", { guild: guild.name })
        : $t("omnidisc.guild_settings.transfer_title", { guild: guild.name })}
    body={danger === "delete"
      ? $t("omnidisc.guild_settings.delete_body", { guild: guild.name })
      : danger === "leave"
        ? $t("omnidisc.guild_settings.leave_body", { guild: guild.name })
        : $t("omnidisc.guild_settings.transfer_body")}
    submitLabel={danger === "delete"
      ? $t("omnidisc.guild_settings.delete_confirm", { guild: guild.name })
      : danger === "leave"
        ? $t("omnidisc.guild_settings.leave_confirm", { guild: guild.name })
        : $t("omnidisc.guild_settings.transfer_confirm")}
    {busy}
    {error}
    canSubmit={danger === "delete" ? typed.trim() === guild.name : danger === "transfer" ? transferTo.length > 0 : true}
    onSubmit={() => void runDanger()}
    onClose={() => {
      danger = null;
      error = null;
    }}
  >
    {#if danger === "delete"}
      <label class="field">
        <span class="field-label">{$t("omnidisc.guild_settings.delete_type", { guild: guild.name })}</span>
        <input class="field-input" type="text" bind:value={typed} placeholder={guild.name} spellcheck="false" />
      </label>
    {:else if danger === "transfer"}
      <label class="field">
        <span class="field-label">{$t("omnidisc.guild_settings.transfer_to")}</span>
        <select class="field-input" bind:value={transferTo}>
          <option value="">{$t("omnidisc.guild_settings.transfer_pick")}</option>
          {#each members.filter((m) => m.id !== instance.userId) as member (member.id)}
            <option value={member.id}>{member.name}</option>
          {/each}
        </select>
      </label>
    {:else}
      <p class="hint">{$t("omnidisc.guild_settings.leave_hint")}</p>
    {/if}
  </OmnidiscPrompt>
{/if}

<style>
  .settings {
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

  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .back:hover {
    color: var(--text);
  }

  .title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
  }

  .layout {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 180px 1fr;
  }

  .sub-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3) var(--space-2);
    border-right: none;
    background: var(--surface-mut);
  }

  .sub-item {
    padding: 6px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .sub-item:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .sub-item.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .content {
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-width: 560px;
  }

  .block-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
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
    border: none;
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    resize: vertical;
  }

  .primary {
    align-self: flex-start;
    padding: 8px var(--space-4);
    border: none;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .segmented {
    display: flex;
  }

  .seg {
    padding: 6px var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .seg:first-child {
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  }

  .seg:last-child {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }

  .seg.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .danger-zone {
    border-top: none;
    padding-top: var(--space-3);
  }

  .danger-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
  }

  .danger-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .danger-text strong {
    font-size: var(--text-sm);
    color: var(--text);
  }

  .danger-text span {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.4;
  }

  .ghost {
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .ghost.danger {
    color: var(--danger);
    border-color: var(--danger);
  }

  .hint {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .ok {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--success);
  }

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
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

  input:focus-visible,
  select:focus-visible,
  textarea:focus-visible,
  button:focus-visible,
  a:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
    }

    .sub-nav {
      flex-direction: row;
      overflow-x: auto;
      border-right: none;
      border-bottom: none;
    }
  }
</style>
