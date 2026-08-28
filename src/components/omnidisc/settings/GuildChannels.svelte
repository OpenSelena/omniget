<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    deleteChannel,
    deleteOverwrite,
    getGuild,
    getRoles,
    putOverwrite,
    updateChannelSettings,
    userName,
  } from "$lib/stores/omnidisc-store.svelte";
  import { PERMISSION_GROUPS, perm, toBits, type PermissionGroup, type PermissionName } from "$lib/omnidisc/permissions";
  import type { OmnidiscChannel } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "../OmnidiscPrompt.svelte";

  let {
    guildId,
    instanceId,
    canManage,
    initialChannelId = null,
  }: { guildId: string; instanceId: string; canManage: boolean; initialChannelId?: string | null } = $props();

  const GROUPS: PermissionGroup[] = ["general", "text", "voice", "moderation"];
  type Tri = "allow" | "inherit" | "deny";

  let guild = $derived(getGuild(guildId));
  let channels = $derived((guild?.channels ?? []).filter((c) => c.kind !== "category"));
  let selectedId = $state<string | null>(null);
  let seeded = false;

  $effect(() => {
    if (seeded) return;
    seeded = true;
    selectedId = initialChannelId;
  });
  let selected = $derived(channels.find((c) => c.id === selectedId) ?? channels[0] ?? null);
  let targetId = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let pendingDelete = $state<OmnidiscChannel | null>(null);

  let roles = $derived(getRoles(guildId));
  let memberTargets = $derived(
    (selected?.overwrites ?? []).filter((o) => o.targetKind === "member").map((o) => o.targetId),
  );
  let target = $derived(
    targetId ?? roles.find((r) => r.isEveryone)?.id ?? roles[0]?.id ?? null,
  );
  let targetKind = $derived<"role" | "member">(roles.some((r) => r.id === target) ? "role" : "member");
  let overwrite = $derived(selected?.overwrites?.find((o) => o.targetId === target) ?? null);

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function run(fn: () => Promise<void>) {
    busy = true;
    error = null;
    try {
      await fn();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  function stateOf(name: PermissionName): Tri {
    if (!overwrite) return "inherit";
    const bit = perm(name);
    if ((toBits(overwrite.allow) & bit) === bit) return "allow";
    if ((toBits(overwrite.deny) & bit) === bit) return "deny";
    return "inherit";
  }

  async function setState(name: PermissionName, next: Tri) {
    if (!selected || !target) return;
    const bit = perm(name);
    let allow = toBits(overwrite?.allow);
    let deny = toBits(overwrite?.deny);
    allow &= ~bit;
    deny &= ~bit;
    if (next === "allow") allow |= bit;
    if (next === "deny") deny |= bit;
    const channelId = selected.id;
    const id = target;
    const kind = targetKind;
    if (allow === 0n && deny === 0n) {
      await run(() => deleteOverwrite(channelId, id));
      return;
    }
    await run(() => putOverwrite(channelId, id, kind, allow.toString(), deny.toString()));
  }

  async function save(patch: Record<string, unknown>) {
    if (!selected) return;
    const id = selected.id;
    await run(() => updateChannelSettings(id, patch));
  }

  async function confirmDelete() {
    const channel = pendingDelete;
    if (!channel) return;
    await run(async () => {
      await deleteChannel(channel.id);
      pendingDelete = null;
      if (selectedId === channel.id) selectedId = null;
    });
  }
</script>

<section class="channels">
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if channels.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.channels.empty")}</p>
      <p class="state-body">{$t("omnidisc.channels.empty_hint")}</p>
    </div>
  {:else}
    <div class="grid">
      <ul class="list">
        {#each channels as channel (channel.id)}
          <li>
            <button type="button" class="row" class:on={selected?.id === channel.id} onclick={() => (selectedId = channel.id)}>
              <span class="glyph" aria-hidden="true">{channel.kind === "voice" ? "🔊" : "#"}</span>
              <span class="row-name">{channel.name}</span>
            </button>
          </li>
        {/each}
      </ul>

      {#if selected}
        <div class="detail">
          <label class="field">
            <span class="field-label">{$t("omnidisc.channel_settings.name")}</span>
            <input
              class="field-input"
              type="text"
              value={selected.name}
              maxlength="100"
              disabled={!canManage || busy}
              onchange={(e) => void save({ name: e.currentTarget.value.trim() })}
            />
          </label>

          {#if selected.kind !== "voice"}
            <label class="field">
              <span class="field-label">{$t("omnidisc.channel_settings.topic")}</span>
              <textarea
                class="field-input"
                rows="2"
                maxlength="1024"
                disabled={!canManage || busy}
                value={selected.topic ?? ""}
                onchange={(e) => void save({ topic: e.currentTarget.value })}
              ></textarea>
            </label>

            <label class="field short">
              <span class="field-label">{$t("omnidisc.channel_settings.slowmode")}</span>
              <select
                class="field-input"
                disabled={!canManage || busy}
                value={String(selected.slowmodeSeconds ?? 0)}
                onchange={(e) => void save({ slowmode_seconds: Number(e.currentTarget.value) })}
              >
                <option value="0">{$t("omnidisc.channel_settings.slowmode_off")}</option>
                <option value="5">5s</option>
                <option value="30">30s</option>
                <option value="60">1min</option>
                <option value="300">5min</option>
              </select>
            </label>

            <label class="toggle">
              <input
                type="checkbox"
                checked={selected.nsfw === true}
                disabled={!canManage || busy}
                onchange={(e) => void save({ nsfw: e.currentTarget.checked })}
              />
              <span>{$t("omnidisc.channel_settings.nsfw")}</span>
            </label>
          {:else}
            <p class="voice-note">{$t("omnidisc.channel_settings.voice_note")}</p>
          {/if}

          <h4 class="group-title">{$t("omnidisc.channel_settings.overwrites")}</h4>
          <p class="hint">{$t("omnidisc.channel_settings.overwrites_hint")}</p>
          <label class="field short">
            <span class="field-label">{$t("omnidisc.channel_settings.overwrite_target")}</span>
            <select class="field-input" value={target ?? ""} onchange={(e) => (targetId = e.currentTarget.value)}>
              {#each roles as role (role.id)}
                <option value={role.id}>{role.isEveryone ? $t("omnidisc.guild_settings.everyone_role") : role.name}</option>
              {/each}
              {#each memberTargets as memberId (memberId)}
                <option value={memberId}>{userName(instanceId, memberId)}</option>
              {/each}
            </select>
          </label>

          {#each GROUPS as group (group)}
            <h5 class="sub-title">{$t(`omnidisc.guild_settings.perm_group_${group}`)}</h5>
            <ul class="perms">
              {#each PERMISSION_GROUPS[group] as name (name)}
                <li class="perm">
                  <span class="perm-name">{$t(`omnidisc.perm.${name}`)}</span>
                  <div class="tri" role="radiogroup" aria-label={$t(`omnidisc.perm.${name}`)}>
                    <button
                      type="button"
                      class="tri-btn deny"
                      class:on={stateOf(name) === "deny"}
                      disabled={!canManage || busy}
                      onclick={() => void setState(name, "deny")}
                      aria-pressed={stateOf(name) === "deny"}
                    >{$t("omnidisc.channel_settings.deny")}</button>
                    <button
                      type="button"
                      class="tri-btn"
                      class:on={stateOf(name) === "inherit"}
                      disabled={!canManage || busy}
                      onclick={() => void setState(name, "inherit")}
                      aria-pressed={stateOf(name) === "inherit"}
                    >{$t("omnidisc.channel_settings.inherit")}</button>
                    <button
                      type="button"
                      class="tri-btn allow"
                      class:on={stateOf(name) === "allow"}
                      disabled={!canManage || busy}
                      onclick={() => void setState(name, "allow")}
                      aria-pressed={stateOf(name) === "allow"}
                    >{$t("omnidisc.channel_settings.allow")}</button>
                  </div>
                </li>
              {/each}
            </ul>
          {/each}

          {#if canManage}
            <button type="button" class="danger" onclick={() => (pendingDelete = selected)}>
              {$t("omnidisc.channel_settings.delete", { name: selected.name })}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

<OmnidiscPrompt
  open={pendingDelete !== null}
  title={$t("omnidisc.channel_settings.delete_title", { name: pendingDelete?.name ?? "" })}
  body={$t("omnidisc.channel_settings.delete_body")}
  submitLabel={$t("omnidisc.channel_settings.delete_confirm", { name: pendingDelete?.name ?? "" })}
  {busy}
  {error}
  onSubmit={() => void confirmDelete()}
  onClose={() => (pendingDelete = null)}
>
  <p class="hint">{$t("omnidisc.channel_settings.delete_hint")}</p>
</OmnidiscPrompt>

<style>
  .channels {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .grid {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--space-4);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
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

  .row:hover {
    background: var(--fill-1);
  }

  .row.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .glyph {
    color: var(--text-muted);
  }

  .row-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field.short {
    max-width: 260px;
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
    resize: vertical;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text);
  }

  .group-title {
    margin: var(--space-3) 0 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .sub-title {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text-muted);
  }

  .hint,
  .voice-note {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .perms {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .perm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 3px 0;
  }

  .perm-name {
    font-size: var(--text-sm);
    color: var(--text);
  }

  .tri {
    display: flex;
    flex: 0 0 auto;
  }

  .tri-btn {
    padding: 3px var(--space-2);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .tri-btn:first-child {
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  }

  .tri-btn:last-child {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }

  .tri-btn.on {
    color: var(--text);
    background: var(--fill-1);
    font-weight: 600;
  }

  .tri-btn.allow.on {
    color: var(--success);
    border-color: var(--success);
  }

  .tri-btn.deny.on {
    color: var(--danger);
    border-color: var(--danger);
  }

  .tri-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .danger {
    align-self: flex-start;
    padding: 6px var(--space-3);
    border: 1px solid var(--danger);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--danger);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .state-title {
    margin: 0;
    font-size: var(--text-base);
    color: var(--text);
  }

  .state-body {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  input:focus-visible,
  select:focus-visible,
  textarea:focus-visible,
  button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @media (max-width: 760px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
