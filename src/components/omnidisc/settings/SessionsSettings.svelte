<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { getSelectedInstance, listSessions, revokeOtherSessions, revokeSession } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscSession } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "../OmnidiscPrompt.svelte";

  let instance = $derived(getSelectedInstance());
  let sessions = $state<OmnidiscSession[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let pending = $state<OmnidiscSession | null>(null);
  let revokeAll = $state(false);
  let loadedFor = $state<string | null>(null);

  function fail(e: unknown) {
    error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
  }

  async function refresh() {
    const id = instance?.id;
    if (!id) return;
    loading = true;
    error = null;
    try {
      sessions = await listSessions(id);
    } catch (e) {
      fail(e);
      sessions = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = instance?.id;
    if (!id || loadedFor === id) return;
    loadedFor = id;
    void refresh();
  });

  function label(session: OmnidiscSession): string {
    return session.deviceName || session.client || $t("omnidisc.sessions.unknown_device");
  }

  function when(session: OmnidiscSession): string {
    const ms = Date.parse(session.lastSeen);
    if (!Number.isFinite(ms)) return "";
    return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  async function confirmRevoke() {
    const id = instance?.id;
    if (!id) return;
    busy = true;
    error = null;
    try {
      if (revokeAll) await revokeOtherSessions(id);
      else if (pending) await revokeSession(id, pending.sessionId);
      pending = null;
      revokeAll = false;
      await refresh();
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
  <section class="sessions">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if loading}
      <div class="state" aria-busy="true">
        {#each Array(3) as _, i (i)}
          <span class="skeleton-line"></span>
        {/each}
      </div>
    {:else if sessions.length === 0}
      <div class="state">
        <p class="state-title">{$t("omnidisc.sessions.empty_title")}</p>
        <p class="state-body">{$t("omnidisc.sessions.empty_body")}</p>
      </div>
    {:else}
      <ul class="list">
        {#each sessions as session (session.sessionId)}
          <li class="row">
            <span class="who">
              <span class="name">
                {label(session)}
                {#if session.current}<span class="tag">{$t("omnidisc.sessions.current")}</span>{/if}
              </span>
              <span class="meta">{$t("omnidisc.sessions.last_seen", { when: when(session) })}</span>
            </span>
            <button
              type="button"
              class="ghost"
              onclick={() => {
                revokeAll = false;
                pending = session;
              }}
            >
              {session.current ? $t("omnidisc.sessions.sign_out_here") : $t("omnidisc.sessions.revoke")}
            </button>
          </li>
        {/each}
      </ul>

      {#if sessions.length > 1}
        <button
          type="button"
          class="ghost danger"
          onclick={() => {
            pending = null;
            revokeAll = true;
          }}
        >
          {$t("omnidisc.sessions.revoke_others", { count: sessions.length - 1 })}
        </button>
      {/if}
    {/if}
  </section>
{/if}

<OmnidiscPrompt
  open={pending !== null || revokeAll}
  title={revokeAll
    ? $t("omnidisc.sessions.revoke_others_title")
    : $t("omnidisc.sessions.revoke_title", { device: pending ? label(pending) : "" })}
  body={revokeAll ? $t("omnidisc.sessions.revoke_others_body") : $t("omnidisc.sessions.revoke_body")}
  submitLabel={revokeAll
    ? $t("omnidisc.sessions.revoke_others_confirm")
    : $t("omnidisc.sessions.revoke_confirm", { device: pending ? label(pending) : "" })}
  {busy}
  {error}
  onSubmit={() => void confirmRevoke()}
  onClose={() => {
    pending = null;
    revokeAll = false;
    error = null;
  }}
>
  <p class="state-body">{$t("omnidisc.sessions.revoke_hint")}</p>
</OmnidiscPrompt>

<style>
  .sessions {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 560px;
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
    flex-direction: column;
    min-width: 0;
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .tag {
    padding: 1px var(--space-2);
    border-radius: var(--radius-full);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .meta {
    font-size: var(--text-xs);
    color: var(--text-muted);
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

  .ghost:hover {
    background: var(--fill-1);
  }

  .ghost.danger {
    align-self: flex-start;
    color: var(--danger);
    border-color: var(--danger);
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
