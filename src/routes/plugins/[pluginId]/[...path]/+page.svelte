<script lang="ts">
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";

  type PluginManifest = {
    id: string;
    name: string;
    version: string;
  };

  let pluginId = $derived(page.params.pluginId);
  let subPath = $derived(page.params.path || "");
  let pluginFound = $state<boolean | null>(null);
  let pluginName = $state("");
  let frontendHtml = $state("");

  onMount(async () => {
    try {
      const manifests = await invoke<PluginManifest[]>("get_loaded_plugin_manifests");
      const manifest = manifests.find((m) => m.id === pluginId);
      if (manifest) {
        pluginFound = true;
        pluginName = manifest.name;
        const frontendPath = await invoke<string>("get_plugin_frontend_path", { pluginId });
        frontendHtml = frontendPath;
      } else {
        pluginFound = false;
      }
    } catch {
      pluginFound = false;
    }
  });
</script>

<div class="plugin-page">
  {#if pluginFound === null}
    <div class="plugin-loading">
      <span class="spinner"></span>
    </div>
  {:else if pluginFound === false}
    <div class="plugin-not-found">
      <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
      </svg>
      <h2>{$t("marketplace.plugin_not_installed")}</h2>
      <p>{$t("marketplace.plugin_install_hint")}</p>
      <a href="/marketplace" class="marketplace-link">{$t("marketplace.go_to_marketplace")}</a>
    </div>
  {:else}
    <div class="plugin-content">
      <p>{pluginName} — {subPath || "index"}</p>
    </div>
  {/if}
</div>

<style>
  .plugin-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - var(--padding) * 4);
  }

  .plugin-loading {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--input-border);
    border-top-color: var(--secondary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .plugin-not-found {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) * 1.5);
    text-align: center;
    color: var(--gray);
  }

  .plugin-not-found h2 {
    font-size: 18px;
    color: var(--secondary);
  }

  .plugin-not-found p {
    font-size: 14px;
    max-width: 300px;
  }

  .marketplace-link {
    padding: 10px 24px;
    font-size: 14px;
    font-weight: 500;
    background: var(--cta);
    color: var(--on-cta);
    border-radius: var(--border-radius);
    text-decoration: none;
  }

  .plugin-content {
    width: 100%;
  }
</style>
