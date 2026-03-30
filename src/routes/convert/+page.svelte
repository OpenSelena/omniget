<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";

  let checking = $state(true);
  let installed = $state(false);

  onMount(async () => {
    try {
      const plugins = await invoke<{ id: string; enabled: boolean }[]>("list_plugins");
      installed = plugins.some((p) => p.id === "convert" && p.enabled);
    } catch {}
    checking = false;
  });
</script>

{#if checking}
  <div class="redirect-page">
    <span class="spinner"></span>
  </div>
{:else if installed}
  <div class="redirect-page">
    <p>{$t("marketplace.browse_loading")}</p>
  </div>
{:else}
  <div class="redirect-page">
    <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M20 10H4l4-4" /><path d="M4 14h16l-4 4" />
    </svg>
    <h2>{$t("marketplace.plugin_not_installed")}</h2>
    <p>{$t("marketplace.plugin_install_hint")}</p>
    <a href="/marketplace" class="marketplace-link">{$t("marketplace.go_to_marketplace")}</a>
  </div>
{/if}

<style>
  .redirect-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - var(--padding) * 4);
    gap: calc(var(--padding) * 1.5);
    text-align: center;
    color: var(--gray);
  }
  .redirect-page h2 { font-size: 18px; color: var(--secondary); }
  .redirect-page p { font-size: 14px; max-width: 300px; }
  .spinner { width: 24px; height: 24px; border: 2px solid var(--input-border); border-top-color: var(--secondary); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .marketplace-link { padding: 10px 24px; font-size: 14px; font-weight: 500; background: var(--cta); color: var(--on-cta); border-radius: var(--border-radius); text-decoration: none; }
</style>
