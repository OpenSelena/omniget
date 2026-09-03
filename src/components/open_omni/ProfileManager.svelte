<script lang="ts">
  import { loadProfiles, saveProfile, deleteProfile, type Profile } from '$lib/api/open_omni';

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
      error = 'Failed to load profiles';
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
      error = 'Please enter a URL or username';
      return;
    }
    error = '';
    try {
      await saveProfile(activePlatform, trimmedUrl);
      newUrl = '';
      await loadPlatformProfiles();
    } catch (err) {
      error = errorMessage(err, 'Failed to add profile');
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
      error = errorMessage(err, 'Failed to delete profile');
      console.error('Failed to delete profile:', err);
    }
  }

  function initials(profile: Profile): string {
    const source = (profile.username || profile.url || '?')
      .replace(/^https?:\/\//, '')
      .replace(/^www\./, '');
    return source.charAt(0).toUpperCase() || '?';
  }

  function switchPlatform(p: string) {
    activePlatform = p;
    loadPlatformProfiles(p);
  }

  $effect(() => {
    // Initial load on mount for activePlatform
    loadPlatformProfiles(activePlatform);
  });
</script>

<div class="pm">
  <div class="pm-segmented" role="tablist" aria-label="Platform">
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
        placeholder="Enter profile URL or username"
        onkeydown={(e) => e.key === 'Enter' && addProfile()}
        autocomplete="off"
        spellcheck="false"
        aria-label="Profile URL or username"
      />
      <button type="button" class="pm-add-btn" onclick={addProfile} disabled={!newUrl.trim() || loading}>
        Add
      </button>
    </div>
  </div>

  {#if loading}
    <p class="pm-empty">Loading profiles...</p>
  {:else if profiles.length === 0}
    <p class="pm-empty">No profiles added yet</p>
  {:else}
    <div class="pm-group">
      {#each profiles as profile (profile.url)}
        <div class="pm-row pm-profile-row">
          <div class="pm-avatar" aria-hidden="true">{initials(profile)}</div>
          <div class="pm-info">
            <p class="pm-name">{profile.username || profile.url}</p>
            {#if profile.username}
              <p class="pm-url">{profile.url}</p>
            {/if}
          </div>
          <div class="pm-actions">
            {#if onSelectProfileForDownload}
              <button
                type="button"
                class="pm-download-btn"
                onclick={() => onSelectProfileForDownload?.(profile.url)}
                aria-label="Download with {profile.username || profile.url}"
              >
                Download
              </button>
            {/if}
            <button
              type="button"
              class="pm-remove"
              onclick={() => promptDelete(profile.url)}
              aria-label="Remove profile {profile.username || profile.url}"
            >
              Remove
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<dialog bind:this={dialogEl} class="pm-dialog" onclose={() => (confirmDeleteUrl = null)}>
  <div class="pm-dialog-content">
    <p class="pm-dialog-title" id="pm-confirm-title">Delete this profile?</p>
    <p class="pm-dialog-url">{confirmDeleteUrl}</p>
    <div class="pm-dialog-actions">
      <button type="button" class="pm-dialog-btn" onclick={cancelDelete}>Cancel</button>
      <button type="button" class="pm-dialog-btn danger" onclick={confirmDelete}>Delete</button>
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
