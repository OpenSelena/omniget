<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { getOutputDir, getCookiesFile, loadSettings, saveSettings } from '$lib/stores/open_omni_settings_store.svelte';
  import { onMount } from 'svelte';

  let outputDir = $state('');
  let cookiesFile = $state('');
  let saving = $state(false);
  let saveStatus = $state('');
  let saveStatusType = $state<'success' | 'error'>('success');
  let statusTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    await loadSettings();
    outputDir = getOutputDir();
    cookiesFile = getCookiesFile();
  });

  async function browseOutputDir() {
    const selected = await open({ directory: true, multiple: false, title: 'Select Output Directory' });
    if (selected && typeof selected === 'string') outputDir = selected;
  }

  async function browseCookiesFile() {
    const selected = await open({ directory: false, multiple: false, title: 'Select Cookies File' });
    if (selected && typeof selected === 'string') cookiesFile = selected;
  }

  async function handleSave() {
    saving = true;
    saveStatus = '';
    if (statusTimer) {
      clearTimeout(statusTimer);
      statusTimer = null;
    }
    try {
      await saveSettings(outputDir, cookiesFile);
      outputDir = getOutputDir();
      cookiesFile = getCookiesFile();
      saveStatus = 'Settings saved';
      saveStatusType = 'success';
      statusTimer = setTimeout(() => {
        if (saveStatus === 'Settings saved') saveStatus = '';
        statusTimer = null;
      }, 2500);
    } catch (error) {
      console.error('Failed to save settings:', error);
      saveStatus = 'Failed to save settings';
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
      <span>{saveStatus}</span>
    </div>
  {/if}

  <div class="sm-group">
    <div class="sm-row">
      <svg class="sm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
        <path d="M2 4.6a1 1 0 0 1 1-1h2.6l1.1 1.4H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z" />
      </svg>
      <span class="sm-label">Output</span>
      <input
        id="output-dir"
        aria-label="Output directory"
        type="text"
        bind:value={outputDir}
        placeholder="Choose a folder…"
        autocomplete="off"
        spellcheck="false"
      />
      <button type="button" class="sm-choose-btn" onclick={browseOutputDir}>Choose…</button>
    </div>
    <div class="sm-row">
      <svg class="sm-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
        <path d="M4.3 2h4.4L12 5.3V13a.7.7 0 0 1-.7.7H4.3a.7.7 0 0 1-.7-.7V2.7A.7.7 0 0 1 4.3 2Z" />
        <path d="M8.7 2v3.3H12" />
      </svg>
      <span class="sm-label">Cookies</span>
      <input
        id="cookies-file"
        aria-label="Cookies file (optional, needed for Stories and Highlights)"
        type="text"
        bind:value={cookiesFile}
        placeholder="Choose a cookies file…"
        autocomplete="off"
        spellcheck="false"
      />
      <button type="button" class="sm-choose-btn" onclick={browseCookiesFile}>Choose…</button>
    </div>
  </div>

  <p class="sm-hint">Cookies are only required for downloading Stories and Highlights.</p>

  <div class="sm-actions">
    <button type="button" class="sm-save-btn" onclick={handleSave} disabled={saving}>
      {saving ? 'Saving…' : 'Save Settings'}
    </button>
  </div>
</div>

<style>
  /* Reads --oo-accent / --oo-border / --oo-group-bg / --oo-text* custom
     properties inherited from the shell in +page.svelte. */
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
  .sm-alert.success { background: var(--success); color: var(--on-success); }
  .sm-alert.error { background: var(--error); color: var(--on-error); }

  .sm-group {
    background: var(--oo-group-bg);
    border-radius: var(--oo-radius, 9px);
    box-shadow: 0 0 0 1px var(--oo-border);
    overflow: hidden;
  }

  .sm-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--oo-border);
  }
  .sm-row:last-child { border-bottom: 0; }

  .sm-icon { width: 15px; height: 15px; color: var(--oo-text-secondary); flex-shrink: 0; }

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
  .sm-row input::placeholder { color: var(--oo-text-secondary); opacity: 0.8; }
  .sm-row input:focus { outline: none; }

  .sm-choose-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    background: var(--oo-group-bg);
    box-shadow: 0 0 0 1px var(--oo-border);
    color: var(--oo-text);
    flex-shrink: 0;
  }
  .sm-choose-btn:hover { background: color-mix(in srgb, var(--oo-text) 5%, var(--oo-group-bg)); }

  .sm-hint {
    font-size: 11.5px;
    color: var(--oo-text-secondary);
    margin: -6px 0 0;
  }

  .sm-actions {
    display: flex;
    justify-content: center;
  }
  .sm-save-btn {
    padding: 8px 26px;
    border-radius: var(--oo-radius, 9px);
    font-weight: 600;
    font-size: 13px;
    background: var(--oo-accent);
    color: #fff;
  }
  .sm-save-btn:hover:not(:disabled) { filter: brightness(1.06); }
  .sm-save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
