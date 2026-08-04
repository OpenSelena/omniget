<script lang="ts">
  import { checkPythonDependencies, loadProfiles } from '$lib/api/open_omni';
  import {
    isActive,
    isCancelling,
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
  let depsChecked = $state(false);

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
  let cancelling = $derived(isCancelling());
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
    // Dependency check runs once on mount, not on every reactive tick.
    await checkDeps();
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
    // is ever shown to the user.
    try {
      const result = await checkPythonDependencies();
      if (!result.includes('OK')) {
        depsStatus = result;
      } else {
        depsStatus = '';
      }
    } catch (error) {
      depsStatus = `Missing dependency: ${error}`;
    } finally {
      depsChecked = true;
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

  // When the user types a URL, clear any chip selection.
  $effect(() => {
    if (url.trim()) {
      selectedProfileIndex = -1;
    }
  });

  // Force content type away from Instagram-only options when the current
  // target is not Instagram. Guard against unnecessary writes.
  $effect(() => {
    if (
      (contentType === 'stories' || contentType === 'highlights') &&
      !isInstagram
    ) {
      contentType = 'photos';
    }
  });

  const CONTENT_TYPES: Array<{ key: string; label: string }> = [
    { key: 'all', label: 'All' },
    { key: 'photos', label: 'Photos' },
    { key: 'videos', label: 'Videos' },
    { key: 'stories', label: 'Stories' },
    { key: 'highlights', label: 'Highlights' },
  ];
</script>

<div class="download-manager">
  {#if depsChecked && depsStatus}
    <div class="deps-check error" role="alert">
      <span>{depsStatus}</span>
    </div>
  {/if}

  {#if status}
    <div
      class="status-alert"
      class:success={statusType === 'success'}
      class:error={statusType === 'error'}
      class:info={statusType === 'info'}
      role="status"
      aria-live="polite"
    >
      <span>{status}</span>
      <button class="close-alert" onclick={clearStatus} aria-label="Dismiss">×</button>
    </div>
  {/if}

  <div class="url-field">
    <label for="profile-url">Profile URL or handle</label>
    <div class="url-input-shell">
      <svg class="url-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
      <input
        id="profile-url"
        type="text"
        bind:value={url}
        placeholder="https://instagram.com/username"
        disabled={downloading}
        oninput={clearSelectedProfile}
        autocomplete="off"
        spellcheck="false"
      />
    </div>
    {#if !url.trim() && savedProfiles.length > 0}
      <div class="chip-list" role="listbox" aria-label="Saved profiles">
        {#each savedProfiles as profile, index}
          <button
            type="button"
            class="chip"
            class:selected={selectedProfileIndex === index}
            onclick={() => selectProfile(index)}
            disabled={downloading}
            role="option"
            aria-selected={selectedProfileIndex === index}
          >
            <span>{profile.username || profile.url}</span>
            {#if profile._platformLabel}
              <span class="chip-platform">{profile._platformLabel}</span>
            {/if}
            {#if selectedProfileIndex === index}
              <span class="chip-check" aria-hidden="true">✓</span>
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
    <span class="segment-label" id="content-type-label">Content type</span>
    <div class="pill-group" role="group" aria-labelledby="content-type-label">
      {#each CONTENT_TYPES as type}
        <button
          type="button"
          class:active={contentType === type.key}
          onclick={() => (contentType = type.key)}
          disabled={downloading || ((type.key === 'stories' || type.key === 'highlights') && !isInstagram)}
          title={(type.key === 'stories' || type.key === 'highlights') && !isInstagram ? 'Instagram only' : ''}
          aria-pressed={contentType === type.key}
        >
          {type.label}
        </button>
      {/each}
    </div>
  </div>

  {#if missingOutputDir}
    <p class="field-warning" role="alert">
      Set an output directory in
      <button type="button" class="inline-link" onclick={() => dispatch('switchToSettings')}>Settings</button>
      before downloading.
    </p>
  {/if}
  {#if needsCookiesWarning}
    <p class="field-warning" role="alert">
      Stories and Highlights need a cookies file — add one in
      <button type="button" class="inline-link" onclick={() => dispatch('switchToSettings')}>Settings</button>.
    </p>
  {/if}
  {#if platformMismatch}
    <p class="field-warning" role="alert">Stories and Highlights are only supported for Instagram.</p>
  {/if}

  <div class="button-group">
    {#if downloading}
      <button class="primary-btn stop-btn" onclick={handleStop} disabled={cancelling}>
        {cancelling ? 'Cancelling…' : 'Stop download'}
      </button>
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
    <div class="ring-wrap" role="status" aria-live="polite" aria-atomic="true">
      <svg class="om-ring" class:indeterminate={ringFraction === null} viewBox="0 0 140 140" width="140" height="140" aria-hidden="true">
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
          Stage {stageIndex ?? 1} of {stageTotal} · {stage ?? ''}
        {:else}
          Downloading{stage ? ` ${stage}` : ''}…
        {/if}
      </p>
      {#if lastMessage}
        <p class="ring-message" title={lastMessage}>{lastMessage}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  /*
    Reads --glass-*, --accent-* custom properties inherited from the
    page shell (src/routes/open-omni/+page.svelte). No local card
    wrappers — the parent glass panel is the only surface; this
    component only lays out content inside it.
  */
  .download-manager {
    max-width: 440px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .deps-check {
    padding: 10px 14px;
    border-radius: var(--glass-radius-sm, 10px);
    font-size: 13px;
    font-weight: 500;
    text-align: center;
  }

  .deps-check.error {
    background: var(--error);
    color: var(--on-error);
  }

  .status-alert {
    padding: 10px 14px;
    border-radius: var(--glass-radius-sm, 10px);
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

  .status-alert.info {
    background: var(--glass-surface-strong);
    color: var(--text);
    border: 1px solid var(--glass-border);
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

  .url-field label {
    display: block;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .url-input-shell {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius, 14px);
    padding: 0 14px;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .url-input-shell:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .url-icon {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .url-input-shell input {
    flex: 1;
    padding: 12px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
  }

  .url-input-shell input:focus {
    outline: none;
  }

  .url-input-shell input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hint-muted {
    font-size: 12.5px;
    color: var(--text-secondary);
    font-style: italic;
    margin: 12px 0 0;
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
    gap: 8px;
    margin-top: 12px;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 999px;
    border: 1px solid var(--glass-border);
    background: var(--glass-surface-strong);
    color: var(--text);
    font-size: 12.5px;
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .chip:hover:not(:disabled) {
    border-color: var(--accent-line);
  }

  .chip.selected {
    background: var(--accent-gradient);
    border-color: transparent;
    color: var(--on-accent);
    font-weight: 500;
    box-shadow: 0 3px 12px -4px var(--accent-glow);
  }

  .chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .chip-platform {
    font-size: 10.5px;
    opacity: 0.75;
  }

  .chip.selected .chip-platform {
    opacity: 0.9;
  }

  .chip-check {
    font-weight: bold;
  }

  .segment-label {
    display: block;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 10px;
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

  .pill-group button:hover:not(:disabled):not(.active) {
    border-color: var(--accent-line);
  }

  .pill-group button.active {
    background: var(--accent-gradient);
    border-color: transparent;
    color: var(--on-accent);
    box-shadow: 0 3px 12px -4px var(--accent-glow);
  }

  .pill-group button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .field-warning {
    font-size: 12px;
    color: var(--error);
    margin: -0.75rem 0 0;
  }

  .primary-btn {
    width: 100%;
    padding: 13px;
    border: none;
    border-radius: var(--glass-radius, 14px);
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    background: var(--accent-gradient);
    color: var(--on-accent);
    box-shadow: 0 6px 20px -6px var(--accent-glow);
    transition: transform 0.12s ease, box-shadow 0.12s ease, opacity 0.12s ease;
  }

  .primary-btn:hover:not(:disabled) {
    box-shadow: 0 8px 24px -6px var(--accent-glow);
  }

  .primary-btn:active:not(:disabled) {
    transform: scale(0.99);
  }

  .primary-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    box-shadow: none;
  }

  .stop-btn {
    background: var(--error);
    color: var(--on-error);
    box-shadow: 0 6px 20px -6px color-mix(in srgb, var(--error) 45%, transparent);
  }

  .ring-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 4px;
  }

  .om-ring-track {
    fill: none;
    stroke: var(--glass-surface-strong);
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
    filter: drop-shadow(0 0 6px var(--accent-glow));
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


