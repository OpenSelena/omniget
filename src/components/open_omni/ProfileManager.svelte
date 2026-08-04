<script lang="ts">
  import { loadProfiles, saveProfile, deleteProfile } from '$lib/api/open_omni';

  type Profile = {
    url: string;
    username?: string;
    platform: string;
  };

  const platforms = [
    { key: 'instagram', label: 'Instagram' },
    { key: 'tiktok', label: 'TikTok' },
    { key: 'facebook', label: 'Facebook' },
    { key: 'x', label: 'X' }
  ];

  let activePlatform = $state('instagram');
  let profiles = $state<Profile[]>([]);
  let newUrl = $state('');
  let loading = $state(false);
  let error = $state('');

  let requestId = 0;

  async function loadPlatformProfiles() {
    const platform = activePlatform;
    const thisRequest = ++requestId;

    loading = true;
    error = '';
    try {
      const result = await loadProfiles(platform);
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

  let confirmDeleteUrl = $state<string | null>(null);

  function promptDelete(url: string) {
    confirmDeleteUrl = url;
  }

  function cancelDelete() {
    confirmDeleteUrl = null;
  }

  async function confirmDelete() {
    const url = confirmDeleteUrl;
    confirmDeleteUrl = null;
    if (!url) return;

    try {
      await deleteProfile(activePlatform, url);
      await loadPlatformProfiles();
    } catch (err) {
      console.error('Failed to delete profile:', err);
    }
  }

  function initials(profile: Profile): string {
    const source = (profile.username || profile.url || '?').replace(/^https?:\/\//, '').replace(/^www\./, '');
    return source.charAt(0).toUpperCase() || '?';
  }

  $effect(() => {
    loadPlatformProfiles();
  });
</script>

<div class="profile-manager">
  <div class="pill-group">
    {#each platforms as platform}
      <button
        type="button"
        class:active={activePlatform === platform.key}
        onclick={() => activePlatform = platform.key}
      >
        {platform.label}
      </button>
    {/each}
  </div>

  {#if error}
    <div class="error-message">{error}</div>
  {/if}

  <div class="field-card add-profile-card">
    <div class="input-with-button">
      <input
        type="text"
        bind:value={newUrl}
        placeholder="Enter profile URL or username"
        onkeydown={(e) => e.key === 'Enter' && addProfile()}
      />
      <button type="button" class="add-btn" onclick={addProfile} disabled={!newUrl.trim() || loading}>
        Add
      </button>
    </div>
  </div>

  {#if loading}
    <p class="loading">Loading profiles…</p>
  {:else if profiles.length === 0}
    <p class="empty">No profiles added yet</p>
  {:else}
    <ul class="profiles-list">
      {#each profiles as profile}
        <li class="profile-item">
          <div class="profile-info">
            <div class="avatar">{initials(profile)}</div>
            <div>
              <p class="profile-name">{profile.username || profile.url}</p>
              {#if profile.username}
                <p class="profile-url">{profile.url}</p>
              {/if}
            </div>
          </div>
          <button class="delete-btn" onclick={() => promptDelete(profile.url)} aria-label="Delete profile">
            Delete
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if confirmDeleteUrl}
  <div class="confirm-overlay" role="presentation" onclick={cancelDelete}>
    <div class="confirm-dialog" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <p class="confirm-title">Delete this profile?</p>
      <p class="confirm-url">{confirmDeleteUrl}</p>
      <div class="confirm-actions">
        <button class="confirm-no" onclick={cancelDelete}>Cancel</button>
        <button class="confirm-yes" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Centered column, same width budget as DownloadManager so tabs
     don't visibly jump width when switching. */
  .profile-manager {
    padding: 0;
    max-width: 460px;
    width: 100%;
    margin: 0 auto;
  }

  .pill-group {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-bottom: 20px;
  }

  .pill-group button {
    padding: 7px 16px;
    border-radius: 16px;
    border: 1px solid var(--input-border);
    background: var(--button-elevated);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .pill-group button:hover:not(.active) {
    border-color: var(--accent);
  }

  .pill-group button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 500;
  }

  .error-message {
    padding: 10px 14px;
    margin-bottom: 20px;
    background: var(--error);
    color: var(--on-error);
    border-radius: var(--border-radius);
    font-size: 13px;
    text-align: center;
  }

  .field-card {
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-radius: 10px;
    padding: 16px 18px;
  }

  .add-profile-card {
    margin-bottom: 20px;
  }

  .input-with-button {
    display: flex;
    gap: 8px;
  }

  .input-with-button input {
    flex: 1;
    padding: 10px 14px;
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: 8px;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
  }

  .input-with-button input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .add-btn {
    padding: 6px 16px;
    border-radius: 16px;
    border: none;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 500;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }

  .add-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .profiles-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-radius: 10px;
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 500;
    font-size: 13px;
    flex-shrink: 0;
  }

  .profile-name {
    font-size: 13.5px;
    font-weight: 500;
    margin: 0;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-url {
    font-size: 11.5px;
    color: var(--text-secondary);
    margin: 3px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delete-btn {
    padding: 5px 12px;
    border-radius: 12px;
    border: 1px solid var(--error);
    background: transparent;
    color: var(--error);
    font-size: 12px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .delete-btn:hover {
    background: var(--error);
    color: var(--on-error);
  }

  .loading, .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 28px 0;
    font-size: 13px;
    margin: 0;
  }

  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.5));
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .confirm-dialog {
    background: var(--popup-bg, var(--button-elevated));
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius);
    padding: 22px;
    max-width: 380px;
    width: 90%;
    text-align: center;
  }

  .confirm-title {
    font-size: 15px;
    font-weight: 500;
    margin: 0 0 6px;
    color: var(--text);
  }

  .confirm-url {
    font-weight: 400;
    word-break: break-all;
    font-size: 12.5px;
    color: var(--text-secondary);
    margin: 0 0 18px;
  }

  .confirm-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
  }

  .confirm-actions button {
    flex: 1;
    padding: 9px 12px;
    border-radius: 16px;
    font-weight: 500;
    font-size: 13px;
    cursor: pointer;
  }

  .confirm-no {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--input-border);
  }

  .confirm-yes {
    background: var(--error);
    color: var(--on-error);
    border: none;
  }

  .confirm-yes:hover {
    opacity: 0.9;
  }
</style>
