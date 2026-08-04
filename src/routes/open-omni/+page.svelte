<script lang="ts">
  import DownloadManager from '$components/open_omni/DownloadManager.svelte';
  import ProfileManager from '$components/open_omni/ProfileManager.svelte';
  import SettingsManager from '$components/open_omni/SettingsManager.svelte';

  let activeTab = $state('download');

  const tabs = [
    {
      key: 'download',
      label: 'Download',
      icon: 'M12 3v12 M7 10 5 5 5-5 M5 21h14'
    },
    { key: 'profiles', label: 'Profiles' },
    { key: 'settings', label: 'Settings' }
  ];
</script>

<svelte:head>
  <title>Open Omni - Social Media Downloader</title>
</svelte:head>

<div class="open-omni-page">
  <div class="ambient-wash" aria-hidden="true"></div>

  <header class="page-header">
    <h1>Open Omni</h1>
    <p class="subtitle">Social media profile &amp; batch downloader</p>
  </header>

  <div class="glass-panel">
    <nav class="tab-switcher" aria-label="Open Omni sections">
      <button
        type="button"
        class:active={activeTab === 'download'}
        onclick={() => (activeTab = 'download')}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/></svg>
        Download
      </button>
      <button
        type="button"
        class:active={activeTab === 'profiles'}
        onclick={() => (activeTab = 'profiles')}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
        Profiles
      </button>
      <button
        type="button"
        class:active={activeTab === 'settings'}
        onclick={() => (activeTab = 'settings')}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        Settings
      </button>
    </nav>

    <div class="content">
      {#if activeTab === 'download'}
        <DownloadManager
          on:switchToProfiles={() => (activeTab = 'profiles')}
          on:switchToSettings={() => (activeTab = 'settings')}
        />
      {:else if activeTab === 'profiles'}
        <ProfileManager />
      {:else}
        <SettingsManager />
      {/if}
    </div>
  </div>
</div>

<style>
  /*
    Glass/Fluent shell.
    --glass-* and --accent-* custom properties are defined here on the
    page root and inherit into every child component's scoped styles
    (CSS custom properties cross Svelte's style scoping boundary), so
    DownloadManager / ProfileManager / SettingsManager can all read
    them without redefining the palette.
  */
  .open-omni-page {
    --glass-surface: color-mix(in srgb, var(--bg) 58%, transparent);
    --glass-surface-strong: color-mix(in srgb, var(--bg) 82%, transparent);
    --glass-border: color-mix(in srgb, var(--content-border) 55%, transparent);
    --glass-radius-lg: 20px;
    --glass-radius: 14px;
    --glass-radius-sm: 10px;
    --accent-soft: color-mix(in srgb, var(--accent) 12%, transparent);
    --accent-line: color-mix(in srgb, var(--accent) 45%, transparent);
    --accent-glow: color-mix(in srgb, var(--accent) 38%, transparent);
    --accent-gradient: linear-gradient(
      135deg,
      color-mix(in srgb, var(--accent) 88%, white 14%),
      color-mix(in srgb, var(--accent) 92%, black 6%)
    );

    position: relative;
    padding: 2.5rem var(--padding) 3.5rem;
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    isolation: isolate;
  }

  .ambient-wash {
    position: absolute;
    inset: -10% -10% auto -10%;
    height: 340px;
    background: radial-gradient(
      ellipse at 50% 0%,
      var(--accent-soft) 0%,
      transparent 70%
    );
    pointer-events: none;
    z-index: -1;
  }

  .page-header {
    margin-bottom: 2rem;
    text-align: center;
  }

  .page-header h1 {
    font-size: 25px;
    margin: 0;
    font-weight: 500;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .subtitle {
    color: var(--text-secondary);
    margin: 6px 0 0;
    font-size: 14px;
  }

  .glass-panel {
    width: 100%;
    background: var(--glass-surface);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius-lg);
    padding: 1.5rem;
    box-sizing: border-box;
    backdrop-filter: blur(20px) saturate(140%);
    -webkit-backdrop-filter: blur(20px) saturate(140%);
    box-shadow:
      0 1px 0 0 rgba(255, 255, 255, 0.08) inset,
      0 18px 40px -20px rgba(0, 0, 0, 0.28);
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .tab-switcher {
    display: inline-flex;
    gap: 4px;
    align-self: center;
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: 999px;
    padding: 4px;
  }

  .tab-switcher button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    transition: color 0.15s ease, background 0.2s ease;
  }

  .tab-switcher button:hover:not(.active) {
    color: var(--text);
  }

  .tab-switcher button.active {
    background: var(--accent-gradient);
    color: var(--on-accent);
    box-shadow: 0 4px 14px -4px var(--accent-glow);
  }

  .content {
    display: flex;
    justify-content: center;
  }
</style>
