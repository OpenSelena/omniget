<script lang="ts">
  import DownloadManager from '$components/open_omni/DownloadManager.svelte';
  import ProfileManager from '$components/open_omni/ProfileManager.svelte';
  import SettingsManager from '$components/open_omni/SettingsManager.svelte';

  let activeTab = $state('download');
</script>

<svelte:head>
  <title>Open Omni - Social Media Downloader</title>
</svelte:head>

<div class="open-omni-page">
  <header class="page-header">
    <h1>Open Omni</h1>
    <p class="subtitle">Social media profile & batch downloader</p>
  </header>

  <div class="tab-switcher">
    <button type="button" class:active={activeTab === 'download'} onclick={() => activeTab = 'download'}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/></svg>
      Download
    </button>
    <button type="button" class:active={activeTab === 'profiles'} onclick={() => activeTab = 'profiles'}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
      Profiles
    </button>
    <button type="button" class:active={activeTab === 'settings'} onclick={() => activeTab = 'settings'}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
      Settings
    </button>
  </div>

  <div class="content">
    {#if activeTab === 'download'}
      <DownloadManager
        on:switchToProfiles={() => activeTab = 'profiles'}
        on:switchToSettings={() => activeTab = 'settings'}
      />
    {:else if activeTab === 'profiles'}
      <ProfileManager />
    {:else}
      <SettingsManager />
    {/if}
  </div>
</div>

<style>
  /* Whole page is centered as a column, capped width, generous
     top/bottom breathing room instead of hugging the left edge. */
  .open-omni-page {
    padding: var(--padding) var(--padding) 3rem;
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .page-header {
    margin: 1rem 0 2rem;
    text-align: center;
  }

  .page-header h1 {
    font-size: 24px;
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

  .tab-switcher {
    display: inline-flex;
    gap: 4px;
    background: var(--button-elevated);
    border-radius: 999px;
    padding: 5px;
    margin-bottom: 2rem;
    border: 1px solid var(--content-border);
  }

  .tab-switcher button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 20px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .tab-switcher button:hover:not(.active) {
    color: var(--text);
  }

  .tab-switcher button.active {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 500;
  }

  .content {
    background: var(--button-elevated);
    border: 1px solid var(--content-border);
    border-radius: 16px;
    padding: 2rem;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    justify-content: center;
  }
</style>
