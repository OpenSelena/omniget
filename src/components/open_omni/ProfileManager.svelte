<script lang="ts">
  import { loadProfiles, saveProfile, deleteProfile } from '$lib/api/open_omni';
  import { onMount } from 'svelte';

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
  let confirmDeleteUrl = $state<string | null>(null);

  let requestId = 0;
  let dialogEl: HTMLDivElement | null = $state(null);

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

  function onDialogKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelDelete();
    }
  }

  // Focus the dialog when it opens so keyboard users land inside it.
  $effect(() => {
    if (confirmDeleteUrl && dialogEl) {
      dialogEl.focus();
    }
  });

  // Reload profiles whenever the active platform changes.
  $effect(() => {
    // Read activePlatform so the effect tracks it.
    void activePlatform;
    loadPlatformProfiles();
  });
</script>

<div class="profile-manager">
  <div class="pill-group" role="tablist" aria-label="Platform">
    {#each platforms as platform}
      <button
        type="button"
        role="tab"
        aria-selected={activePlatform === platform.key}
        class:active={activePlatform === platform.key}
        onclick={() => (activePlatform = platform.key)}
      >
        {platform.label}
      </button>
    {/each}
  </div>

  {#if error}
    <div class="error-message" role="alert">{error}</div>
  {/if}

  <div class="add-profile-field">
    <div class="input-with-button">
      <svg class="field-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
      <input
        type="text"
        bind:value={newUrl}
        placeholder="Enter profile URL or username"
        onkeydown={(e) => e.key === 'Enter' && addProfile()}
        autocomplete="off"
        spellcheck="false"
        aria-label="Profile URL or username"
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
            <div class="avatar" aria-hidden="true">{initials(profile)}</div>
            <div>
              <p class="profile-name">{profile.username || profile.url}</p>
              {#if profile.username}
                <p class="profile-url">{profile.url}</p>
              {/if}
            </div>
          </div>
          <button class="delete-btn" onclick={() => promptDelete(profile.url)} aria-label="Delete profile {profile.username || profile.url}">
            Delete
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if confirmDeleteUrl}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="confirm-overlay"
    role="presentation"
    onclick={cancelDelete}
    onkeydown={onDialogKeydown}
  >
    <div
      class="confirm-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      tabindex="-1"
      bind:this={dialogEl}
      onclick={(e) => e.stopPropagation()}
      onkeydown={onDialogKeydown}
    >
      <p class="confirm-title" id="confirm-title">Delete this profile?</p>
      <p class="confirm-url">{confirmDeleteUrl}</p>
      <div class="confirm-actions">
        <button class="confirm-no" onclick={cancelDelete}>Cancel</button>
        <button class="confirm-yes" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Reads --glass-*, --accent-* custom properties inherited from the
     page shell (src/routes/open-omni/+page.svelte). */
  .profile-manager {
    max-width: 440px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .pill-group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .pill-group button {
    padding: 8px 16px;
    border-radius: 999px;
    border: 1px solid var(--glass-border);
    background: var(--glass-surface-strong);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .pill-group button:hover:not(.active) {
    border-color: var(--accent-line);
  }

  .pill-group button.active {
    background: var(--accent-gradient);
    border-color: transparent;
    color: var(--on-accent);
    font-weight: 500;
    box-shadow: 0 3px 12px -4px var(--accent-glow);
  }

  .error-message {
    padding: 10px 14px;
    background: var(--error);
    color: var(--on-error);
    border-radius: var(--glass-radius-sm, 10px);
    font-size: 13px;
    text-align: center;
  }

  .input-with-button {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius, 14px);
    padding: 0 8px 0 14px;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .input-with-button:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .field-icon {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .input-with-button input {
    flex: 1;
    padding: 12px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
  }

  .input-with-button input:focus {
    outline: none;
  }

  .add-btn {
    padding: 8px 18px;
    border-radius: 999px;
    border: none;
    background: var(--accent-gradient);
    color: var(--on-accent);
    font-weight: 500;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    box-shadow: 0 3px 12px -4px var(--accent-glow);
  }

  .add-btn:hover:not(:disabled) {
    box-shadow: 0 4px 16px -4px var(--accent-glow);
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
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
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius, 14px);
    transition: border-color 0.15s ease;
  }

  .profile-item:hover {
    border-color: var(--accent-line);
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
    background: var(--accent-gradient);
    color: var(--on-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 500;
    font-size: 13px;
    flex-shrink: 0;
    box-shadow: 0 3px 10px -3px var(--accent-glow);
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
    border-radius: 999px;
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
    padding: 20px 0;
    font-size: 13px;
    margin: 0;
  }

  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.4));
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .confirm-dialog {
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius-lg, 20px);
    padding: 24px;
    max-width: 380px;
    width: 90%;
    text-align: center;
    backdrop-filter: blur(24px) saturate(140%);
    -webkit-backdrop-filter: blur(24px) saturate(140%);
    box-shadow: 0 18px 40px -20px rgba(0, 0, 0, 0.35);
    outline: none;
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
    border-radius: 999px;
    font-weight: 500;
    font-size: 13px;
    cursor: pointer;
  }

  .confirm-no {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--glass-border);
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
