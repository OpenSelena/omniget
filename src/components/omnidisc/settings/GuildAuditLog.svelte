<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { loadAuditLog, userName } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscAuditEntry } from "$lib/omnidisc/types";

  let { guildId, instanceId }: { guildId: string; instanceId: string } = $props();

  const ACTIONS = [
    "guild.update",
    "channel.create",
    "channel.update",
    "channel.delete",
    "channel.overwrite",
    "role.create",
    "role.update",
    "role.delete",
    "member.update",
    "member.kick",
    "member.ban",
    "member.unban",
    "member.leave",
    "message.delete",
    "message.bulk_delete",
    "message.pin",
    "message.unpin",
    "guild.transfer",
  ];

  let entries = $state<OmnidiscAuditEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let filter = $state("");
  let loadedKey = $state<string | null>(null);

  async function refresh() {
    loading = true;
    error = null;
    try {
      entries = await loadAuditLog(guildId, filter || undefined);
    } catch (e) {
      error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
      entries = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const key = `${guildId}:${filter}`;
    if (loadedKey === key) return;
    loadedKey = key;
    void refresh();
  });

  function sentence(entry: OmnidiscAuditEntry): string {
    const actor = userName(instanceId, entry.actorId);
    const target = entry.targetId ? userName(instanceId, entry.targetId) : "";
    const key = `omnidisc.audit.${entry.action.replace(".", "_")}`;
    const translated = $t(key, { actor, target });
    return translated === key ? $t("omnidisc.audit.fallback", { actor, action: entry.action, target }) : translated;
  }

  function when(entry: OmnidiscAuditEntry): string {
    const ms = Date.parse(entry.createdAt);
    if (!Number.isFinite(ms)) return "";
    return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }
</script>

<section class="audit">
  <label class="field">
    <span class="field-label">{$t("omnidisc.guild_settings.audit_filter")}</span>
    <select class="field-input" bind:value={filter}>
      <option value="">{$t("omnidisc.guild_settings.audit_all")}</option>
      {#each ACTIONS as action (action)}
        <option value={action}>{$t(`omnidisc.audit_action.${action.replace(".", "_")}`)}</option>
      {/each}
    </select>
  </label>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if loading}
    <div class="state" aria-busy="true">
      {#each Array(4) as _, i (i)}
        <span class="skeleton-line"></span>
      {/each}
    </div>
  {:else if entries.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.guild_settings.audit_empty_title")}</p>
      <p class="state-body">{$t("omnidisc.guild_settings.audit_empty_body")}</p>
    </div>
  {:else}
    <ul class="list">
      {#each entries as entry (entry.id)}
        <li class="row">
          <span class="text">{sentence(entry)}{#if entry.reason} — {entry.reason}{/if}</span>
          <time class="when">{when(entry)}</time>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .audit {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 640px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 260px;
  }

  .field-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text);
  }

  .field-input {
    padding: 8px var(--space-2);
    border-radius: var(--radius-sm);
    border: none;
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .field-input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
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
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-bottom: none;
  }

  .text {
    font-size: var(--text-sm);
    color: var(--text);
    line-height: 1.4;
  }

  .when {
    flex: 0 0 auto;
    font-size: var(--text-xs);
    color: var(--text-muted);
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
    height: 32px;
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
