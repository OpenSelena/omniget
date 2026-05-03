<script lang="ts">
  import { t } from "$lib/i18n";
  import { getSettings, toggleBool, changeNumber, type DependencyStatus } from "./settings-helpers";
  import DependencyRow from "./DependencyRow.svelte";

  let { deps = [], installingDep = null, onInstallDep, onRefresh }: {
    deps?: DependencyStatus[];
    installingDep?: string | null;
    onInstallDep: (name: string, variant: string | null) => void;
    onRefresh?: () => void | Promise<void>;
  } = $props();

  let settings = $derived(getSettings());
</script>

{#if settings}
  <section class="section">
    <h5 class="section-title">{$t('settings.download.telegram_plugin_section')}</h5>
    <div class="card">
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.telegram.concurrent_downloads')}</span>
          <span class="setting-path">{$t('settings.telegram.concurrent_downloads_desc')}</span>
        </div>
        <input
          type="number"
          class="input-number"
          min="1"
          max="10"
          value={settings.telegram.concurrent_downloads}
          onchange={(e) => changeNumber("telegram", "concurrent_downloads", e)}
        />
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.telegram.fix_file_extensions')}</span>
          <span class="setting-path">{$t('settings.telegram.fix_file_extensions_desc')}</span>
        </div>
        <button
          class="toggle"
          class:on={settings.telegram.fix_file_extensions}
          onclick={() => toggleBool("telegram", "fix_file_extensions", settings.telegram.fix_file_extensions)}
          role="switch"
          aria-checked={settings.telegram.fix_file_extensions}
          aria-label={$t('settings.telegram.fix_file_extensions') as string}
        >
          <span class="toggle-knob"></span>
        </button>
      </div>
    </div>
  </section>

  {#if deps.length > 0}
    <section class="section">
      <h5 class="section-title">{$t('settings.dependencies.title')}</h5>
      <div class="card">
        {#each deps as dep, i (dep.name)}
          {#if i > 0}<div class="divider"></div>{/if}
          <DependencyRow
            name={dep.name}
            installed={dep.installed}
            version={dep.version}
            busy={installingDep === dep.name}
            onInstall={(variant) => onInstallDep(dep.name, variant)}
            onAfterCustomFile={onRefresh}
          />
        {/each}
      </div>
    </section>
  {/if}
{/if}
