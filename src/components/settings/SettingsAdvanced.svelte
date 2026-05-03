<script lang="ts">
  import { t } from "$lib/i18n";
  import { isDebugEnabled, setDebugEnabled, setDebugPanelOpen } from "$lib/stores/debug-store.svelte";
  import { getSettings, updateSettings, toggleBool, changeNumber } from "./settings-helpers";

  let { resetting = false, onReset }: {
    resetting?: boolean;
    onReset: () => void;
  } = $props();

  let settings = $derived(getSettings());
</script>

{#if settings}
<section class="section">
  <h5 class="section-title">{$t('settings.advanced.title')}</h5>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.advanced.max_concurrent_downloads')}</span>
        <span class="setting-path">{$t('settings.advanced.max_concurrent_downloads_desc')}</span>
      </div>
      <input type="number" class="input-number" min="1" max="10" value={settings.advanced.max_concurrent_downloads} onchange={(e) => changeNumber("advanced", "max_concurrent_downloads", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.advanced.max_concurrent_segments')}</span>
      <input type="number" class="input-number" min="1" max="100" value={settings.advanced.max_concurrent_segments} onchange={(e) => changeNumber("advanced", "max_concurrent_segments", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.advanced.concurrent_fragments')}</span>
        <span class="setting-path">{$t('settings.advanced.concurrent_fragments_desc')}</span>
      </div>
      <input type="number" class="input-number" min="1" max="32" value={settings.advanced.concurrent_fragments} onchange={(e) => changeNumber("advanced", "concurrent_fragments", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.advanced.max_retries')}</span>
      <input type="number" class="input-number" min="1" max="20" value={settings.advanced.max_retries} onchange={(e) => changeNumber("advanced", "max_retries", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.advanced.stagger_delay')}</span>
        <span class="setting-path">{$t('settings.advanced.stagger_delay_desc')}</span>
      </div>
      <input type="number" class="input-number" min="0" max="2000" step="50" value={settings.advanced.stagger_delay_ms} onchange={(e) => changeNumber("advanced", "stagger_delay_ms", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.advanced.torrent_listen_port')}</span>
        <span class="setting-path">{$t('settings.advanced.torrent_listen_port_desc')}</span>
      </div>
      <input type="number" class="input-number" min="1024" max="65525" value={settings.advanced.torrent_listen_port} onchange={(e) => changeNumber("advanced", "torrent_listen_port", e)} />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.advanced.cookies_from_browser')}</span>
        <span class="setting-path">{$t('settings.advanced.cookies_from_browser_desc')}</span>
      </div>
      <input
        type="text"
        class="input-text"
        placeholder="e.g., firefox, chrome, edge"
        value={settings.advanced?.cookies_from_browser ?? ""}
        onchange={(e) => updateSettings({ advanced: { cookies_from_browser: (e.target as HTMLInputElement).value.trim() } })}
      />
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('debug.enable')}</span>
        <span class="setting-path">{$t('debug.enable_desc')}</span>
      </div>
      <button class="toggle" class:on={isDebugEnabled()} onclick={() => setDebugEnabled(!isDebugEnabled())} role="switch" aria-checked={isDebugEnabled()} aria-label={$t('debug.enable') as string}>
        <span class="toggle-knob"></span>
      </button>
    </div>
    {#if isDebugEnabled()}
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('debug.open_panel')}</span>
        <button class="button" onclick={() => setDebugPanelOpen(true)}>
          {$t('debug.open_panel')}
        </button>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.advanced.reset')}</span>
      <button class="button reset-btn" onclick={onReset} disabled={resetting}>
        {$t('settings.advanced.reset')}
      </button>
    </div>
  </div>
</section>
{/if}
