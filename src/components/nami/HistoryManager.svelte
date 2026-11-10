<script lang="ts">
  import { onMount } from "svelte";
  import {
    getRecords,
    getFilteredRecords,
    isLoading,
    isReconciling,
    getPlatformFilter,
    setPlatformFilter,
    getSearchQuery,
    setSearchQuery,
    getReconcileStats,
    clearReconcileStats,
    getError,
    clearError,
    loadHistory,
    reconcileDisk,
    deleteRecord,
    openFile,
    openFolder,
    formatFileSize,
  } from "$lib/stores/nami_history_store.svelte";
  import { t } from "$lib/i18n";
  import type { NamiMediaRecord } from "$lib/api/nami";

  interface Props {
    onSwitchToDownload?: () => void;
  }

  let { onSwitchToDownload }: Props = $props();

  const platforms = [
    { key: "all", labelKey: "nami.platform_all" },
    { key: "instagram", label: "Instagram" },
    { key: "tiktok", label: "TikTok" },
    { key: "facebook", label: "Facebook" },
    { key: "x", label: "X" },
  ];

  let deleteCandidate = $state<NamiMediaRecord | null>(null);
  let dialogEl: HTMLDialogElement | null = $state(null);

  onMount(() => {
    loadHistory();
  });

  function handlePlatformClick(p: string) {
    setPlatformFilter(p);
  }

  function handleSearchInput(e: Event) {
    const target = e.target as HTMLInputElement;
    setSearchQuery(target.value);
  }

  function promptDelete(record: NamiMediaRecord) {
    deleteCandidate = record;
    dialogEl?.showModal();
  }

  function cancelDelete() {
    deleteCandidate = null;
    dialogEl?.close();
  }

  async function confirmDelete() {
    if (!deleteCandidate) return;
    const id = deleteCandidate.id;
    cancelDelete();
    await deleteRecord(id);
  }

  function formatDate(unixSeconds: number): string {
    if (!unixSeconds) return "";
    const d = new Date(unixSeconds * 1000);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }

  function getPlatformBadgeClass(plat: string): string {
    const p = plat.toLowerCase();
    if (p === "instagram") return "badge-ig";
    if (p === "tiktok") return "badge-tt";
    if (p === "facebook") return "badge-fb";
    if (p === "x") return "badge-x";
    return "";
  }
</script>

<div class="hm">
  <div class="hm-toolbar">
    <div class="hm-segmented" role="tablist" aria-label={$t("nami.platform_aria")}>
      {#each platforms as platform (platform.key)}
        <button
          type="button"
          role="tab"
          aria-selected={getPlatformFilter() === platform.key}
          class:on={getPlatformFilter() === platform.key}
          onclick={() => handlePlatformClick(platform.key)}
        >
          {platform.labelKey ? $t(platform.labelKey) : platform.label}
        </button>
      {/each}
    </div>

    <button
      type="button"
      class="hm-sync-btn"
      onclick={() => reconcileDisk()}
      disabled={isReconciling() || isLoading()}
      aria-label={$t("nami.sync_disk")}
    >
      <svg
        class="hm-btn-icon"
        class:spin={isReconciling()}
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.4"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M1.5 8a6.5 6.5 0 0 1 11.1-4.6L14 5" />
        <path d="M14 2v3h-3" />
        <path d="M14.5 8a6.5 6.5 0 0 1-11.1 4.6L2 11" />
        <path d="M2 14v-3h3" />
      </svg>
      <span>{isReconciling() ? $t("nami.syncing_disk") : $t("nami.sync_disk")}</span>
    </button>
  </div>

  <div class="hm-search-box">
    <svg class="hm-search-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true">
      <circle cx="7" cy="7" r="4.5" />
      <path d="m10.5 10.5 3.5 3.5" />
    </svg>
    <input
      type="search"
      value={getSearchQuery()}
      oninput={handleSearchInput}
      placeholder={$t("nami.search_history_placeholder")}
      aria-label={$t("nami.search_history_placeholder")}
      autocomplete="off"
      spellcheck="false"
    />
    {#if getSearchQuery()}
      <button
        type="button"
        class="hm-clear-search"
        onclick={() => setSearchQuery("")}
        aria-label={$t("common.clear")}
      >
        ×
      </button>
    {/if}
  </div>

  {#if getError()}
    <div class="hm-alert error" role="alert">
      <span>{getError()}</span>
      <button type="button" class="hm-alert-dismiss" onclick={clearError}>×</button>
    </div>
  {/if}

  {#if getReconcileStats()}
    <div class="hm-alert info" role="status">
      <span>
        {$t("nami.disk_synced", {
          checked: getReconcileStats()?.checked ?? 0,
          removed: getReconcileStats()?.removed ?? 0,
        })}
      </span>
      <button type="button" class="hm-alert-dismiss" onclick={clearReconcileStats}>×</button>
    </div>
  {/if}

  {#if isLoading()}
    <div class="hm-empty">
      <p>{$t("common.loading")}</p>
    </div>
  {:else if getFilteredRecords().length === 0}
    <div class="hm-empty">
      <p>{$t("nami.no_history_yet")}</p>
      {#if onSwitchToDownload}
        <button type="button" class="hm-action-link" onclick={onSwitchToDownload}>
          {$t("nami.start_download")}
        </button>
      {/if}
    </div>
  {:else}
    <div class="hm-list" role="list">
      {#each getFilteredRecords() as record (record.id)}
        <div class="hm-card" role="listitem">
          <div class="hm-card-header">
            <div class="hm-card-meta">
              <span class="hm-badge {getPlatformBadgeClass(record.platform)}">
                {record.platform.toUpperCase()}
              </span>
              <span class="hm-author" title={record.author ?? record.post_id}>
                {record.author ? `@${record.author}` : `ID: ${record.post_id}`}
              </span>
            </div>
            <span class="hm-date">{formatDate(record.downloaded_at)}</span>
          </div>

          <div class="hm-card-body">
            <p class="hm-path" title={record.file_path}>
              {record.file_path}
            </p>
            <div class="hm-card-footer">
              <span class="hm-size">{formatFileSize(record.file_size)}</span>
              <div class="hm-card-actions">
                <button
                  type="button"
                  class="hm-btn secondary"
                  onclick={() => openFile(record.file_path)}
                  title={$t("nami.open_file")}
                  aria-label={$t("nami.open_file")}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" class="hm-icon-sm" aria-hidden="true">
                    <polygon points="4 2 12 8 4 14 4 2" />
                  </svg>
                  <span>{$t("nami.open_file")}</span>
                </button>
                <button
                  type="button"
                  class="hm-btn secondary"
                  onclick={() => openFolder(record.file_path)}
                  title={$t("nami.open_folder")}
                  aria-label={$t("nami.open_folder")}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" class="hm-icon-sm" aria-hidden="true">
                    <path d="M1.5 3.5A1.5 1.5 0 0 1 3 2h3l1.5 2H13a1.5 1.5 0 0 1 1.5 1.5v7A1.5 1.5 0 0 1 13 14H3a1.5 1.5 0 0 1-1.5-1.5z" />
                  </svg>
                  <span>{$t("nami.open_folder")}</span>
                </button>
                <button
                  type="button"
                  class="hm-btn danger"
                  onclick={() => promptDelete(record)}
                  title={$t("nami.delete_record")}
                  aria-label={$t("nami.delete_record")}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" class="hm-icon-sm" aria-hidden="true">
                    <line x1="3" y1="4" x2="13" y2="4" />
                    <path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1" />
                    <path d="M6 7v5M10 7v5" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<dialog bind:this={dialogEl} class="hm-dialog" onclose={() => (deleteCandidate = null)}>
  <div class="hm-dialog-content">
    <p class="hm-dialog-title">{$t("nami.delete_record")}</p>
    <p class="hm-dialog-body">{$t("nami.confirm_delete_record")}</p>
    {#if deleteCandidate}
      <p class="hm-dialog-path">{deleteCandidate.file_path}</p>
    {/if}
    <div class="hm-dialog-actions">
      <button type="button" class="hm-dialog-btn" onclick={cancelDelete}>{$t("common.cancel")}</button>
      <button type="button" class="hm-dialog-btn danger" onclick={confirmDelete}>{$t("nami.delete")}</button>
    </div>
  </div>
</dialog>

<style>
  .hm {
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
  }

  .hm-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .hm-segmented {
    display: flex;
    background: color-mix(in srgb, var(--oo-text) 6%, transparent);
    border-radius: var(--oo-radius, 9px);
    padding: 2px;
    gap: 1px;
    flex: 1;
    min-width: 240px;
  }

  .hm-segmented button {
    flex: 1;
    min-height: 34px;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 500;
    color: var(--oo-text-secondary);
    border-radius: 7px;
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 150ms ease, color 150ms ease;
  }

  .hm-segmented button.on {
    background: var(--oo-group-bg);
    color: var(--oo-text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .hm-sync-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 34px;
    padding: 6px 14px;
    border-radius: var(--oo-radius-sm);
    background: var(--oo-group-bg);
    border: 1px solid var(--oo-border);
    color: var(--oo-text);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
    flex-shrink: 0;
  }

  .hm-sync-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--oo-text) 5%, var(--oo-group-bg));
  }

  .hm-sync-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hm-btn-icon {
    width: 14px;
    height: 14px;
  }

  .hm-btn-icon.spin {
    animation: hm-spin 1s linear infinite;
  }

  @keyframes hm-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .hm-search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    padding: 0 12px;
    background: var(--oo-group-bg);
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius, 9px);
  }

  .hm-search-icon {
    width: 15px;
    height: 15px;
    color: var(--oo-text-secondary);
    flex-shrink: 0;
  }

  .hm-search-box input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--oo-text);
    font-size: 13px;
    outline: none;
    min-width: 0;
  }

  .hm-clear-search {
    background: transparent;
    border: none;
    color: var(--oo-text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .hm-clear-search:hover {
    color: var(--oo-text);
  }

  .hm-alert {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: var(--oo-radius-sm);
    font-size: 12.5px;
  }

  .hm-alert.error {
    background: var(--error);
    color: var(--on-error);
  }

  .hm-alert.info {
    background: color-mix(in srgb, var(--oo-accent) 15%, var(--oo-group-bg));
    border: 1px solid var(--oo-accent);
    color: var(--oo-text);
  }

  .hm-alert-dismiss {
    background: transparent;
    border: none;
    font-size: 15px;
    cursor: pointer;
    color: inherit;
    padding: 0 4px;
  }

  .hm-empty {
    text-align: center;
    padding: 40px 16px;
    color: var(--oo-text-secondary);
    background: var(--oo-group-bg);
    border-radius: var(--oo-radius, 9px);
    border: 1px dashed var(--oo-border);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .hm-empty p {
    margin: 0;
    font-size: 13px;
  }

  .hm-action-link {
    background: var(--oo-accent);
    color: var(--on-accent);
    border: none;
    border-radius: var(--oo-radius-sm);
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    margin-top: 4px;
  }

  .hm-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hm-card {
    background: var(--oo-group-bg);
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius, 9px);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: border-color 150ms ease;
  }

  .hm-card:hover {
    border-color: color-mix(in srgb, var(--oo-accent) 40%, var(--oo-border));
  }

  .hm-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .hm-card-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .hm-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--oo-text) 8%, transparent);
    color: var(--oo-text-secondary);
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .hm-badge.badge-ig {
    background: color-mix(in srgb, #e1306c 15%, transparent);
    color: #e1306c;
  }

  .hm-badge.badge-tt {
    background: color-mix(in srgb, #00f2fe 15%, transparent);
    color: var(--oo-text);
  }

  .hm-badge.badge-fb {
    background: color-mix(in srgb, #1877f2 15%, transparent);
    color: #1877f2;
  }

  .hm-badge.badge-x {
    background: color-mix(in srgb, var(--oo-text) 12%, transparent);
    color: var(--oo-text);
  }

  .hm-author {
    font-size: 13px;
    font-weight: 600;
    color: var(--oo-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hm-date {
    font-size: 11px;
    color: var(--oo-text-secondary);
    flex-shrink: 0;
  }

  .hm-card-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .hm-path {
    font-size: 11.5px;
    font-family: var(--font-mono, monospace);
    color: var(--oo-text-secondary);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.4;
  }

  .hm-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 2px;
  }

  .hm-size {
    font-size: 11.5px;
    color: var(--oo-text-secondary);
    font-weight: 500;
  }

  .hm-card-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hm-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 28px;
    padding: 4px 10px;
    border-radius: var(--oo-radius-sm);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: background 120ms ease, color 120ms ease;
  }

  .hm-btn.secondary {
    background: color-mix(in srgb, var(--oo-text) 7%, transparent);
    color: var(--oo-text);
  }

  .hm-btn.secondary:hover {
    background: color-mix(in srgb, var(--oo-text) 12%, transparent);
  }

  .hm-btn.danger {
    background: transparent;
    color: var(--oo-text-secondary);
    padding: 4px 6px;
  }

  .hm-btn.danger:hover {
    background: color-mix(in srgb, var(--error, #e53935) 15%, transparent);
    color: var(--error, #e53935);
  }

  .hm-icon-sm {
    width: 12px;
    height: 12px;
  }

  .hm-dialog {
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius, 9px);
    background: var(--oo-content-bg);
    color: var(--oo-text);
    padding: 0;
    max-width: 420px;
    width: 90%;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.28);
  }

  .hm-dialog::backdrop {
    background: rgba(0, 0, 0, 0.45);
  }

  .hm-dialog-content {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .hm-dialog-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }

  .hm-dialog-body {
    font-size: 12.5px;
    color: var(--oo-text-secondary);
    margin: 0;
    line-height: 1.45;
  }

  .hm-dialog-path {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--oo-text-secondary);
    background: var(--oo-group-bg);
    padding: 6px 8px;
    border-radius: 4px;
    word-break: break-all;
    margin: 0;
  }

  .hm-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
  }

  .hm-dialog-btn {
    min-height: 32px;
    padding: 6px 14px;
    border-radius: var(--oo-radius-sm);
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--oo-border);
    background: var(--oo-group-bg);
    color: var(--oo-text);
    cursor: pointer;
  }

  .hm-dialog-btn.danger {
    background: var(--error, #e53935);
    color: var(--on-error, #ffffff);
    border-color: var(--error, #e53935);
  }
</style>
