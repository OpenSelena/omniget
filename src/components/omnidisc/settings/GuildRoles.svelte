<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { createRole, deleteRole, getRoles, updateRole } from "$lib/stores/omnidisc-store.svelte";
  import { PERMISSION_GROUPS, has, perm, toBits, type PermissionGroup, type PermissionName } from "$lib/omnidisc/permissions";
  import type { OmnidiscRole } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "../OmnidiscPrompt.svelte";

  let { guildId, canManage }: { guildId: string; canManage: boolean } = $props();

  const GROUPS: PermissionGroup[] = ["general", "text", "voice", "moderation"];

  let roles = $derived(getRoles(guildId));
  let selectedId = $state<string | null>(null);
  let selected = $derived(roles.find((r) => r.id === selectedId) ?? roles[0] ?? null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let createOpen = $state(false);
  let newName = $state("");
  let pendingDelete = $state<OmnidiscRole | null>(null);

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

  async function togglePermission(role: OmnidiscRole, name: PermissionName) {
    const bits = toBits(role.permissions);
    const next = has(bits, name) ? bits & ~perm(name) : bits | perm(name);
    await run(() => updateRole(guildId, role.id, { permissions: next.toString() }));
  }

  async function rename(role: OmnidiscRole, name: string) {
    const trimmed = name.trim();
    if (!trimmed || trimmed === role.name) return;
    await run(() => updateRole(guildId, role.id, { name: trimmed }));
  }

  async function setColor(role: OmnidiscRole, hex: string) {
    const value = Number.parseInt(hex.replace("#", ""), 16);
    if (!Number.isFinite(value)) return;
    await run(() => updateRole(guildId, role.id, { color: value }));
  }

  async function submitCreate() {
    const name = newName.trim();
    if (!name) return;
    await run(async () => {
      const role = await createRole(guildId, name, "0");
      if (role) selectedId = role.id;
      createOpen = false;
      newName = "";
    });
  }

  async function confirmDelete() {
    const role = pendingDelete;
    if (!role) return;
    await run(async () => {
      await deleteRole(guildId, role.id);
      pendingDelete = null;
      if (selectedId === role.id) selectedId = null;
    });
  }

  function colorHex(role: OmnidiscRole): string {
    return `#${(role.color ?? 0).toString(16).padStart(6, "0")}`;
  }
</script>

<section class="roles">
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if roles.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.guild_settings.roles_empty_title")}</p>
      <p class="state-body">{$t("omnidisc.guild_settings.roles_empty_body")}</p>
    </div>
  {:else}
    <div class="grid">
      <div class="list-col">
        <ul class="list">
          {#each roles as role (role.id)}
            <li>
              <button type="button" class="role" class:on={selected?.id === role.id} onclick={() => (selectedId = role.id)}>
                <span class="swatch" style:background={role.color ? colorHex(role) : "var(--text-dim)"}></span>
                <span class="role-name">{role.isEveryone ? $t("omnidisc.guild_settings.everyone_role") : role.name}</span>
              </button>
            </li>
          {/each}
        </ul>
        {#if canManage}
          <button type="button" class="add" onclick={() => (createOpen = true)}>{$t("omnidisc.guild_settings.role_create")}</button>
        {/if}
      </div>

      {#if selected}
        <div class="detail">
          <div class="fields">
            <label class="field">
              <span class="field-label">{$t("omnidisc.guild_settings.role_name")}</span>
              <input
                class="field-input"
                type="text"
                value={selected.isEveryone ? $t("omnidisc.guild_settings.everyone_role") : selected.name}
                maxlength="60"
                disabled={!canManage || selected.isEveryone || busy}
                onchange={(e) => selected && void rename(selected, e.currentTarget.value)}
              />
            </label>
            {#if !selected.isEveryone}
              <label class="field short">
                <span class="field-label">{$t("omnidisc.guild_settings.role_color")}</span>
                <input
                  class="field-color"
                  type="color"
                  value={colorHex(selected)}
                  disabled={!canManage || busy}
                  onchange={(e) => selected && void setColor(selected, e.currentTarget.value)}
                />
              </label>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={selected.hoist}
                  disabled={!canManage || busy}
                  onchange={(e) => selected && void run(() => updateRole(guildId, selected.id, { hoist: e.currentTarget.checked }))}
                />
                <span>{$t("omnidisc.guild_settings.role_hoist")}</span>
              </label>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={selected.mentionable}
                  disabled={!canManage || busy}
                  onchange={(e) => selected && void run(() => updateRole(guildId, selected.id, { mentionable: e.currentTarget.checked }))}
                />
                <span>{$t("omnidisc.guild_settings.role_mentionable")}</span>
              </label>
            {/if}
          </div>

          {#each GROUPS as group (group)}
            <h4 class="group-title">{$t(`omnidisc.guild_settings.perm_group_${group}`)}</h4>
            <ul class="perms">
              {#each PERMISSION_GROUPS[group] as name (name)}
                <li class="perm">
                  <label class="perm-row">
                    <input
                      type="checkbox"
                      checked={has(toBits(selected.permissions), name)}
                      disabled={!canManage || busy}
                      onchange={() => selected && void togglePermission(selected, name)}
                    />
                    <span class="perm-text">
                      <span class="perm-name">{$t(`omnidisc.perm.${name}`)}</span>
                      <span class="perm-desc">{$t(`omnidisc.perm_desc.${name}`)}</span>
                    </span>
                  </label>
                </li>
              {/each}
            </ul>
          {/each}

          {#if canManage && !selected.isEveryone}
            <button type="button" class="danger" onclick={() => (pendingDelete = selected)}>
              {$t("omnidisc.guild_settings.role_delete", { name: selected.name })}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

<OmnidiscPrompt
  open={createOpen}
  title={$t("omnidisc.guild_settings.role_create_title")}
  submitLabel={$t("omnidisc.guild_settings.role_create_submit")}
  {busy}
  {error}
  canSubmit={newName.trim().length > 0}
  onSubmit={() => void submitCreate()}
  onClose={() => (createOpen = false)}
>
  <label class="field">
    <span class="field-label">{$t("omnidisc.guild_settings.role_name")}</span>
    <input class="field-input" type="text" bind:value={newName} maxlength="60" placeholder={$t("omnidisc.guild_settings.role_name_placeholder")} />
  </label>
</OmnidiscPrompt>

<OmnidiscPrompt
  open={pendingDelete !== null}
  title={$t("omnidisc.guild_settings.role_delete_title", { name: pendingDelete?.name ?? "" })}
  body={$t("omnidisc.guild_settings.role_delete_body")}
  submitLabel={$t("omnidisc.guild_settings.role_delete_confirm", { name: pendingDelete?.name ?? "" })}
  {busy}
  {error}
  onSubmit={() => void confirmDelete()}
  onClose={() => (pendingDelete = null)}
>
  <p class="confirm-text">{$t("omnidisc.guild_settings.role_delete_hint")}</p>
</OmnidiscPrompt>

<style>
  .roles {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .grid {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--space-4);
  }

  .list-col {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .role {
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

  .role:hover {
    background: var(--fill-1);
  }

  .role.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .swatch {
    width: 10px;
    height: 10px;
    flex: 0 0 10px;
    border-radius: var(--radius-full);
  }

  .role-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .add,
  .danger {
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .add:hover {
    background: var(--fill-1);
  }

  .danger {
    align-self: flex-start;
    color: var(--danger);
    border-color: var(--danger);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1 1 200px;
  }

  .field.short {
    flex: 0 0 auto;
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
  }

  .field-color {
    width: 48px;
    height: 34px;
    padding: 2px;
    border-radius: var(--radius-sm);
    border: none;
    background: var(--input-bg);
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

  .perms {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .perm-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: 4px 0;
    cursor: pointer;
  }

  .perm-text {
    display: flex;
    flex-direction: column;
  }

  .perm-name {
    font-size: var(--text-sm);
    color: var(--text);
  }

  .perm-desc {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.4;
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

  .state-body,
  .confirm-text {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  input:focus-visible,
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
