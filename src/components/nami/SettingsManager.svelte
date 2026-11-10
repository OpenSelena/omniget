<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    getOutputDir,
    getCookiesFile,
    getBrowserCookies,
    loadSettings,
    saveSettings,
  } from '$lib/stores/nami_settings_store.svelte';
  import { detectBrowsers, extractBrowserCookies, type DetectedBrowser } from '$lib/api/nami';
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';

  let outputDir = $state('');
  let cookiesFile = $state('');
  let browserCookies = $state('auto');
  let detectedBrowsers = $state<DetectedBrowser[]>([]);
  let saving = $state(false);
  let testingBridge = $state(false);
  let saveStatus = $state('');
  let saveStatusType = $state<'success' | 'error'>('success');
  let bridgeStatus = $state('');
  let bridgeStatusType = $state<'success' | 'error'>('success');
  let statusTimer: ReturnType<typeof setTimeout> | null = null;
  let bridgeTimer: ReturnType<typeof setTimeout> | null = null;

  let isDirty = $derived(
    outputDir.trim() !== getOutputDir().trim() ||
    cookiesFile.trim() !== getCookiesFile().trim() ||
    browserCookies.trim() !== getBrowserCookies().trim()
  );

  onMount(async () => {
    await loadSettings();
    outputDir = getOutputDir();
    cookiesFile = getCookiesFile();
    browserCookies = getBrowserCookies();

    try {
      detectedBrowsers = await detectBrowsers();
    } catch (err) {
      console.warn('Failed to detect browsers:', err);
    }
  });

  async function browseOutputDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: $t('nami.select_output_dir_title'),
      });
      if (selected && typeof selected === 'string') outputDir = selected;
    } catch (err) {
      console.error('Failed to open directory dialog:', err);
    }
  }

  async function browseCookiesFile() {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: $t('nami.select_cookies_title'),
      });
      if (selected && typeof selected === 'string') cookiesFile = selected;
    } catch (err) {
      console.error('Failed to open file dialog:', err);
    }
  }

  function clearCookies() {
    cookiesFile = '';
  }

  async function handleTestBridge() {
    testingBridge = true;
    bridgeStatus = '';
    if (bridgeTimer) {
      clearTimeout(bridgeTimer);
      bridgeTimer = null;
    }
    try {
      const result = await extractBrowserCookies(browserCookies, 'instagram');
      bridgeStatus = result || 'Bridge verified successfully';
      bridgeStatusType = 'success';
      bridgeTimer = setTimeout(() => {
        bridgeStatus = '';
        bridgeTimer = null;
      }, 5000);
    } catch (err) {
      console.error('Failed to test cookie bridge:', err);
      bridgeStatus = typeof err === 'string' ? err : 'Bridge test failed';
      bridgeStatusType = 'error';
    } finally {
      testingBridge = false;
    }
  }

  async function handleSave() {
    saving = true;
    saveStatus = '';
    if (statusTimer) {
      clearTimeout(statusTimer);
      statusTimer = null;
    }
    try {
      await saveSettings(outputDir, cookiesFile, browserCookies);
      outputDir = getOutputDir();
      cookiesFile = getCookiesFile();
      browserCookies = getBrowserCookies();
      saveStatus = 'nami.settings_saved';
      saveStatusType = 'success';
      statusTimer = setTimeout(() => {
        if (saveStatus === 'nami.settings_saved') saveStatus = '';
        statusTimer = null;
      }, 2500);
    } catch (error) {
      console.error('Failed to save settings:', error);
      saveStatus = 'nami.failed_to_save_settings';
      saveStatusType = 'error';
    } finally {
      saving = false;
    }
  }
</script>

<div class="sm">
  {#if saveStatus}
    <div
      class="sm-alert"
      class:success={saveStatusType === 'success'}
      class:error={saveStatusType === 'error'}
      role="status"
      aria-live="polite"
    >
      <span>{$t(saveStatus)}</span>
    </div>
  {/if}

  {#if bridgeStatus}
    <div
      class="sm-alert"
      class:success={bridgeStatusType === 'success'}
      class:error={bridgeStatusType === 'error'}
      role="status"
      aria-live="polite"
    >
      <span>{bridgeStatus}</span>
    </div>
  {/if}

  <div class="sm-group">
    <div class="sm-row">
      <svg class="sm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
        <path d="M2 4.6a1 1 0 0 1 1-1h2.6l1.1 1.4H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z" />
      </svg>
      <span class="sm-label">{$t('nami.output_label')}</span>
      <input
        id="output-dir"
        aria-label={$t('nami.output_dir_aria')}
        type="text"
        bind:value={outputDir}
        placeholder={$t('nami.choose_folder_placeholder')}
        autocomplete="off"
        spellcheck="false"
      />
      <button type="button" class="sm-choose-btn" onclick={browseOutputDir}>{$t('nami.choose')}</button>
    </div>

    <div class="sm-row">
      <svg class="sm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
        <circle cx="8" cy="8" r="6.2" />
        <path d="M1.8 8h12.4M8 1.8c2 2 3.1 4.1 3.1 6.2s-1.1 4.2-3.1 6.2c-2-2-3.1-4.1-3.1-6.2S6 3.8 8 1.8z" />
      </svg>
      <span class="sm-label">{$t('nami.browser_cookies_label')}</span>
      <select
        id="browser-cookies"
        aria-label={$t('nami.browser_cookies_aria')}
        bind:value={browserCookies}
        class="sm-select"
      >
        <option value="auto">{$t('nami.browser_cookies_auto')}</option>
        {#each detectedBrowsers as b}
          <option value={b.id}>
            {b.name} {b.available ? '✓' : ''} {b.is_default ? '(Default)' : ''}
          </option>
        {/each}
        <option value="custom">{$t('nami.browser_cookies_manual')}</option>
        <option value="none">{$t('nami.browser_cookies_disabled')}</option>
      </select>
      {#if browserCookies !== 'none' && browserCookies !== 'custom'}
        <button
          type="button"
          class="sm-test-btn"
          onclick={handleTestBridge}
          disabled={testingBridge}
        >
          {testingBridge ? $t('nami.testing_cookies') : $t('nami.test_extract_cookies')}
        </button>
      {/if}
    </div>

    {#if browserCookies === 'custom' || (cookiesFile && browserCookies !== 'none')}
      <div class="sm-row">
        <svg class="sm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
          <path d="M4.3 2h4.4L12 5.3V13a.7.7 0 0 1-.7.7H4.3a.7.7 0 0 1-.7-.7V2.7A.7.7 0 0 1 4.3 2Z" />
          <path d="M8.7 2v3.3H12" />
        </svg>
        <span class="sm-label">{$t('nami.cookies_label')}</span>
        <input
          id="cookies-file"
          aria-label={$t('nami.cookies_file_aria')}
          type="text"
          bind:value={cookiesFile}
          placeholder={$t('nami.choose_cookies_placeholder')}
          autocomplete="off"
          spellcheck="false"
        />
        {#if cookiesFile}
          <button type="button" class="sm-clear-btn" onclick={clearCookies} title={$t('nami.clear_cookies_file')}>×</button>
        {/if}
        <button type="button" class="sm-choose-btn" onclick={browseCookiesFile}>{$t('nami.choose')}</button>
      </div>
    {/if}
  </div>

  <p class="sm-hint">{$t('nami.cookies_bridge_hint')}</p>

  <div class="sm-actions">
    <button
      type="button"
      class="sm-save-btn"
      onclick={handleSave}
      disabled={saving || !isDirty}
    >
      {saving ? $t('nami.saving') : $t('nami.save_settings')}
    </button>
  </div>
</div>

<style>
  .sm {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .sm-alert {
    padding: 9px 12px;
    border-radius: var(--oo-radius, 9px);
    font-size: 12.5px;
    text-align: center;
  }

  .sm-alert.success {
    background: var(--success);
    color: var(--on-success);
  }

  .sm-alert.error {
    background: var(--error);
    color: var(--on-error);
  }

  .sm-group {
    background: var(--oo-group-bg);
    border-radius: var(--oo-radius, 9px);
    border: 1px solid var(--oo-border);
    overflow: hidden;
  }

  .sm-row {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 48px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--oo-border);
  }

  .sm-row:last-child {
    border-bottom: 0;
  }

  .sm-icon {
    width: 15px;
    height: 15px;
    color: var(--oo-text-secondary);
    flex-shrink: 0;
  }

  .sm-label {
    font-size: 13px;
    color: var(--oo-text);
    flex-shrink: 0;
  }

  .sm-row input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--oo-text);
    font-size: 12.5px;
    min-width: 0;
    text-align: right;
  }

  .sm-row input::placeholder {
    color: var(--oo-text-secondary);
    opacity: 0.8;
  }

  .sm-row input:focus-visible {
    outline: none;
  }

  .sm-select {
    flex: 1;
    border: 1px solid var(--oo-border);
    background: var(--oo-card-bg, var(--oo-group-bg));
    color: var(--oo-text);
    font-size: 12.5px;
    border-radius: 6px;
    padding: 5px 8px;
    min-width: 0;
    cursor: pointer;
  }

  .sm-select:focus-visible {
    outline: 2px solid var(--oo-accent);
    outline-offset: 1px;
  }

  .sm-clear-btn {
    background: transparent;
    border: none;
    color: var(--oo-text-secondary);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 0 4px;
    display: flex;
    align-items: center;
  }

  .sm-clear-btn:hover {
    color: var(--error);
  }

  .sm-choose-btn,
  .sm-test-btn {
    min-height: 34px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    background: var(--oo-group-bg);
    border: 1px solid var(--oo-border);
    color: var(--oo-text);
    flex-shrink: 0;
    cursor: pointer;
    transition: background 150ms ease, border-color 150ms ease;
  }

  .sm-choose-btn:hover,
  .sm-test-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--oo-text) 5%, var(--oo-group-bg));
    border-color: var(--oo-accent);
  }

  .sm-test-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .sm-hint {
    font-size: 11.5px;
    color: var(--oo-text-secondary);
    margin: -6px 0 0;
    line-height: 1.4;
  }

  .sm-actions {
    display: flex;
    justify-content: center;
  }

  .sm-save-btn {
    min-height: 42px;
    padding: 9px 26px;
    border-radius: var(--oo-radius, 9px);
    font-weight: 600;
    font-size: 13px;
    background: var(--oo-accent);
    color: var(--on-accent);
    border: none;
    cursor: pointer;
    transition: filter 150ms ease, opacity 150ms ease;
  }

  .sm-save-btn:hover:not(:disabled) {
    filter: brightness(1.06);
  }

  .sm-save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 500px) {
    .sm-row {
      align-items: flex-start;
      flex-wrap: wrap;
    }
    .sm-row input,
    .sm-select {
      order: 3;
      flex-basis: calc(100% - 24px);
      text-align: left;
    }
    .sm-choose-btn,
    .sm-test-btn {
      margin-left: auto;
    }
  }
</style>
