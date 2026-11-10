<script lang="ts">
  import { onMount } from 'svelte';
  import {
    loadProfiles,
    saveProfile,
    deleteProfile,
    pullCreator,
    pullAllCreators,
    type Profile,
  } from '$lib/api/nami';
  import { t } from '$lib/i18n';

  interface Props {
    onSelectProfileForDownload?: (url: string) => void;
  }

  let { onSelectProfileForDownload }: Props = $props();

  const platforms = [
    { key: 'instagram', label: 'Instagram' },
    { key: 'tiktok', label: 'TikTok' },
    { key: 'facebook', label: 'Facebook' },
    { key: 'x', label: 'X' },
  ];

  let activePlatform = $state('instagram');
  let profiles = $state<Profile[]>([]);
  let newUrl = $state('');
  let loading = $state(false);
  let error = $state('');
  let confirmDeleteUrl = $state<string | null>(null);
  let syncingProfileUrl = $state<string | null>(null);
  let syncingAll = $state(false);
  let syncFeedback = $state<string | null>(null);

  let requestId = 0;
  let dialogEl: HTMLDialogElement | null = $state(null);

  async function loadPlatformProfiles(targetPlatform = activePlatform) {
    const thisRequest = ++requestId;
    loading = true;
    error = '';
    try {
      const result = await loadProfiles(targetPlatform);
      if (thisRequest !== requestId) return;
      profiles = result;
    } catch (err) {
      if (thisRequest !== requestId) return;
      error = $t('nami.failed_to_load_profiles') as string;
      console.error(err);
    } finally {
      if (thisRequest === requestId) {
        loading = false;
      }
    }
  }

  function errorMessage(err: unknown, fallback: string): string {
    if (typeof err === 'string') return err;
    if (err instanceof Error) return err.message;
    return fallback;
  }

  async function addProfile() {
    const trimmedUrl = newUrl.trim();
    if (!trimmedUrl) {
      error = $t('nami.enter_url_or_username') as string;
      return;
    }
    error = '';
    try {
      await saveProfile(activePlatform, trimmedUrl);
      newUrl = '';
      await loadPlatformProfiles();
    } catch (err) {
      error = errorMessage(err, $t('nami.failed_to_add_profile') as string);
      console.error(err);
    }
  }

  function promptDelete(url: string) {
    confirmDeleteUrl = url;
    dialogEl?.showModal();
  }

  function cancelDelete() {
    confirmDeleteUrl = null;
    dialogEl?.close();
  }

  async function confirmDelete() {
    const url = confirmDeleteUrl;
    cancelDelete();
    if (!url) return;
    try {
      await deleteProfile(activePlatform, url);
      await loadPlatformProfiles();
    } catch (err) {
      error = errorMessage(err, $t('nami.failed_to_delete_profile') as string);
      console.error('Failed to delete profile:', err);
    }
  }

  function initials(profile: Profile): string {
    const source = (profile.username || profile.url || '?')
      .replace(/^https?:\/\//, '')
      .replace(/^www\./, '');
    return source.charAt(0).toUpperCase() || '?';
  }

  function formatSynced(timestamp?: number | null): string {
    if (!timestamp) return '';
    const d = new Date(timestamp * 1000);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }

  async function handlePull(profile: Profile) {
    syncingProfileUrl = profile.url;
    syncFeedback = null;
    error = '';
    try {
      const res = await pullCreator(profile.url, activePlatform);
      if (res.success) {
        syncFeedback = $t('nami.pull_success', {
          name: profile.username || profile.url,
          count: res.files_downloaded,
        }) as string;
        await loadPlatformProfiles();
      } else {
        error = res.message || 'Sync failed';
      }
    } catch (err) {
      error = errorMessage(err, 'Failed to sync profile');
    } finally {
      syncingProfileUrl = null;
    }
  }

  async function handlePullAll() {
    if (profiles.length === 0 || syncingAll) return;
    syncingAll = true;
    syncFeedback = null;
    error = '';
    try {
      const summary = await pullAllCreators(activePlatform);
      syncFeedback = $t('nami.pull_all_success', {
        total: summary.succeeded,
        count: summary.total_downloaded,
      }) as string;
      await loadPlatformProfiles();
    } catch (err) {
      error = errorMessage(err, 'Failed to sync all creators');
    } finally {
      syncingAll = false;
    }
  }

  function switchPlatform(p: string) {
    activePlatform = p;
    loadPlatformProfiles(p);
  }

  onMount(() => {
    loadPlatformProfiles(activePlatform);
  });
</script>

<div class="pm">
  <div class="pm-segmented" role="tablist" aria-label={$t('nami.platform_aria')}>
    {#each platforms as platform (platform.key)}
      <button
        type="button"
        role="tab"
        aria-selected={activePlatform === platform.key}
        class:on={activePlatform === platform.key}
        onclick={() => switchPlatform(platform.key)}
      >
        {platform.label}
      </button>
    {/each}
  </div>

  {#if error}
    <div class="pm-alert" role="alert">{error}</div>
  {/if}

  {#if syncFeedback}
    <div class="pm-alert info" role="status">
      <span>{syncFeedback}</span>
      <button type="button" class="pm-alert-dismiss" onclick={() => (syncFeedback = null)}>×</button>
    </div>
  {/if}

  <div class="pm-group">
    <div class="pm-row">
      <svg class="pm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" aria-hidden="true">
        <path d="M5.5 10.5c-.8.8-2.2.8-3 0-.8-.8-.8-2.2 0-3L4 6" />
        <path d="M10.5 5.5c.8-.8 2.2-.8 3 0 .8.8.8 2.2 0 3L12 10" />
        <path d="M6.3 9.7 9.7 6.3" />
      </svg>
      <input
        id="profile-url"
        type="text"
        bind:value={newUrl}
        placeholder={$t('nami.profile_placeholder')}
        onkeydown={(e) => e.key === 'Enter' && addProfile()}
        autocomplete="off"
        spellcheck="false"
        aria-label={$t('nami.profile_url_aria')}
      />
      <button type="button" class="pm-add-btn" onclick={addProfile} disabled={!newUrl.trim() || loading}>
        {$t('nami.add')}
      </button>
    </div>
  </div>

  {#if loading}
    <p class="pm-empty">{$t('nami.loading_profiles')}</p>
  {:else if profiles.length === 0}
    <p class="pm-empty">{$t('nami.no_profiles_yet')}</p>
  {:else}
    <div class="pm-header-actions">
      <span class="pm-count">{profiles.length} {profiles.length === 1 ? 'profile' : 'profiles'}</span>
      <button
        type="button"
        class="pm-sync-all-btn"
        onclick={handlePullAll}
        disabled={syncingAll}
      >
        <svg
          class="pm-btn-icon"
          class:spin={syncingAll}
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
        <span>{syncingAll ? $t('nami.syncing_creator') : $t('nami.sync_all_creators')}</span>
      </button>
    </div>

    <div class="pm-group">
      {#each profiles as profile (profile.url)}
        <div class="pm-row pm-profile-row">
          <div class="pm-avatar" aria-hidden="true">{initials(profile)}</div>
          <div class="pm-info">
            <p class="pm-name">
              {profile.username || profile.url}
              {#if profile.last_pulled_at}
                <span class="pm-synced">• {formatSynced(profile.last_pulled_at)}</span>
              {/if}
            </p>
            {#if profile.username}
              <p class="pm-url">{profile.url}</p>
            {/if}
          </div>
          <div class="pm-actions">
            <button
              type="button"
              class="pm-sync-btn"
              onclick={() => handlePull(profile)}
              disabled={syncingProfileUrl === profile.url}
              aria-label={$t('nami.sync_creator')}
              title={$t('nami.sync_creator')}
            >
              <svg
                class="pm-btn-icon"
                class:spin={syncingProfileUrl === profile.url}
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
              <span>{syncingProfileUrl === profile.url ? $t('nami.syncing_creator') : $t('nami.sync_creator')}</span>
            </button>
            {#if onSelectProfileForDownload}
              <button
                type="button"
                class="pm-download-btn"
                onclick={() => onSelectProfileForDownload?.(profile.url)}
                aria-label={$t('nami.download_with', { name: profile.username || profile.url })}
              >
                {$t('nami.download')}
              </button>
            {/if}
            <button
              type="button"
              class="pm-remove"
              onclick={() => promptDelete(profile.url)}
              aria-label={$t('nami.remove_profile_aria', { name: profile.username || profile.url })}
            >
              {$t('nami.remove')}
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<dialog bind:this={dialogEl} class="pm-dialog" onclose={() => (confirmDeleteUrl = null)}>
  <div class="pm-dialog-content">
    <p class="pm-dialog-title" id="pm-confirm-title">{$t('nami.delete_profile_title')}</p>
    <p class="pm-dialog-url">{confirmDeleteUrl}</p>
    <div class="pm-dialog-actions">
      <button type="button" class="pm-dialog-btn" onclick={cancelDelete}>{$t('common.cancel')}</button>
      <button type="button" class="pm-dialog-btn danger" onclick={confirmDelete}>{$t('nami.delete')}</button>
    </div>
  </div>
</dialog>

<style>
  .pm {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .pm-segmented {
    display: flex;
    background: color-mix(in srgb, var(--oo-text) 6%, transparent);
    border-radius: var(--oo-radius, 9px);
    padding: 2px;
    gap: 1px;
  }

  .pm-segmented button {
    flex: 1;
    min-height: 38px;
    padding: 7px 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--oo-text-secondary);
    border-radius: 7px;
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 150ms ease, color 150ms ease;
  }

  .pm-segmented button.on {
    background: var(--oo-group-bg);
    color: var(--oo-text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .pm-alert {
    padding: 9px 12px;
    background: var(--error);
    color: var(--on-error);
    border-radius: var(--oo-radius, 9px);
    font-size: 12.5px;
  }

  .pm-group {
    background: var(--oo-group-bg);
    border-radius: var(--oo-radius, 9px);
    border: 1px solid var(--oo-border);
    overflow: hidden;
  }

  .pm-row {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 48px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--oo-border);
  }

  .pm-row:last-child {
    border-bottom: 0;
  }

  .pm-icon {
    width: 15px;
    height: 15px;
    color: var(--oo-text-secondary);
    flex-shrink: 0;
  }

  .pm-row input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--oo-text);
    font-size: 13px;
    min-width: 0;
  }

  .pm-row input:focus {
    outline: none;
  }

  .pm-add-btn {
    min-height: 34px;
    padding: 6px 15px;
    border-radius: var(--oo-radius-sm);
    background: var(--oo-accent);
    color: var(--on-accent);
    border: none;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .pm-add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .pm-profile-row {
    gap: 10px;
  }

  .pm-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--oo-accent);
    color: var(--on-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .pm-info {
    min-width: 0;
    flex: 1;
  }

  .pm-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--oo-text);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pm-url {
    font-size: 11px;
    color: var(--oo-text-secondary);
    margin: 2px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pm-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .pm-sync-btn {
    min-height: 30px;
    padding: 4px 10px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--oo-text);
    background: color-mix(in srgb, var(--oo-text) 8%, transparent);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: background 150ms ease;
  }

  .pm-sync-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--oo-text) 14%, transparent);
  }

  .pm-sync-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .pm-sync-all-btn {
    min-height: 30px;
    padding: 4px 12px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--oo-text);
    background: var(--oo-group-bg);
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius-sm, 7px);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: background 150ms ease;
  }

  .pm-sync-all-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--oo-text) 6%, var(--oo-group-bg));
  }

  .pm-sync-all-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .pm-btn-icon {
    width: 13px;
    height: 13px;
  }

  .pm-btn-icon.spin {
    animation: pm-spin 1s linear infinite;
  }

  @keyframes pm-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .pm-synced {
    font-size: 10.5px;
    color: var(--oo-text-secondary);
    margin-left: 6px;
    font-weight: normal;
  }

  .pm-header-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px;
  }

  .pm-count {
    font-size: 12px;
    color: var(--oo-text-secondary);
  }

  .pm-alert.info {
    background: color-mix(in srgb, var(--oo-accent) 15%, var(--oo-group-bg));
    border: 1px solid var(--oo-accent);
    color: var(--oo-text);
    padding: 8px 12px;
    border-radius: var(--oo-radius, 9px);
    font-size: 12.5px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .pm-alert-dismiss {
    background: transparent;
    border: none;
    font-size: 15px;
    cursor: pointer;
    color: inherit;
    padding: 0 4px;
  }

  .pm-download-btn {
    min-height: 30px;
    padding: 4px 10px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--oo-accent);
    background: color-mix(in srgb, var(--oo-accent) 10%, transparent);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .pm-download-btn:hover {
    background: color-mix(in srgb, var(--oo-accent) 18%, transparent);
  }

  .pm-remove {
    min-height: 30px;
    padding: 4px 8px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--error);
    background: transparent;
    border: none;
    cursor: pointer;
    flex-shrink: 0;
  }

  .pm-remove:hover {
    opacity: 0.75;
  }

  .pm-empty {
    text-align: center;
    color: var(--oo-text-secondary);
    padding: 18px 0;
    font-size: 12.5px;
  }

  .pm-dialog {
    background: var(--popup-bg, #252528);
    color: var(--text, #fff);
    border: 1px solid var(--oo-border);
    border-radius: 12px;
    padding: 20px;
    max-width: 340px;
    width: 90%;
    text-align: center;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .pm-dialog::backdrop {
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.5));
  }

  .pm-dialog-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--oo-text);
    margin: 0 0 6px;
  }

  .pm-dialog-url {
    font-size: 12px;
    color: var(--oo-text-secondary);
    overflow-wrap: anywhere;
    margin: 0 0 16px;
  }

  .pm-dialog-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .pm-dialog-btn {
    flex: 1;
    min-height: 38px;
    padding: 8px 10px;
    border-radius: 7px;
    font-weight: 500;
    font-size: 12.5px;
    background: color-mix(in srgb, var(--oo-text) 8%, transparent);
    color: var(--oo-text);
    border: none;
    cursor: pointer;
  }

  .pm-dialog-btn.danger {
    background: var(--error);
    color: var(--on-error);
  }

  .pm-dialog-btn.danger:hover {
    opacity: 0.9;
  }

  @media (max-width: 440px) {
    .pm-segmented {
      flex-wrap: wrap;
    }
    .pm-segmented button {
      flex: 1 1 42%;
    }
    .pm-row {
      padding-inline: 12px;
    }
  }
</style>
