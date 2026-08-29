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

  // Ring geometry lives in one place (r=44) so the SVG markup below and this
  // circumference constant can never drift out of sync.
  const RING_RADIUS = 44;
  const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

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

<div class="dl">
  {#if depsChecked && depsStatus}
    <div class="dl-alert error" role="alert">
      <span>{depsStatus}</span>
    </div>
  {/if}

  {#if status}
    <div
      class="dl-alert"
      class:success={statusType === 'success'}
      class:error={statusType === 'error'}
      class:info={statusType === 'info'}
      role="status"
      aria-live="polite"
    >
      <span>{status}</span>
      <button class="dl-alert-close" onclick={clearStatus} aria-label="Dismiss">×</button>
    </div>
  {/if}

  {#if !url.trim() && savedProfiles.length > 0}
    <div class="dl-quicklist" role="listbox" aria-label="Saved profiles">
      {#each savedProfiles as profile, index}
        <button
          type="button"
          class="dl-chip"
          class:on={selectedProfileIndex === index}
          onclick={() => selectProfile(index)}
          disabled={downloading}
          role="option"
          aria-selected={selectedProfileIndex === index}
        >
          <span>{profile.username || profile.url}</span>
          {#if profile._platformLabel}
            <span class="dl-chip-sub">{profile._platformLabel}</span>
          {/if}
        </button>
      {/each}
    </div>
  {:else if !url.trim() && savedProfiles.length === 0}
    <p class="dl-hint">
      No saved profiles —
      <button type="button" class="dl-link" onclick={() => dispatch('switchToProfiles')}>add one</button>
    </p>
  {/if}

  <div class="dl-group">
    <div class="dl-row">
      <svg class="dl-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" aria-hidden="true">
        <path d="M5.5 10.5c-.8.8-2.2.8-3 0-.8-.8-.8-2.2 0-3L4 6" />
        <path d="M10.5 5.5c.8-.8 2.2-.8 3 0 .8.8.8 2.2 0 3L12 10" />
        <path d="M6.3 9.7 9.7 6.3" />
      </svg>
      <input
        id="profile-url"
        aria-label="Profile URL or handle"
        type="text"
        bind:value={url}
        placeholder="https://instagram.com/username"
        disabled={downloading}
        oninput={clearSelectedProfile}
        autocomplete="off"
        spellcheck="false"
      />
    </div>
  </div>

  <div class="dl-segmented" role="group" aria-label="Content type">
    {#each CONTENT_TYPES as type}
      <button
        type="button"
        class:on={contentType === type.key}
        onclick={() => (contentType = type.key)}
        disabled={downloading || ((type.key === 'stories' || type.key === 'highlights') && !isInstagram)}
        title={(type.key === 'stories' || type.key === 'highlights') && !isInstagram ? 'Instagram only' : ''}
        aria-pressed={contentType === type.key}
      >
        {type.label}
      </button>
    {/each}
  </div>

  {#if missingOutputDir}
    <p class="dl-warning" role="alert">
      Set an output directory in
      <button type="button" class="dl-link" onclick={() => dispatch('switchToSettings')}>Settings</button>
      before downloading.
    </p>
  {/if}
  {#if needsCookiesWarning}
    <p class="dl-warning" role="alert">
      Stories and Highlights need a cookies file — add one in
      <button type="button" class="dl-link" onclick={() => dispatch('switchToSettings')}>Settings</button>.
    </p>
  {/if}
  {#if platformMismatch}
    <p class="dl-warning" role="alert">Stories and Highlights are only supported for Instagram.</p>
  {/if}

  <div class="dl-actions">
    {#if downloading}
      <button class="dl-btn stop" onclick={handleStop} disabled={cancelling}>
        {cancelling ? 'Cancelling…' : 'Stop Download'}
      </button>
    {:else}
      <button
        class="dl-btn primary"
        onclick={startDownload}
        disabled={!getDownloadUrl() || missingOutputDir || needsCookiesWarning || platformMismatch}
      >
        Start Download
      </button>
    {/if}
  </div>

  {#if downloading}
    <div class="dl-progress" role="status" aria-live="polite" aria-atomic="true">
      <svg class="dl-ring" class:indeterminate={ringFraction === null} viewBox="0 0 104 104" width="104" height="104" aria-hidden="true">
        <circle class="dl-ring-track" cx="52" cy="52" r={RING_RADIUS} />
        {#if ringFraction === null}
          <circle class="dl-ring-progress" cx="52" cy="52" r={RING_RADIUS} />
        {:else}
          <circle
            class="dl-ring-progress"
            cx="52" cy="52" r={RING_RADIUS}
            stroke-dasharray={RING_CIRCUMFERENCE}
            stroke-dashoffset={ringDashoffset}
          />
        {/if}
        <text x="52" y="49" text-anchor="middle" class="dl-ring-count">{filesDownloaded}</text>
        <text x="52" y="66" text-anchor="middle" class="dl-ring-label">files</text>
      </svg>
      <p class="dl-progress-stage">
        {#if stageTotal && stageTotal > 1}
          Stage {stageIndex ?? 1} of {stageTotal} · {stage ?? ''}
        {:else}
          Downloading{stage ? ` ${stage}` : ''}…
        {/if}
      </p>
      {#if lastMessage}
        <p class="dl-progress-msg" title={lastMessage}>{lastMessage}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* Reads --oo-accent / --oo-border / --oo-group-bg / --oo-text* custom
     properties inherited from the shell in +page.svelte. */
  .dl {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .dl-alert {
    padding: 9px 12px;
    border-radius: var(--oo-radius, 9px);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
  }
  .dl-alert.success { background: var(--success); color: var(--on-success); }
  .dl-alert.error { background: var(--error); color: var(--on-error); }
  .dl-alert.info {
    background: color-mix(in srgb, var(--oo-accent) 12%, transparent);
    color: var(--oo-accent);
  }
  .dl-alert-close {
    font-size: 16px;
    line-height: 1;
    opacity: 0.7;
  }

  .dl-quicklist {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .dl-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: 999px;
    font-size: 11.5px;
    background: var(--oo-group-bg);
    box-shadow: 0 0 0 1px var(--oo-border);
    color: var(--oo-text);
  }
  .dl-chip:disabled { opacity: 0.5; cursor: not-allowed; }
  .dl-chip.on { background: var(--oo-accent); color: #fff; box-shadow: none; font-weight: 500; }
  .dl-chip-sub { font-size: 10px; opacity: 0.75; }

  .dl-hint {
    font-size: 12px;
    color: var(--oo-text-secondary);
  }
  .dl-link {
    color: var(--oo-accent);
    text-decoration: underline;
    font-size: inherit;
  }
  .dl-link:hover { opacity: 0.8; }

  .dl-group {
    background: var(--oo-group-bg);
    border-radius: var(--oo-radius, 9px);
    box-shadow: 0 0 0 1px var(--oo-border);
    overflow: hidden;
  }
  .dl-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 12px;
  }
  .dl-icon {
    width: 15px;
    height: 15px;
    color: var(--oo-text-secondary);
    flex-shrink: 0;
  }
  .dl-row input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--oo-text);
    font-size: 13px;
    min-width: 0;
  }
  .dl-row input:focus { outline: none; }
  .dl-row input:disabled { opacity: 0.6; cursor: not-allowed; }

  .dl-segmented {
    display: flex;
    background: color-mix(in srgb, var(--oo-text) 6%, transparent);
    border-radius: var(--oo-radius, 9px);
    padding: 2px;
    gap: 1px;
  }
  .dl-segmented button {
    flex: 1;
    padding: 6px 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--oo-text-secondary);
    border-radius: 7px;
  }
  .dl-segmented button.on {
    background: var(--oo-group-bg);
    color: var(--oo-text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }
  .dl-segmented button:disabled { opacity: 0.4; cursor: not-allowed; }
  .dl-segmented button:disabled.on { background: transparent; box-shadow: none; }

  .dl-warning {
    font-size: 12px;
    color: var(--error, #d9362c);
    margin: -4px 0 0;
  }

  .dl-actions {
    display: flex;
    justify-content: center;
  }
  .dl-btn {
    padding: 8px 26px;
    border-radius: var(--oo-radius, 9px);
    font-weight: 600;
    font-size: 13px;
    background: var(--oo-accent);
    color: #fff;
  }
  .dl-btn:hover:not(:disabled) { filter: brightness(1.06); }
  .dl-btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .dl-btn.stop { background: var(--error, #ff453a); }

  .dl-progress {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 2px;
  }
  .dl-ring { color: var(--oo-accent); }
  .dl-ring-track {
    fill: none;
    stroke: color-mix(in srgb, var(--oo-text) 8%, transparent);
    stroke-width: 8;
  }
  .dl-ring-progress {
    fill: none;
    stroke: currentColor;
    stroke-width: 8;
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: 52px 52px;
    transition: stroke-dashoffset 0.35s ease;
  }
  .dl-ring.indeterminate .dl-ring-progress {
    stroke-dasharray: 70 207;
    transition: none;
    animation: dl-ring-spin 1.1s linear infinite;
  }
  @keyframes dl-ring-spin {
    from { transform: rotate(-90deg); }
    to { transform: rotate(270deg); }
  }
  .dl-ring-count { font-size: 17px; font-weight: 700; fill: var(--oo-text); }
  .dl-ring-label { font-size: 9.5px; fill: var(--oo-text-secondary); }

  .dl-progress-stage {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--oo-text);
    margin: 10px 0 2px;
    text-align: center;
  }
  .dl-progress-msg {
    font-size: 11.5px;
    color: var(--oo-text-secondary);
    margin: 0;
    max-width: 280px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
  }
</style>
