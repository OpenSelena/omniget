<script lang="ts">
  import DownloadManager from '$components/open_omni/DownloadManager.svelte';
  import ProfileManager from '$components/open_omni/ProfileManager.svelte';
  import SettingsManager from '$components/open_omni/SettingsManager.svelte';

  type TabKey = 'download' | 'profiles' | 'settings';

  let activeTab = $state<TabKey>('download');

  const NAV_ITEMS: { key: TabKey; label: string; icon: string }[] = [
    { key: 'download', label: 'Download', icon: '/icons/open-omni-download.png' },
    { key: 'profiles', label: 'Profiles', icon: '/icons/open-omni-profiles.png' },
    { key: 'settings', label: 'Settings', icon: '/icons/open-omni-settings.png' }
  ];

  function switchTab(tab: TabKey) {
    activeTab = tab;
  }
</script>

<svelte:head>
  <title>Open Omni - Social Media Downloader</title>
</svelte:head>

<div class="oo-shell">
  <aside class="oo-sidebar" aria-label="Open Omni sections">
    <nav class="oo-nav">
      {#each NAV_ITEMS as item (item.key)}
        <button
          type="button"
          class="oo-nav-item"
          class:active={activeTab === item.key}
          aria-current={activeTab === item.key ? 'page' : undefined}
          onclick={() => switchTab(item.key)}
        >
          <img src={item.icon} alt="" width="24" height="24" />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>
  </aside>

  <div class="oo-content">
    <h1 class="oo-title">{NAV_ITEMS.find((i) => i.key === activeTab)?.label}</h1>

    {#if activeTab === 'download'}
      <DownloadManager
        on:switchToProfiles={() => switchTab('profiles')}
        on:switchToSettings={() => switchTab('settings')}
      />
    {:else if activeTab === 'profiles'}
      <ProfileManager />
    {:else}
      <SettingsManager />
    {/if}
  </div>
</div>

<style>
  /*
    Open Omni shell — macOS System Settings layout.
    Reads app theme tokens: --accent, --cta, --bg, --button, --secondary,
    --tertiary, --content-border. Falls back to sane defaults if a token
    is not defined by the surrounding app shell.
  */
  .oo-shell {
    --oo-accent: var(--cta, var(--accent, #0071e3));
    --oo-border: color-mix(in srgb, var(--content-border, var(--tertiary, #67676c)) 22%, transparent);
    --oo-sidebar-bg: color-mix(in srgb, var(--button, var(--bg, #fff)) 88%, var(--tertiary, #67676c) 10%);
    --oo-content-bg: var(--bg, #ececef);
    --oo-group-bg: var(--button, #ffffff);
    --oo-text: var(--text, var(--secondary, #1d1d1f));
    --oo-text-secondary: var(--text-secondary, var(--tertiary, #67676c));
    --oo-radius-lg: 12px;
    --oo-radius: 9px;
    --oo-radius-sm: 6px;

    max-width: 680px;
    margin: 0 auto;
    display: flex;
    min-height: 480px;
    background: var(--oo-content-bg);
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius-lg);
    overflow: hidden;
    box-shadow: 0 18px 44px -24px rgba(0, 0, 0, 0.28);
  }

  .oo-sidebar {
    width: 196px;
    flex-shrink: 0;
    background: var(--oo-sidebar-bg);
    border-right: 1px solid var(--oo-border);
    padding: 12px 8px;
    box-sizing: border-box;
  }

  .oo-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .oo-nav-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border-radius: var(--oo-radius-sm);
    font-size: 13px;
    color: var(--oo-text);
    text-align: left;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .oo-nav-item img {
    width: 24px;
    height: 24px;
    object-fit: contain;
    flex-shrink: 0;
    filter: brightness(0) invert(1);
  }

  :global([data-theme="light"]) .oo-nav-item:not(.active) img,
  :global([data-theme="catppuccin-latte"]) .oo-nav-item:not(.active) img,
  :global([data-theme="eink-day"]) .oo-nav-item:not(.active) img,
  :global([data-theme="eink-sepia"]) .oo-nav-item:not(.active) img,
  :global([data-theme="nyxvamp-radiance"]) .oo-nav-item:not(.active) img {
    filter: brightness(0);
  }

  .oo-nav-item:not(.active):hover {
    background: color-mix(in srgb, var(--oo-text) 6%, transparent);
  }

  .oo-nav-item.active {
    background: var(--oo-accent);
    color: #ffffff;
  }

  .oo-content {
    flex: 1;
    min-width: 0;
    padding: 22px 24px 26px;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .oo-title {
    font-size: 19px;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--oo-text);
    margin: 0 0 16px;
  }

  @media (max-width: 520px) {
    .oo-shell {
      flex-direction: column;
      min-height: 0;
    }
    .oo-sidebar {
      width: 100%;
      border-right: 0;
      border-bottom: 1px solid var(--oo-border);
    }
    .oo-nav {
      flex-direction: row;
      overflow-x: auto;
    }
    .oo-nav-item {
      flex-shrink: 0;
    }
  }
</style>
