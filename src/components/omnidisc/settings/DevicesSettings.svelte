<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    getDevices,
    getSelectedInstance,
    getThisDeviceId,
    loadDevices,
    revokeDevice,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { OmnidiscDevice } from "$lib/omnidisc/types";
  import OmnidiscPrompt from "../OmnidiscPrompt.svelte";

  let instance = $derived(getSelectedInstance());
  let devices = $derived(getDevices(instance?.id ?? null));
  let thisDeviceId = $derived(getThisDeviceId(instance?.id ?? null));
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let pending = $state<OmnidiscDevice | null>(null);
  let copied = $state<string | null>(null);
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
      await loadDevices(id);
    } catch (e) {
      fail(e);
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

  function label(device: OmnidiscDevice): string {
    return device.name?.trim() || $t("omnidisc.devices.unnamed");
  }

  function when(raw: string): string {
    const ms = Date.parse(raw);
    if (!Number.isFinite(ms)) return "";
    return new Date(ms).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function copy(device: OmnidiscDevice) {
    if (!device.fingerprint) return;
    try {
      await navigator.clipboard.writeText(device.fingerprint);
      copied = device.deviceId;
      setTimeout(() => (copied = null), 2000);
    } catch {
      error = $t("omnidisc.devices.copy_failed");
    }
  }

  async function confirmRevoke() {
    const id = instance?.id;
    const device = pending;
    if (!id || !device) return;
    busy = true;
    error = null;
    try {
      await revokeDevice(id, device.deviceId);
      pending = null;
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  let isThisDevice = $derived(pending?.deviceId === thisDeviceId);
</script>

{#if !instance}
  <div class="state">
    <p class="state-title">{$t("omnidisc.profile_settings.no_instance_title")}</p>
    <p class="state-body">{$t("omnidisc.profile_settings.no_instance_body")}</p>
  </div>
{:else}
  <section class="devices">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <p class="state-body">{$t("omnidisc.devices.fingerprint_hint")}</p>

    {#if loading}
      <div class="state" aria-busy="true">
        {#each Array(2) as _, i (i)}
          <span class="skeleton-line"></span>
        {/each}
      </div>
    {:else if devices.length === 0}
      <div class="state">
        <p class="state-title">{$t("omnidisc.devices.empty_title")}</p>
        <p class="state-body">{$t("omnidisc.devices.empty_body")}</p>
      </div>
    {:else}
      <ul class="list">
        {#each devices as device (device.deviceId)}
          <li class="row">
            <span class="who">
              <span class="name">
                {label(device)}
                {#if device.deviceId === thisDeviceId}
                  <span class="tag">{$t("omnidisc.devices.this_device")}</span>
                {/if}
              </span>
              <span class="meta">{$t("omnidisc.devices.last_seen", { when: when(device.lastSeenAt) })}</span>
              {#if device.fingerprint}
                <button
                  type="button"
                  class="fingerprint"
                  onclick={() => void copy(device)}
                  title={$t("omnidisc.devices.copy")}
                >
                  <code>{device.fingerprint}</code>
                  <span class="copy-state">
                    {copied === device.deviceId ? $t("omnidisc.devices.copied") : $t("omnidisc.devices.copy")}
                  </span>
                </button>
              {/if}
            </span>
            <button type="button" class="ghost" onclick={() => (pending = device)}>
              {$t("omnidisc.devices.revoke")}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<OmnidiscPrompt
  open={pending !== null}
  title={isThisDevice
    ? $t("omnidisc.devices.revoke_this_title")
    : $t("omnidisc.devices.revoke_title", { device: pending ? label(pending) : "" })}
  body={isThisDevice ? $t("omnidisc.devices.revoke_this_body") : $t("omnidisc.devices.revoke_body")}
  submitLabel={$t("omnidisc.devices.revoke_confirm", { device: pending ? label(pending) : "" })}
  {busy}
  {error}
  onSubmit={() => void confirmRevoke()}
  onClose={() => {
    pending = null;
    error = null;
  }}
>
  <p class="state-body">{$t("omnidisc.devices.no_history_warning")}</p>
</OmnidiscPrompt>

<style>
  .devices {
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
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) 0;
    border-bottom: none;
  }

  .who {
    display: flex;
    flex-direction: column;
    gap: 4px;
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

  .fingerprint {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px 0;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
  }

  .fingerprint code {
    font-family: var(--font-mono, monospace);
    font-size: var(--text-xs);
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  .copy-state {
    font-size: var(--text-xs);
    color: var(--accent);
    flex: 0 0 auto;
  }

  .fingerprint:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .ghost {
    flex: 0 0 auto;
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
    height: 56px;
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
