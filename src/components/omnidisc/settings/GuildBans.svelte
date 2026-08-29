<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { listBans, unbanMember, userName } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscBan } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "../OmnidiscPrompt.svelte";

  let { guildId, instanceId, canBan }: { guildId: string; instanceId: string; canBan: boolean } = $props();

  let bans = $state<OmnidiscBan[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let pending = $state<OmnidiscBan | null>(null);
  let loadedFor = $state<string | null>(null);

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function refresh() {
    loading = true;
    error = null;
    try {
      bans = await listBans(guildId);
    } catch (e) {
      fail(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (loadedFor === guildId) return;
    loadedFor = guildId;
    void refresh();
  });

  async function confirmUnban() {
    const ban = pending;
    if (!ban) return;
    busy = true;
    error = null;
    try {
      await unbanMember(guildId, ban.userId);
      pending = null;
      await refresh();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="bans">
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if loading}
    <div class="state" aria-busy="true">
      {#each Array(3) as _, i (i)}
        <span class="skeleton-line"></span>
      {/each}
    </div>
  {:else if bans.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.guild_settings.bans_empty_title")}</p>
      <p class="state-body">{$t("omnidisc.guild_settings.bans_empty_body")}</p>
    </div>
  {:else}
    <ul class="list">
      {#each bans as ban (ban.userId)}
        <li class="row">
          <span class="who">
            <span class="name">{userName(instanceId, ban.userId)}</span>
            <span class="meta">
              {$t("omnidisc.guild_settings.ban_by", { name: userName(instanceId, ban.bannedBy) })}
              {#if ban.reason}— {ban.reason}{/if}
            </span>
          </span>
          {#if canBan}
            <button type="button" class="ghost" onclick={() => (pending = ban)}>{$t("omnidisc.guild_settings.unban")}</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<OmnidiscPrompt
  open={pending !== null}
  title={$t("omnidisc.guild_settings.unban_title", { name: pending ? userName(instanceId, pending.userId) : "" })}
  body={$t("omnidisc.guild_settings.unban_body")}
  submitLabel={$t("omnidisc.guild_settings.unban_confirm", { name: pending ? userName(instanceId, pending.userId) : "" })}
  {busy}
  {error}
  onSubmit={() => void confirmUnban()}
  onClose={() => (pending = null)}
>
  <p class="state-body">{$t("omnidisc.guild_settings.unban_hint")}</p>
</OmnidiscPrompt>

<style>
  .bans {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-width: 620px;
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
    border-bottom: none;
  }

  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .meta {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .ghost {
    padding: 4px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .ghost:hover {
    background: var(--fill-1);
  }

  .ghost:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
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

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  .skeleton-line {
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    animation: pulse 1.4s ease-in-out infinite;
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
