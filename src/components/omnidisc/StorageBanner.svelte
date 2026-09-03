<script lang="ts">
  import { t } from "$lib/i18n";
  import { getSelectedInstance, getStorage } from "$lib/stores/omnidisc-store.svelte";

  // A wipe that arrives unannounced reads as data loss. Warning early — while
  // there is still room — is what turns the policy into something people can
  // act on instead of something that happens to them.
  let instance = $derived(getSelectedInstance());
  let storage = $derived(getStorage(instance?.id ?? null));
  let dismissed = $state<string | null>(null);
  let key = $derived(storage ? `${storage.level}:${storage.purgedFiles ?? 0}` : null);
  let visible = $derived(
    !!storage && storage.level !== "ok" && key !== null && dismissed !== key,
  );

  function percent(ratio: number): number {
    return Math.min(100, Math.round(ratio * 100));
  }

  function minutes(seconds: number): number {
    return Math.max(1, Math.round(seconds / 60));
  }
</script>

{#if visible && storage}
  <div
    class="storage-banner"
    class:critical={storage.level !== "warning"}
    role={storage.level === "warning" ? "status" : "alert"}
  >
    <span class="text">
      {#if storage.level === "purged"}
        {$t("omnidisc.storage.purged", { count: storage.purgedFiles ?? 0 })}
      {:else if storage.level === "critical"}
        {$t("omnidisc.storage.critical", { percent: percent(storage.ratio) })}
      {:else}
        {$t("omnidisc.storage.warning", {
          percent: percent(storage.ratio),
          minutes: minutes(storage.attachmentTtlSeconds),
        })}
      {/if}
    </span>
    <button
      type="button"
      class="dismiss"
      onclick={() => (dismissed = key)}
      aria-label={$t("common.dismiss")}
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12" /></svg>
    </button>
  </div>
{/if}

<style>
  .storage-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--warning) 16%, var(--surface));
    border-bottom: 1px solid color-mix(in srgb, var(--warning) 40%, var(--border));
    color: var(--text);
    font-size: var(--text-sm);
    line-height: 1.4;
  }

  .storage-banner.critical {
    background: color-mix(in srgb, var(--danger) 16%, var(--surface));
    border-bottom-color: color-mix(in srgb, var(--danger) 40%, var(--border));
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .dismiss {
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .dismiss:hover {
    color: var(--text);
    background: var(--surface-hi);
  }

  .dismiss:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
