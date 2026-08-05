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

<div class="settings-manager">
  {#if saveStatus}
    <div
      class="status-alert"
      class:success={saveStatusType === 'success'}
      class:error={saveStatusType === 'error'}
      role="status"
      aria-live="polite"
    >
      <span>{saveStatus}</span>
    </div>
  {/if}

  <div class="settings-field">
    <div class="input-with-button">
      <svg class="field-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>
      <input id="output-dir" aria-label="Output directory" type="text" bind:value={outputDir} placeholder="E:\OmniGet" autocomplete="off" spellcheck="false" />
      <button type="button" class="browse-btn" onclick={browseOutputDir}>
        Browse
      </button>
    </div>
  </div>

  <div class="settings-field">
    <div class="input-with-button">
      <svg class="field-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/></svg>
      <input id="cookies-file" aria-label="Cookies file (optional, needed for Stories and Highlights)" type="text" bind:value={cookiesFile} placeholder="instagram.com_cookies.txt" autocomplete="off" spellcheck="false" />
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
  /* Reads --glass-*, --accent-* custom properties inherited from the
     page shell (src/routes/open-omni/+page.svelte). */
  .settings-manager {
    max-width: 440px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .status-alert {
    padding: 10px 14px;
    border-radius: var(--glass-radius-sm, 10px);
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

  .settings-field label {
    display: block;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .optional-tag {
    font-weight: 400;
    color: var(--text-muted, var(--text-secondary));
  }

  .input-with-button {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--glass-surface-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--glass-radius, 14px);
    padding: 0 8px 0 14px;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .input-with-button:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .field-icon {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .input-with-button input {
    flex: 1;
    padding: 12px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 14px;
    box-sizing: border-box;
  }

  .input-with-button input:focus {
    outline: none;
  }

  .browse-btn {
    padding: 7px 16px;
    border-radius: 999px;
    border: 1px solid var(--glass-border);
    background: var(--glass-surface);
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.15s ease;
  }

  .browse-btn:hover {
    border-color: var(--accent-line);
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
    transition: box-shadow 0.12s ease, transform 0.12s ease;
  }

  .primary-btn:hover:not(:disabled) {
    box-shadow: 0 8px 24px -6px var(--accent-glow);
  }

  .primary-btn:active:not(:disabled) {
    transform: scale(0.99);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }
</style>
