<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    acceptFriend,
    addFriend,
    blockUser,
    getSelectedInstance,
    getPresence,
    getRelationships,
    getUser,
    loadRelationships,
    openDm,
    removeFriend,
    userName,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscRelationship } from "$lib/omnidisc/types";
  import ProfilePopover from "$components/omnidisc/ProfilePopover.svelte";

  type Tab = "online" | "all" | "pending" | "blocked";

  let instance = $derived(getSelectedInstance());
  let connected = $derived(instance?.status === "connected");
  let tab = $state<Tab>("online");
  let username = $state("");
  let addBusy = $state(false);
  let addError = $state<string | null>(null);
  let addOk = $state(false);
  let listError = $state<string | null>(null);
  let loading = $state(true);
  let busyId = $state<string | null>(null);
  let profile = $state<{ userId: string; x: number; y: number } | null>(null);
  let loadedFor = $state<string | null>(null);

  let relationships = $derived(getRelationships(instance?.id ?? null));
  let friends = $derived(relationships.filter((r) => r.kind === "friend"));
  let incoming = $derived(relationships.filter((r) => r.kind === "incoming_request"));
  let outgoing = $derived(relationships.filter((r) => r.kind === "outgoing_request"));
  let blocked = $derived(relationships.filter((r) => r.kind === "blocked"));
  let onlineFriends = $derived(
    friends.filter((r) => instance && getPresence(instance.id, r.userId) !== "offline"),
  );

  $effect(() => {
    const id = instance?.id;
    if (!id || loadedFor === id) return;
    loadedFor = id;
    loading = true;
    listError = null;
    loadRelationships(id)
      .catch((e) => {
        listError = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
      })
      .finally(() => {
        loading = false;
      });
  });

  function fail(e: unknown): string {
    return translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function submitAdd(e: Event) {
    e.preventDefault();
    const id = instance?.id;
    const name = username.trim();
    if (!id || !name) return;
    addBusy = true;
    addError = null;
    addOk = false;
    try {
      await addFriend(id, name);
      addOk = true;
      username = "";
    } catch (err) {
      addError = fail(err);
    } finally {
      addBusy = false;
    }
  }

  async function act(userId: string, fn: () => Promise<void>) {
    busyId = userId;
    listError = null;
    try {
      await fn();
    } catch (e) {
      listError = fail(e);
    } finally {
      busyId = null;
    }
  }

  async function message(userId: string) {
    const id = instance?.id;
    if (!id) return;
    await act(userId, async () => {
      const channel = await openDm(id, userId);
      if (channel) await goto(`/omnidisc/dm/${channel.id}`);
    });
  }

  function openProfile(userId: string, anchor: HTMLElement) {
    const rect = anchor.getBoundingClientRect();
    const width = typeof window !== "undefined" ? window.innerWidth : 1280;
    const height = typeof window !== "undefined" ? window.innerHeight : 800;
    profile = { userId, x: Math.min(rect.left, width - 320), y: Math.min(rect.bottom + 6, height - 420) };
  }

  function initial(name: string): string {
    return name.trim().charAt(0).toUpperCase() || "?";
  }

  let visible = $derived.by<OmnidiscRelationship[]>(() => {
    switch (tab) {
      case "online":
        return onlineFriends;
      case "all":
        return friends;
      case "pending":
        return [...incoming, ...outgoing];
      case "blocked":
        return blocked;
    }
  });
</script>

<div class="friends">
  <header class="bar">
    <h2 class="title">{$t("omnidisc.friends.title")}</h2>
    <nav class="tabs" aria-label={$t("omnidisc.friends.title")}>
      <button type="button" class="tab" class:on={tab === "online"} onclick={() => (tab = "online")}>{$t("omnidisc.friends.tab_online")}</button>
      <button type="button" class="tab" class:on={tab === "all"} onclick={() => (tab = "all")}>{$t("omnidisc.friends.tab_all", { count: friends.length })}</button>
      <button type="button" class="tab" class:on={tab === "pending"} onclick={() => (tab = "pending")}>
        {$t("omnidisc.friends.tab_pending")}
        {#if incoming.length > 0}<span class="pill">{incoming.length}</span>{/if}
      </button>
      <button type="button" class="tab" class:on={tab === "blocked"} onclick={() => (tab = "blocked")}>{$t("omnidisc.friends.tab_blocked")}</button>
    </nav>
  </header>

  <div class="content">
    <form class="add" onsubmit={submitAdd}>
      <label class="add-label" for="od-friend-username">{$t("omnidisc.friends.add_label")}</label>
      <div class="add-row">
        <input
          id="od-friend-username"
          class="input"
          type="text"
          bind:value={username}
          placeholder={$t("omnidisc.friends.add_placeholder")}
          spellcheck="false"
          maxlength="32"
          disabled={!connected || addBusy}
        />
        <button type="submit" class="primary" disabled={!connected || addBusy || username.trim().length === 0}>
          {addBusy ? $t("omnidisc.friends.sending") : $t("omnidisc.friends.send_request")}
        </button>
      </div>
      {#if addError}
        <p class="error" role="alert">{addError}</p>
      {:else if addOk}
        <p class="ok" role="status">{$t("omnidisc.friends.request_sent")}</p>
      {:else}
        <p class="hint">{$t("omnidisc.friends.add_hint")}</p>
      {/if}
    </form>

    {#if listError}
      <p class="error" role="alert">{listError}</p>
    {/if}

    {#if loading}
      <div class="state" aria-busy="true">
        {#each Array(4) as _, i (i)}
          <span class="skeleton-line"></span>
        {/each}
      </div>
    {:else if visible.length === 0}
      <div class="state">
        <p class="state-title">{$t(`omnidisc.friends.empty_${tab}_title`)}</p>
        <p class="state-body">{$t(`omnidisc.friends.empty_${tab}_body`)}</p>
      </div>
    {:else}
      <ul class="list">
        {#each visible as rel (rel.userId)}
          {@const name = instance ? userName(instance.id, rel.userId) : rel.userId}
          {@const user = instance ? getUser(instance.id, rel.userId) : null}
          <li class="row" aria-busy={busyId === rel.userId}>
            <button type="button" class="who" onclick={(e) => openProfile(rel.userId, e.currentTarget)}>
              <span class="avatar" aria-hidden="true">
                {initial(name)}
                <span class="dot" class:online={instance && getPresence(instance.id, rel.userId) !== "offline"}></span>
              </span>
              <span class="names">
                <span class="name">{name}</span>
                <span class="handle">@{user?.username ?? "…"}</span>
              </span>
            </button>
            <div class="row-actions">
              {#if rel.kind === "friend"}
                <button type="button" class="ghost" onclick={() => void message(rel.userId)} disabled={!connected}>{$t("omnidisc.profile.message")}</button>
                <button type="button" class="ghost" onclick={() => instance && void act(rel.userId, () => removeFriend(instance.id, rel.userId))}>{$t("omnidisc.friends.remove")}</button>
              {:else if rel.kind === "incoming_request"}
                <button type="button" class="primary small" onclick={() => instance && void act(rel.userId, () => acceptFriend(instance.id, rel.userId))}>{$t("omnidisc.friends.accept")}</button>
                <button type="button" class="ghost" onclick={() => instance && void act(rel.userId, () => removeFriend(instance.id, rel.userId))}>{$t("omnidisc.friends.decline")}</button>
              {:else if rel.kind === "outgoing_request"}
                <span class="tag">{$t("omnidisc.friends.pending_out")}</span>
                <button type="button" class="ghost" onclick={() => instance && void act(rel.userId, () => removeFriend(instance.id, rel.userId))}>{$t("omnidisc.friends.cancel_request")}</button>
              {:else}
                <button type="button" class="ghost" onclick={() => instance && void act(rel.userId, () => removeFriend(instance.id, rel.userId))}>{$t("omnidisc.friends.unblock")}</button>
              {/if}
              {#if rel.kind !== "blocked"}
                <button type="button" class="ghost danger" onclick={() => instance && void act(rel.userId, () => blockUser(instance.id, rel.userId))}>{$t("omnidisc.friends.block")}</button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

{#if profile && instance}
  <ProfilePopover
    instanceId={instance.id}
    userId={profile.userId}
    x={profile.x}
    y={profile.y}
    onClose={() => (profile = null)}
  />
{/if}

<style>
  .friends {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: 48px;
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }

  .title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
  }

  .tabs {
    display: flex;
    gap: var(--space-1);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 4px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .tab:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .tab.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .pill {
    min-width: 16px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-xs);
    font-weight: 600;
    text-align: center;
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
    max-width: 720px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .add {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }

  .add-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .add-row {
    display: flex;
    gap: var(--space-2);
  }

  .input {
    flex: 1;
    min-width: 0;
    padding: 8px var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .input:focus-visible,
  .primary:focus-visible,
  .ghost:focus-visible,
  .tab:focus-visible,
  .who:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .primary {
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

  .primary.small {
    padding: 4px var(--space-3);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .hint,
  .ok,
  .error {
    margin: 0;
    font-size: var(--text-xs);
  }

  .hint {
    color: var(--text-muted);
  }

  .ok {
    color: var(--success);
  }

  .error {
    color: var(--danger);
  }

  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
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
    line-height: 1.5;
  }

  .skeleton-line {
    height: 44px;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--border);
  }

  .who {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    cursor: pointer;
    text-align: left;
    min-width: 0;
  }

  .avatar {
    position: relative;
    width: 32px;
    height: 32px;
    flex: 0 0 32px;
    border-radius: var(--radius-full);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .dot {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    border: 2px solid var(--bg);
    background: var(--text-dim);
  }

  .dot.online {
    background: var(--success);
  }

  .names {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-size: var(--text-sm);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .handle {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .ghost {
    padding: 4px var(--space-2);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .ghost:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .ghost.danger {
    color: var(--danger);
  }

  .tag {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-line {
      animation: none;
    }
  }
</style>
