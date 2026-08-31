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
    <header class="oo-header">
      <p class="oo-kicker">Open Omni</p>
      <h1 class="oo-title">{NAV_ITEMS.find((i) => i.key === activeTab)?.label}</h1>
      <p class="oo-description">
        {activeTab === 'download'
          ? 'Save posts and media from your connected social profiles.'
          : activeTab === 'profiles'
            ? 'Manage the profiles you use most often.'
            : 'Choose where Open Omni saves downloaded media.'}
      </p>
    </header>

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
    --oo-accent: var(--accent);
    --oo-border: var(--content-border);
    --oo-sidebar-bg: var(--sidebar-bg);
    --oo-content-bg: var(--button-elevated);
    --oo-group-bg: var(--button);
    --oo-text: var(--text);
    --oo-text-secondary: var(--text-secondary);
    --oo-radius-lg: calc(var(--border-radius) * 1.25);
    --oo-radius: var(--border-radius);
    --oo-radius-sm: calc(var(--border-radius) * 0.72);

    width: min(100%, 940px);
    margin: clamp(16px, 4vh, 40px) auto;
    display: grid;
    grid-template-columns: 216px minmax(0, 1fr);
    min-height: min(620px, calc(100dvh - 128px));
    border: 1px solid var(--oo-border);
    border-radius: var(--oo-radius-lg);
    overflow: hidden;
    box-sizing: border-box;
  }

  .oo-sidebar {
    background: var(--oo-sidebar-bg);
    border-right: 1px solid var(--oo-border);
    padding: calc(var(--padding) * 1.25) var(--padding);
    box-sizing: border-box;
  }

  .oo-nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .oo-nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 8px 10px;
    border-radius: var(--oo-radius-sm);
    font-size: 14px;
    font-weight: 500;
    color: var(--oo-text);
    text-align: left;
    transition: background 150ms ease, color 150ms ease;
  }

  .oo-nav-item img {
    width: 20px;
    height: 20px;
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
    color: var(--on-accent);
  }

  .oo-nav-item:focus-visible,
  :global(.oo-shell button:focus-visible),
  :global(.oo-shell input:focus-visible) {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .oo-content {
    min-width: 0;
    padding: clamp(24px, 4vw, 40px);
    overflow-y: auto;
    box-sizing: border-box;
  }

  .oo-header {
    max-width: 580px;
    margin-bottom: 28px;
  }

  .oo-kicker {
    margin: 0 0 6px;
    color: var(--oo-text-secondary);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .oo-title {
    font-size: 24px;
    font-weight: 500;
    letter-spacing: -0.01em;
    color: var(--oo-text);
    margin: 0;
  }

  .oo-description {
    max-width: 52ch;
    margin: 8px 0 0;
    color: var(--oo-text-secondary);
    font-size: 14px;
    line-height: 1.55;
  }

  :global(.oo-content > :not(.oo-header)) {
    max-width: 580px;
  }

  @media (max-width: 760px) {
    .oo-shell {
      grid-template-columns: 180px minmax(0, 1fr);
      min-height: min(580px, calc(100dvh - 112px));
    }
  }

  @media (max-width: 600px) {
    .oo-shell {
      grid-template-columns: 1fr;
      min-height: 0;
      margin: var(--padding) auto;
    }
    .oo-sidebar {
      width: 100%;
      border-right: 0;
      border-bottom: 1px solid var(--oo-border);
      padding: 8px;
    }
    .oo-nav {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
    .oo-nav-item {
      justify-content: center;
      gap: 7px;
      min-height: 40px;
      padding: 6px;
      font-size: 12px;
    }
    .oo-content { padding: 24px 20px 28px; }
    .oo-header { margin-bottom: 22px; }
    .oo-title { font-size: 22px; }
  }
</style>
