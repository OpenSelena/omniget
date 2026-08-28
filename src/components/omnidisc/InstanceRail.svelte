<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    getInstances,
    getSelectedInstance,
    selectInstance,
    getGuilds,
    getSelectedGuildId,
    createGuild,
    joinInvite,
    reconnectInstance,
    resetConnect,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscGuild, OmnidiscInstance } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "./OmnidiscPrompt.svelte";

  let instances = $derived(getInstances());
  let selected = $derived(getSelectedInstance());
  let guilds = $derived(getGuilds(selected?.id ?? null));
  let selectedGuildId = $derived(getSelectedGuildId());
  let isHome = $derived(page.url.pathname === "/omnidisc");
  let isFriends = $derived(page.url.pathname.startsWith("/omnidisc/friends"));

  let menuOpen = $state(false);
  let dialog = $state<"create" | "join" | null>(null);
  let dialogBusy = $state(false);
  let dialogError = $state<string | null>(null);
  let guildName = $state("");
  let inviteCode = $state("");

  function initials(name: string): string {
    const parts = name.trim().split(/\s+/).filter(Boolean);
    if (parts.length === 0) return "?";
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }

  function openInstance(instance: OmnidiscInstance) {
    selectInstance(instance.id);
    goto("/omnidisc");
  }

  function openGuild(guild: OmnidiscGuild) {
    const first = guild.channels.find((c) => c.kind === "text") ?? guild.channels.find((c) => c.kind !== "category");
    if (!first) return;
    goto(`/omnidisc/g/${guild.id}/${first.id}`);
  }

  function statusKey(status: OmnidiscInstance["status"]): string {
    return `omnidisc.rail.status_${status}`;
  }

  function instanceTitle(instance: OmnidiscInstance): string {
    let base = `${instance.name} — ${$t(statusKey(instance.status))}`;
    if ((instance.status === "error" || instance.status === "signed_out") && instance.error) {
      base = `${base}. ${translateBackendError(instance.error, $t)}`;
    }
    if (instance.insecure) {
      base = `${base}. ${$t("omnidisc.connect.insecure_short")}`;
    }
    return base;
  }

  function openDialog(kind: "create" | "join") {
    menuOpen = false;
    dialogError = null;
    dialogBusy = false;
    dialog = kind;
  }

  function closeDialog() {
    dialog = null;
    guildName = "";
    inviteCode = "";
    dialogError = null;
  }

  async function submitCreate() {
    const instance = selected;
    if (!instance) return;
    dialogBusy = true;
    dialogError = null;
    try {
      const guild = await createGuild(instance.id, guildName.trim());
      closeDialog();
      if (guild) openGuild(guild);
    } catch (e) {
      dialogError = translateBackendError(typeof e === "string" ? e : String(e), $t);
    } finally {
      dialogBusy = false;
    }
  }

  async function submitJoin() {
    const instance = selected;
    if (!instance) return;
    dialogBusy = true;
    dialogError = null;
    try {
      const guild = await joinInvite(instance.id, inviteCode.trim());
      closeDialog();
      if (guild) openGuild(guild);
    } catch (e) {
      dialogError = translateBackendError(typeof e === "string" ? e : String(e), $t);
    } finally {
      dialogBusy = false;
    }
  }

  function addInstance() {
    menuOpen = false;
    resetConnect();
    goto("/omnidisc?add=1");
  }

  function onMenuKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") menuOpen = false;
  }

  let canCreate = $derived(selected?.status === "connected");
</script>

<nav class="rail" aria-label={$t("omnidisc.title")}>
  <a class="rail-item home" class:active={isHome} href="/omnidisc" aria-label={$t("omnidisc.rail.home")} title={$t("omnidisc.rail.home")}>
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M21 12a8 8 0 0 1-8 8H7l-4 3V12a8 8 0 0 1 8-8h2a8 8 0 0 1 8 8z" />
    </svg>
  </a>

  <a class="rail-item friends" class:active={isFriends} href="/omnidisc/friends" aria-label={$t("omnidisc.friends.title")} title={$t("omnidisc.friends.title")}>
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
      <path d="M16 3.13a4 4 0 0 1 0 7.75" />
    </svg>
  </a>

  <div class="rail-divider" role="presentation"></div>

  {#each instances as instance (instance.id)}
    <button
      type="button"
      class="rail-item instance"
      class:active={selected?.id === instance.id}
      class:error={instance.status === "error" || instance.status === "signed_out"}
      onclick={() => openInstance(instance)}
      title={instanceTitle(instance)}
      aria-label={instanceTitle(instance)}
      aria-pressed={selected?.id === instance.id}
    >
      <span class="initials">{initials(instance.name)}</span>
      <span class="status-dot {instance.status}" aria-hidden="true"></span>
      {#if instance.insecure}
        <span class="insecure-dot" title={$t("omnidisc.rail.insecure", { name: instance.name })} aria-hidden="true">
          <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 6v7M12 17h.01" />
          </svg>
        </span>
      {/if}
    </button>
  {/each}

  {#if selected && (selected.status === "error" || selected.status === "disconnected" || selected.status === "signed_out")}
    <button
      type="button"
      class="rail-action"
      onclick={() => (selected?.status === "signed_out" ? addInstance() : reconnectInstance(selected.id))}
      title={selected.status === "signed_out" ? $t("omnidisc.rail.sign_in") : $t("omnidisc.rail.reconnect")}
      aria-label={selected.status === "signed_out" ? $t("omnidisc.rail.sign_in") : $t("omnidisc.rail.reconnect")}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M21 12a9 9 0 1 1-2.6-6.4" />
        <path d="M21 3v6h-6" />
      </svg>
    </button>
  {/if}

  {#if guilds.length > 0}
    <div class="rail-divider" role="presentation"></div>
    {#each guilds as guild (guild.id)}
      <button
        type="button"
        class="rail-item guild"
        class:active={selectedGuildId === guild.id}
        onclick={() => openGuild(guild)}
        title={guild.name}
        aria-label={guild.name}
        aria-pressed={selectedGuildId === guild.id}
      >
        <span class="initials">{initials(guild.name)}</span>
      </button>
    {/each}
  {/if}

  <div class="rail-spacer"></div>

  <div class="add-wrap">
    <button
      type="button"
      class="rail-item add"
      aria-label={$t("omnidisc.rail.menu")}
      title={$t("omnidisc.rail.menu")}
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={() => (menuOpen = !menuOpen)}
    >
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <path d="M12 5v14M5 12h14" />
      </svg>
    </button>
    {#if menuOpen}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="menu" role="menu" tabindex="-1" onkeydown={onMenuKeydown}>
        <button type="button" role="menuitem" onclick={() => openDialog("create")} disabled={!canCreate}>{$t("omnidisc.rail.create_server")}</button>
        <button type="button" role="menuitem" onclick={() => openDialog("join")} disabled={!canCreate}>{$t("omnidisc.rail.join_invite")}</button>
        <button type="button" role="menuitem" onclick={addInstance}>{$t("omnidisc.rail.add_instance")}</button>
        <button type="button" role="menuitem" onclick={() => { menuOpen = false; goto("/omnidisc/settings?view=voice"); }}>{$t("omnidisc.voice.settings")}</button>
      </div>
    {/if}
  </div>
</nav>

<OmnidiscPrompt
  open={dialog === "create"}
  title={$t("omnidisc.guild.create_title")}
  body={$t("omnidisc.guild.create_body")}
  submitLabel={$t("omnidisc.guild.create_submit")}
  busy={dialogBusy}
  error={dialogError}
  canSubmit={guildName.trim().length > 0}
  onSubmit={submitCreate}
  onClose={closeDialog}
>
  <label class="field">
    <span class="label">{$t("omnidisc.guild.name_label")}</span>
    <input class="input" type="text" bind:value={guildName} placeholder={$t("omnidisc.guild.name_placeholder")} maxlength="100" />
  </label>
</OmnidiscPrompt>

<OmnidiscPrompt
  open={dialog === "join"}
  title={$t("omnidisc.guild.join_title")}
  body={$t("omnidisc.guild.join_body")}
  submitLabel={$t("omnidisc.guild.join_submit")}
  busy={dialogBusy}
  error={dialogError}
  canSubmit={inviteCode.trim().length > 0}
  onSubmit={submitJoin}
  onClose={closeDialog}
>
  <label class="field">
    <span class="label">{$t("omnidisc.guild.join_label")}</span>
    <input class="input" type="text" bind:value={inviteCode} placeholder={$t("omnidisc.guild.join_placeholder")} spellcheck="false" autocomplete="off" />
  </label>
</OmnidiscPrompt>

<style>
  .rail {
    width: 64px;
    flex: 0 0 64px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) 0;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    overflow-x: hidden;
  }

  .rail-item {
    position: relative;
    width: 44px;
    height: 44px;
    flex: 0 0 44px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    text-decoration: none;
    cursor: pointer;
    transition: border-radius var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .rail-item:hover {
    background: var(--surface-hi);
    border-radius: var(--radius-md);
  }

  .rail-item:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .rail-item.active {
    background: var(--accent);
    color: var(--on-accent);
    border-radius: var(--radius-md);
  }

  .rail-item.active::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 50%;
    width: 4px;
    height: 24px;
    transform: translateY(-50%);
    border-radius: var(--radius-full);
    background: var(--accent);
  }

  .rail-item.error {
    box-shadow: inset 0 0 0 1px var(--danger);
  }

  .initials {
    font-size: var(--text-sm);
    font-weight: 600;
    letter-spacing: var(--track-snug);
  }

  .status-dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 12px;
    height: 12px;
    border-radius: var(--radius-full);
    border: 2px solid var(--sidebar-bg);
    background: var(--text-dim);
  }

  .status-dot.connected {
    background: var(--success);
  }

  .status-dot.connecting,
  .status-dot.reconnecting {
    background: var(--warning);
  }

  .status-dot.error,
  .status-dot.signed_out {
    background: var(--danger);
  }

  /* http instead of https: quiet but permanent, because it is a property of
     the server and not a one-off error the user can retry away. */
  .insecure-dot {
    position: absolute;
    left: -3px;
    bottom: -3px;
    width: 14px;
    height: 14px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-full);
    border: 2px solid var(--sidebar-bg);
    background: var(--warning);
    color: var(--bg);
  }

  .rail-action {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .rail-action:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .rail-action:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .rail-divider {
    width: 28px;
    height: 1px;
    background: var(--border);
    margin: var(--space-1) 0;
  }

  .rail-spacer {
    flex: 1;
  }

  .add-wrap {
    position: relative;
  }

  .rail-item.add {
    background: transparent;
    color: var(--text-muted);
    border: 1px dashed var(--border-hi);
  }

  .rail-item.add:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .menu {
    position: absolute;
    left: 52px;
    bottom: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    padding: var(--space-1);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .menu button {
    padding: 8px var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }

  .menu button:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .menu button:disabled {
    color: var(--text-dim);
    cursor: default;
  }

  .menu button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  @media (prefers-reduced-motion: reduce) {
    .rail-item {
      transition: none;
    }
  }
</style>
