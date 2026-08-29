<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    getMe,
    getOwnPresence,
    getSelectedInstance,
    setOwnPresence,
    updateMe,
  } from "$lib/stores/omnidisc-store.svelte";

  const PRESENCES = ["online", "idle", "dnd", "invisible"] as const;
  const BIO_MAX = 320;

  let instance = $derived(getSelectedInstance());
  let me = $derived(getMe(instance?.id ?? null));
  let connected = $derived(instance?.status === "connected");

  let displayName = $state("");
  let pronouns = $state("");
  let bio = $state("");
  let accent = $state("#5865f2");
  let statusText = $state("");
  let statusExpiry = $state("0");
  let seeded = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let saved = $state(false);

  $effect(() => {
    if (!me || seeded === me.id) return;
    seeded = me.id;
    displayName = me.displayName;
    pronouns = me.pronouns ?? "";
    bio = me.bio ?? "";
    if (me.accentColor !== undefined) accent = `#${me.accentColor.toString(16).padStart(6, "0")}`;
  });

  let presence = $derived(getOwnPresence(instance?.id ?? null));

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function save() {
    const id = instance?.id;
    if (!id) return;
    busy = true;
    error = null;
    saved = false;
    try {
      await updateMe(id, {
        display_name: displayName.trim() || undefined,
        pronouns: pronouns.trim(),
        bio: bio.trim(),
        accent_color: Number.parseInt(accent.replace("#", ""), 16),
      });
      saved = true;
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function applyPresence(next: string) {
    const id = instance?.id;
    if (!id) return;
    busy = true;
    error = null;
    try {
      const minutes = Number(statusExpiry);
      await setOwnPresence(id, next, {
        text: statusText.trim() || undefined,
        expiresAt: minutes > 0 ? new Date(Date.now() + minutes * 60_000).toISOString() : undefined,
      });
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if !instance}
  <div class="state">
    <p class="state-title">{$t("omnidisc.profile_settings.no_instance_title")}</p>
    <p class="state-body">{$t("omnidisc.profile_settings.no_instance_body")}</p>
  </div>
{:else}
  <section class="profile-settings">
    <div class="preview" aria-label={$t("omnidisc.profile_settings.preview")}>
      <span class="preview-banner" style:background={accent}></span>
      <span class="preview-avatar">{(displayName || me?.username || "?").trim().charAt(0).toUpperCase()}</span>
      <span class="preview-names">
        <strong>{displayName || me?.username}</strong>
        <span class="handle">@{me?.username ?? "…"}</span>
        {#if pronouns}<span class="handle">{pronouns}</span>{/if}
      </span>
      {#if bio}<p class="preview-bio">{bio}</p>{/if}
    </div>

    <label class="field">
      <span class="field-label">{$t("omnidisc.profile_settings.display_name")}</span>
      <input class="field-input" type="text" bind:value={displayName} maxlength="32" disabled={!connected || busy} />
    </label>

    <label class="field">
      <span class="field-label">{$t("omnidisc.profile_settings.pronouns")}</span>
      <input class="field-input" type="text" bind:value={pronouns} maxlength="40" disabled={!connected || busy} placeholder={$t("omnidisc.profile_settings.pronouns_placeholder")} />
    </label>

    <label class="field">
      <span class="field-label">{$t("omnidisc.profile_settings.bio")}</span>
      <textarea class="field-input" rows="3" maxlength={BIO_MAX} bind:value={bio} disabled={!connected || busy}></textarea>
      <span class="counter">{bio.length}/{BIO_MAX}</span>
    </label>

    <label class="field short">
      <span class="field-label">{$t("omnidisc.profile_settings.accent")}</span>
      <input class="field-color" type="color" bind:value={accent} disabled={!connected || busy} />
    </label>

    <button type="button" class="primary" onclick={() => void save()} disabled={!connected || busy}>
      {busy ? $t("omnidisc.guild.working") : $t("omnidisc.profile_settings.save")}
    </button>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {:else if saved}
      <p class="ok" role="status">{$t("omnidisc.profile_settings.saved")}</p>
    {/if}

    <p class="note">{$t("omnidisc.profile_settings.avatar_note")}</p>

    <h4 class="block-title">{$t("omnidisc.profile_settings.presence")}</h4>
    <div class="segmented" role="radiogroup" aria-label={$t("omnidisc.profile_settings.presence")}>
      {#each PRESENCES as option (option)}
        <button
          type="button"
          class="seg"
          class:on={presence === option}
          aria-pressed={presence === option}
          disabled={!connected || busy}
          onclick={() => void applyPresence(option)}
        >
          {$t(`omnidisc.profile_settings.presence_${option}`)}
        </button>
      {/each}
    </div>

    <label class="field">
      <span class="field-label">{$t("omnidisc.profile_settings.custom_status")}</span>
      <input class="field-input" type="text" bind:value={statusText} maxlength="128" disabled={!connected || busy} placeholder={$t("omnidisc.profile_settings.custom_status_placeholder")} />
    </label>

    <label class="field short">
      <span class="field-label">{$t("omnidisc.profile_settings.custom_status_expiry")}</span>
      <select class="field-input" bind:value={statusExpiry} disabled={!connected || busy}>
        <option value="0">{$t("omnidisc.profile_settings.expiry_never")}</option>
        <option value="30">{$t("omnidisc.profile_settings.expiry_30m")}</option>
        <option value="240">{$t("omnidisc.profile_settings.expiry_4h")}</option>
        <option value="1440">{$t("omnidisc.profile_settings.expiry_today")}</option>
      </select>
    </label>

    <button type="button" class="ghost" onclick={() => void applyPresence(presence)} disabled={!connected || busy}>
      {$t("omnidisc.profile_settings.apply_status")}
    </button>
  </section>
{/if}

<style>
  .profile-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 520px;
  }

  .preview {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: 0 var(--space-3) var(--space-3);
    border: none;
    border-radius: var(--border-radius);
    background: var(--surface);
    overflow: hidden;
  }

  .preview-banner {
    height: 48px;
    margin: 0 calc(-1 * var(--space-3));
  }

  .preview-avatar {
    width: 48px;
    height: 48px;
    margin-top: -24px;
    border: 3px solid var(--surface);
    border-radius: var(--radius-full);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-weight: 600;
  }

  .preview-names {
    display: flex;
    flex-direction: column;
  }

  .handle {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .preview-bio {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-secondary, var(--text));
    line-height: 1.45;
    white-space: pre-wrap;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field.short {
    max-width: 220px;
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

  .field-color {
    width: 56px;
    height: 34px;
    padding: 2px;
    border-radius: var(--radius-sm);
    border: none;
    background: var(--input-bg);
  }

  .counter {
    align-self: flex-end;
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
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

  .ghost {
    align-self: flex-start;
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .primary:disabled,
  .ghost:disabled,
  .seg:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .block-title {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
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

  .note {
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

  input:focus-visible,
  select:focus-visible,
  textarea:focus-visible,
  button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
