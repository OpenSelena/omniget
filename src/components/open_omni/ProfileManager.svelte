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

<div class="pm">
  <div class="pm-segmented" role="tablist" aria-label="Platform">
    {#each platforms as platform}
      <button
        type="button"
        role="tab"
        aria-selected={activePlatform === platform.key}
        class:on={activePlatform === platform.key}
        onclick={() => (activePlatform = platform.key)}
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
    <p class="pm-empty">Loading profiles…</p>
  {:else if profiles.length === 0}
    <p class="pm-empty">No profiles added yet</p>
  {:else}
    <div class="pm-group">
      {#each profiles as profile}
        <div class="pm-row pm-profile-row">
          <div class="pm-avatar" aria-hidden="true">{initials(profile)}</div>
          <div class="pm-info">
            <p class="pm-name">{profile.username || profile.url}</p>
            {#if profile.username}
              <p class="pm-url">{profile.url}</p>
            {/if}
          </div>
          <button
            class="pm-remove"
            onclick={() => promptDelete(profile.url)}
            aria-label="Remove profile {profile.username || profile.url}"
          >
            Remove
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if confirmDeleteUrl}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="pm-overlay"
    role="presentation"
    onclick={cancelDelete}
    onkeydown={onDialogKeydown}
  >
    <div
      class="pm-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="pm-confirm-title"
      tabindex="-1"
      bind:this={dialogEl}
      onclick={(e) => e.stopPropagation()}
      onkeydown={onDialogKeydown}
    >
      <p class="pm-dialog-title" id="pm-confirm-title">Delete this profile?</p>
      <p class="pm-dialog-url">{confirmDeleteUrl}</p>
      <div class="pm-dialog-actions">
        <button class="pm-dialog-btn" onclick={cancelDelete}>Cancel</button>
        <button class="pm-dialog-btn danger" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Reads --oo-accent / --oo-border / --oo-group-bg / --oo-text* custom
     properties inherited from the shell in +page.svelte. */
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
    padding: 6px 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--oo-text-secondary);
    border-radius: 7px;
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
    box-shadow: 0 0 0 1px var(--oo-border);
    overflow: hidden;
  }

  .pm-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--oo-border);
  }
  .pm-row:last-child { border-bottom: 0; }

  .pm-icon { width: 15px; height: 15px; color: var(--oo-text-secondary); flex-shrink: 0; }

  .pm-row input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--oo-text);
    font-size: 13px;
    min-width: 0;
  }
  .pm-row input:focus { outline: none; }

  .pm-add-btn {
    padding: 5px 13px;
    border-radius: 999px;
    background: var(--oo-accent);
    color: #fff;
    font-size: 12px;
    font-weight: 500;
  }
  .pm-add-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .pm-profile-row { gap: 10px; }

  .pm-avatar {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--oo-accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .pm-info { min-width: 0; flex: 1; }
  .pm-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--oo-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pm-url {
    font-size: 11px;
    color: var(--oo-text-secondary);
    margin-top: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pm-remove {
    font-size: 12px;
    font-weight: 500;
    color: var(--error, #ff453a);
    flex-shrink: 0;
  }
  .pm-remove:hover { opacity: 0.75; }

  .pm-empty {
    text-align: center;
    color: var(--oo-text-secondary);
    padding: 18px 0;
    font-size: 12.5px;
  }

  .pm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .pm-dialog {
    background: var(--oo-group-bg);
    border-radius: 12px;
    padding: 20px;
    max-width: 340px;
    width: 90%;
    text-align: center;
    box-shadow: 0 18px 40px -20px rgba(0, 0, 0, 0.4);
    outline: none;
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
    word-break: break-all;
    margin: 0 0 16px;
  }
  .pm-dialog-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }
  .pm-dialog-btn {
    flex: 1;
    padding: 8px 10px;
    border-radius: 7px;
    font-weight: 500;
    font-size: 12.5px;
    background: color-mix(in srgb, var(--oo-text) 6%, transparent);
    color: var(--oo-text);
  }
  .pm-dialog-btn.danger {
    background: var(--error, #ff453a);
    color: #fff;
  }
  .pm-dialog-btn.danger:hover { opacity: 0.9; }
</style>
