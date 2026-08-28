<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { createInviteWithOptions, getGuild } from "$lib/stores/omnidisc-store.svelte";

  let { guildId, canInvite }: { guildId: string; canInvite: boolean } = $props();

  let guild = $derived(getGuild(guildId));
  let textChannels = $derived((guild?.channels ?? []).filter((c) => c.kind === "text"));
  let channelId = $state<string>("");
  let maxAge = $state("86400");
  let maxUses = $state("0");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let link = $state<string | null>(null);
  let copied = $state(false);

  async function create() {
    busy = true;
    error = null;
    copied = false;
    try {
      const created = await createInviteWithOptions(
        guildId,
        channelId || undefined,
        Number(maxAge) || null,
        Number(maxUses) || null,
      );
      link = created;
      if (!created) error = $t("omnidisc.guild.invite_failed");
    } catch (e) {
      error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
    } catch {
      error = $t("omnidisc.guild.invite_failed");
    }
  }
</script>

<section class="invites">
  <p class="hint">{$t("omnidisc.guild_settings.invites_hint")}</p>

  <div class="fields">
    <label class="field">
      <span class="field-label">{$t("omnidisc.guild_settings.invite_channel")}</span>
      <select class="field-input" bind:value={channelId} disabled={!canInvite || busy}>
        <option value="">{$t("omnidisc.guild_settings.invite_channel_any")}</option>
        {#each textChannels as channel (channel.id)}
          <option value={channel.id}>#{channel.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span class="field-label">{$t("omnidisc.guild_settings.invite_expiry")}</span>
      <select class="field-input" bind:value={maxAge} disabled={!canInvite || busy}>
        <option value="1800">{$t("omnidisc.guild_settings.expiry_30m")}</option>
        <option value="86400">{$t("omnidisc.guild_settings.expiry_1d")}</option>
        <option value="604800">{$t("omnidisc.guild_settings.expiry_7d")}</option>
        <option value="0">{$t("omnidisc.guild_settings.expiry_never")}</option>
      </select>
    </label>

    <label class="field">
      <span class="field-label">{$t("omnidisc.guild_settings.invite_uses")}</span>
      <select class="field-input" bind:value={maxUses} disabled={!canInvite || busy}>
        <option value="0">{$t("omnidisc.guild_settings.uses_unlimited")}</option>
        <option value="1">1</option>
        <option value="5">5</option>
        <option value="25">25</option>
      </select>
    </label>
  </div>

  <button type="button" class="primary" onclick={() => void create()} disabled={!canInvite || busy}>
    {busy ? $t("omnidisc.guild.working") : $t("omnidisc.guild_settings.invite_create")}
  </button>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if link}
    <div class="result" role="status">
      <code class="link">{link}</code>
      <button type="button" class="ghost" onclick={() => void copy()}>
        {copied ? $t("omnidisc.guild.invite_copied") : $t("omnidisc.guild_settings.invite_copy")}
      </button>
    </div>
  {:else}
    <p class="hint">{$t("omnidisc.guild_settings.invites_empty")}</p>
  {/if}

  <p class="note">{$t("omnidisc.guild_settings.invites_no_list")}</p>
</section>

<style>
  .invites {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 560px;
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1 1 160px;
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

  .result {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .link {
    flex: 1;
    font-size: var(--text-xs);
    color: var(--text);
    overflow-wrap: anywhere;
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

  .hint,
  .note {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  select:focus-visible,
  button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
