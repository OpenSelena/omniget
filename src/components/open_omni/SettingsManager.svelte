<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { getOutputDir, getCookiesFile, loadSettings, saveSettings } from '$lib/stores/open_omni_settings_store.svelte';
  import { onMount } from 'svelte';

  let outputDir = $state('');
  let cookiesFile = $state('');
  let saving = $state(false);
  let saveStatus = $state('');
  let saveStatusType = $state<'success' | 'error'>('success');

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
    try {
      await saveSettings(outputDir, cookiesFile);
      outputDir = getOutputDir();
      cookiesFile = getCookiesFile();
      saveStatus = 'Settings saved';
      saveStatusType = 'success';
      setTimeout(() => {
        if (saveStatus === 'Settings saved') saveStatus = '';
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

<div class="settings-manager">
  {#if saveStatus}
    <div class="status-alert" class:success={saveStatusType === 'success'} class:error={saveStatusType === 'error'}>
      <span>{saveStatus}</span>
    </div>
  {/if}

  <div class="field-card">
    <label for="output-dir">Output directory</label>
    <div class="input-with-button">
      <input id="output-dir" type="text" bind:value={outputDir} placeholder="E:\OmniGet" />
      <button type="button" class="browse-btn" onclick={browseOutputDir}>
        Browse
      </button>
    </div>
  </div>

  <div class="field-card">
    <label for="cookies-file">Cookies file <span class="optional-tag">(optional, needed for Stories and Highlights)</span></label>
    <div class="input-with-button">
      <input id="cookies-file" type="text" bind:value={cookiesFile} placeholder="instagram.com_cookies.txt" />
      <button type="button" class="browse-btn" onclick={browseCookiesFile}>
        Browse
      </button>
    </div>
  </div>

  <button type="button" class="primary-btn" onclick={handleSave} disabled={saving}>
    {saving ? 'Saving…' : 'Save settings'}
  </button>
</div>

<style>
  /* Centered column, same width budget as the other tabs. */
  .settings-manager {
    padding: 0;
    max-width: 460px;
    width: 100%;
    margin: 0 auto;
  }

  .status-alert {
    padding: 10px 14px;
    margin-bottom: 20px;
    border-radius: var(--border-radius);
    font-size: 13px;
    text-align: center;
  }

  .status-alert.success {
    background: var(--success);
    color: var(--on-success);
  }

  .status-alert.error {
    background: var(--error);
    color: var(--on-error);
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

  .optional-tag {
    font-weight: 400;
    color: var(--text-secondary);
  }

  .input-with-button {
    display: flex;
    gap: 8px;
  }

  .input-with-button input {
    flex: 1;
    padding: 10px 14px;
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: 8px;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
  }

  .input-with-button input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .browse-btn {
    padding: 6px 16px;
    border-radius: 16px;
    border: 1px solid var(--input-border);
    background: var(--button-elevated);
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }

  .browse-btn:hover {
    border-color: var(--accent);
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
</style>
