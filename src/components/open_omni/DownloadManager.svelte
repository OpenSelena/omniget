<script lang="ts">
  import { checkPythonDependencies, loadProfiles } from '$lib/api/open_omni';
  import {
    isActive,
    getFilesDownloaded,
    getStage,
    getStageIndex,
    getStageTotal,
    getLastMessage,
    getStatus,
    getStatusType,
    clearStatus as clearStoreStatus,
    startDownload as startStoreDownload,
    stopDownload as stopStoreDownload,
    reattachIfActive,
  } from '$lib/stores/open_omni_download_store.svelte';
  import {
    getOutputDir,
    getCookiesFile,
    loadSettings,
  } from '$lib/stores/open_omni_settings_store.svelte';
  import { createEventDispatcher, onMount } from 'svelte';

  const dispatch = createEventDispatcher();

  const RING_CIRCUMFERENCE = 2 * Math.PI * 58;

  let url = $state('');
  let contentType = $state('all');
  let depsStatus = $state('');

  function detectPlatform(u: string): string {
    const low = u.toLowerCase();
    if (low.includes('instagram.com')) return 'Instagram';
    if (low.includes('tiktok.com')) return 'TikTok';
    if (low.includes('facebook.com')) return 'Facebook';
    if (low.includes('x.com') || low.includes('twitter.com')) return 'X';
    return 'Other';
  }

  let savedProfiles = $state<Array<{ url: string; username?: string; _platformLabel?: string }>>([]);
  let selectedProfileIndex = $state(-1);

  let downloading = $derived(isActive());
  let filesDownloaded = $derived(getFilesDownloaded());
  let stage = $derived(getStage());
  let stageIndex = $derived(getStageIndex());
  let stageTotal = $derived(getStageTotal());
  let lastMessage = $derived(getLastMessage());
  let status = $derived(getStatus());
  let statusType = $derived(getStatusType());

  let outputDir = $derived(getOutputDir());
  let cookiesFile = $derived(getCookiesFile());

  let missingOutputDir = $derived(!outputDir.trim());
  let needsCookiesWarning = $derived(
    (contentType === 'stories' || contentType === 'highlights') && !cookiesFile.trim()
  );

  let currentPlatform = $derived(detectPlatform(getDownloadUrl() ?? ''));
  let isInstagram = $derived(currentPlatform === 'Instagram');

  let platformMismatch = $derived(
    (contentType === 'stories' || contentType === 'highlights') && !isInstagram
  );

  let ringFraction = $derived.by(() => {
    if (!stageTotal || stageTotal <= 1) return null;
    const idx = stageIndex ?? 1;
    const raw = (idx - 0.5) / stageTotal;
    return Math.min(1, Math.max(0, raw));
  });

  let ringDashoffset = $derived(
    ringFraction === null ? 0 : RING_CIRCUMFERENCE * (1 - ringFraction)
  );

  onMount(async () => {
    reattachIfActive();
    await loadSettings();
    await loadSavedProfiles();
  });

  async function loadSavedProfiles() {
    try {
      const platformLabels: Record<string, string> = {
        instagram: 'Instagram',
        tiktok: 'TikTok',
        facebook: 'Facebook',
        x: 'X',
      };
      const results = await Promise.all(
        Object.keys(platformLabels).map(async (platform) => {
          const profiles = await loadProfiles(platform);
          return profiles.map((p) => ({ ...p, _platformLabel: platformLabels[platform] }));
        })
      );
      savedProfiles = results.flat();

      if (savedProfiles.length > 0 && selectedProfileIndex === -1 && !url.trim()) {
        selectedProfileIndex = 0;
      }
    } catch (error) {
      console.error('Failed to load profiles:', error);
    }
  }

  async function checkDeps() {
    // Only surface this banner when a dependency is actually missing.
    // When everything is installed, leave depsStatus empty so nothing
    // is ever shown to the user — no "All dependencies OK" flash.
    try {
      const result = await checkPythonDependencies();
      if (!result.includes('OK')) {
        depsStatus = result;
      } else {
        depsStatus = '';
      }
    } catch (error) {
      depsStatus = `Missing dependency: ${error}`;
    }
  }

  function getDownloadUrl(): string | null {
    if (url.trim()) {
      return url.trim();
    }
    if (selectedProfileIndex >= 0 && savedProfiles[selectedProfileIndex]) {
      return savedProfiles[selectedProfileIndex].url;
    }
    return null;
  }

  async function startDownload() {
    const downloadUrl = getDownloadUrl();

    if (!downloadUrl || missingOutputDir) {
      clearStoreStatus();
      return;
    }

    try {
      await startStoreDownload(downloadUrl, outputDir, cookiesFile, contentType);
    } catch (error) {
      console.error('Failed to start download:', error);
      return;
    }

    if (getStatusType() === 'success') {
      dispatch('downloadComplete', {
        url: downloadUrl,
        contentType,
      });
    }
  }

  async function handleStop() {
    await stopStoreDownload();
  }

  function clearStatus() {
    clearStoreStatus();
  }

  function selectProfile(index: number) {
    selectedProfileIndex = index;
    if (index >= 0) {
      url = '';
    }
  }

  function clearSelectedProfile() {
    selectedProfileIndex = -1;
  }

  $effect(() => {
    checkDeps();
  });

  $effect(() => {
    if (url.trim()) {
      selectedProfileIndex = -1;
    }
  });

  $effect(() => {
    if ((contentType === 'stories' || contentType === 'highlights') && !isInstagram) {
      contentType = 'photos';
    }
  });
</script>

<div class="download-manager">
  {#if depsStatus}
    <div class="deps-check" class:ok={depsStatus.includes('OK')} class:error={!depsStatus.includes('OK')}>
      <span>{depsStatus}</span>
    </div>
  {/if}

  {#if status}
    <div class="status-alert" class:success={statusType === 'success'} class:error={statusType === 'error'}>
      <span>{status}</span>
      <button class="close-alert" onclick={clearStatus} aria-label="Dismiss">×</button>
    </div>
  {/if}

  <div class="field-card">
    <label for="profile-url">Profile URL or handle</label>
    <input
      id="profile-url"
      type="text"
      bind:value={url}
      placeholder="https://instagram.com/username"
      disabled={downloading}
      oninput={clearSelectedProfile}
    />
    {#if !url.trim() && savedProfiles.length > 0}
      <div class="chip-list">
        {#each savedProfiles as profile, index}
          <button
            type="button"
            class="chip"
            class:selected={selectedProfileIndex === index}
            onclick={() => selectProfile(index)}
            disabled={downloading}
          >
            <span>{profile.username || profile.url}</span>
            {#if selectedProfileIndex === index}
              <span class="chip-check">✓</span>
            {/if}
          </button>
        {/each}
      </div>
    {:else if !url.trim() && savedProfiles.length === 0}
      <p class="hint-muted">
        No saved profiles —
        <button type="button" class="inline-link" onclick={() => dispatch('switchToProfiles')}>add one</button>
      </p>
    {/if}
  </div>

  <div class="segment-block">
    <span class="segment-label">Content type</span>
    <div class="pill-group">
      <button type="button" class:active={contentType === 'all'} onclick={() => contentType = 'all'} disabled={downloading}>All</button>
      <button type="button" class:active={contentType === 'photos'} onclick={() => contentType = 'photos'} disabled={downloading}>Photos</button>
      <button type="button" class:active={contentType === 'videos'} onclick={() => contentType = 'videos'} disabled={downloading}>Videos</button>
      <button type="button" class:active={contentType === 'stories'} onclick={() => contentType = 'stories'} disabled={downloading || !isInstagram} title={isInstagram ? '' : 'Instagram only'}>Stories</button>
      <button type="button" class:active={contentType === 'highlights'} onclick={() => contentType = 'highlights'} disabled={downloading || !isInstagram} title={isInstagram ? '' : 'Instagram only'}>Highlights</button>
    </div>
  </div>

  {#if missingOutputDir}
    <p class="field-warning">
      Set an output directory in
      <button type="button" class="inline-link" onclick={() => dispatch('switchToSettings')}>Settings</button>
      before downloading.
    </p>
  {/if}
  {#if needsCookiesWarning}
    <p class="field-warning">
      Stories and Highlights need a cookies file — add one in
      <button type="button" class="inline-link" onclick={() => dispatch('switchToSettings')}>Settings</button>.
    </p>
  {/if}
  {#if platformMismatch}
    <p class="field-warning">Stories and Highlights are only supported for Instagram.</p>
  {/if}

  <div class="button-group">
    {#if downloading}
      <button class="primary-btn stop-btn" onclick={handleStop}>Stop download</button>
    {:else}
      <button
        class="primary-btn"
        onclick={startDownload}
        disabled={!getDownloadUrl() || missingOutputDir || needsCookiesWarning || platformMismatch}
      >
        Start download
      </button>
    {/if}
  </div>

  {#if downloading}
    <div class="ring-wrap">
      <svg class="om-ring" class:indeterminate={ringFraction === null} viewBox="0 0 140 140" width="140" height="140">
        <circle class="om-ring-track" cx="70" cy="70" r="58" />
        {#if ringFraction === null}
          <circle class="om-ring-progress" cx="70" cy="70" r="58" />
        {:else}
          <circle
            class="om-ring-progress"
            cx="70" cy="70" r="58"
            stroke-dasharray={RING_CIRCUMFERENCE}
            stroke-dashoffset={ringDashoffset}
          />
        {/if}
        <text x="70" y="65" text-anchor="middle" class="om-ring-count">{filesDownloaded}</text>
        <text x="70" y="84" text-anchor="middle" class="om-ring-label">files</text>
      </svg>
      <p class="ring-stage">
        {#if stageTotal && stageTotal > 1}
          Stage {stageIndex ?? 1} of {stageTotal} &middot; {stage ?? ''}
        {:else}
          Downloading{stage ? ` ${stage}` : ''}&hellip;
        {/if}
      </p>
      {#if lastMessage}
        <p class="ring-message">{lastMessage}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* Centered column instead of left-hugging the container. */
  .download-manager {
    padding: 0;
    max-width: 460px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }

  .deps-check {
    padding: 10px 14px;
    margin-bottom: 1.25rem;
    border-radius: var(--border-radius);
    font-size: 13px;
    font-weight: 500;
    text-align: center;
  }

  .deps-check.ok {
    background: var(--button-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--content-border);
  }

  .deps-check.error {
    background: var(--error);
    color: var(--on-error);
  }

  .status-alert {
    padding: 10px 14px;
    margin-bottom: 1.25rem;
    border-radius: var(--border-radius);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }

  .status-alert.success {
    background: var(--success);
    color: var(--on-success);
  }

  .status-alert.error {
    background: var(--error);
    color: var(--on-error);
  }

  .close-alert {
    background: none;
    border: none;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    opacity: 0.8;
    padding: 0;
    color: inherit;
  }

  .field-card {
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-radius: 10px;
    padding: 16px 18px;
    margin-bottom: 20px;
  }

  .field-card label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 8px;
    text-align: center;
  }

  .field-card input {
    width: 100%;
    padding: 10px 14px;
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: 8px;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
    text-align: center;
  }

  .field-card input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .field-card input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hint-muted {
    font-size: 12.5px;
    color: var(--text-secondary);
    font-style: italic;
    margin: 14px 0 0;
    text-align: center;
  }

  .inline-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    font-size: inherit;
    font-style: normal;
    cursor: pointer;
    text-decoration: underline;
  }

  .inline-link:hover {
    opacity: 0.8;
  }

  .chip-list {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 14px;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 14px;
    border-radius: 16px;
    border: 1px solid var(--input-border);
    background: var(--button-elevated);
    color: var(--text);
    font-size: 12.5px;
    cursor: pointer;
  }

  .chip:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .chip.selected {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 500;
  }

  .chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .chip-check {
    font-weight: bold;
  }

  .segment-block {
    margin-bottom: 20px;
    text-align: center;
  }

  .segment-label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 10px;
  }

  .pill-group {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
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

  .pill-group button:hover:not(:disabled):not(.active) {
    border-color: var(--accent);
  }

  .pill-group button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .pill-group button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .field-warning {
    font-size: 12px;
    color: var(--error);
    margin: 8px 0 0;
    text-align: center;
  }

  .button-group {
    margin: 20px 0 4px;
  }

  .primary-btn {
    width: 100%;
    padding: 12px;
    border: none;
    border-radius: var(--border-radius);
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    background: var(--accent);
    color: var(--on-accent);
  }

  .primary-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .stop-btn {
    background: var(--error);
    color: var(--on-error);
  }

  .ring-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 22px 0 6px;
  }

  .om-ring-track {
    fill: none;
    stroke: var(--button-elevated);
    stroke-width: 12;
  }

  .om-ring-progress {
    fill: none;
    stroke: var(--accent);
    stroke-width: 12;
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: 70px 70px;
    transition: stroke-dashoffset 0.4s ease;
  }

  .om-ring.indeterminate .om-ring-progress {
    stroke-dasharray: 90 274.42;
    transition: none;
    animation: om-ring-spin 1.1s linear infinite;
  }

  @keyframes om-ring-spin {
    from { transform: rotate(-90deg); }
    to { transform: rotate(270deg); }
  }

  .om-ring-count {
    font-size: 22px;
    font-weight: 500;
    fill: var(--text);
  }

  .om-ring-label {
    font-size: 11px;
    fill: var(--text-secondary);
  }

  .ring-stage {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin: 10px 0 2px;
    text-align: center;
  }

  .ring-message {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
    max-width: 280px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
  }
</style>
