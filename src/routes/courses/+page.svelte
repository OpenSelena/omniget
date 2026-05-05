<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { onMount } from "svelte";
  import { PLATFORM_ICONS, DEFAULT_ICON } from "./platform-icons";
  import { t } from "$lib/i18n";

  type PlatformConfig = {
    id: string;
    name: string;
    color: string;
    icon: string;
    commands: { check_session: string };
  };

  const FALLBACK_PLATFORMS: PlatformConfig[] = [
    { id: "hotmart", name: "Hotmart", color: "#F04E23", icon: "hotmart", commands: { check_session: "hotmart_check_session" } },
    { id: "udemy", name: "Udemy", color: "#A435F0", icon: "udemy", commands: { check_session: "udemy_check_session" } },
    { id: "kiwify", name: "Kiwify", color: "#22C55E", icon: "kiwify", commands: { check_session: "kiwify_check_session" } },
    { id: "teachable", name: "Teachable", color: "#4B5563", icon: "teachable", commands: { check_session: "teachable_check_session" } },
    { id: "kajabi", name: "Kajabi", color: "#2563EB", icon: "kajabi", commands: { check_session: "kajabi_check_session" } },
    { id: "gumroad", name: "Gumroad", color: "#FF90E8", icon: "gumroad", commands: { check_session: "gumroad_check_session" } },
    { id: "skool", name: "Skool", color: "#5865F2", icon: "skool", commands: { check_session: "skool_check_session" } },
    { id: "greatcourses", name: "Wondrium", color: "#1E3A5F", icon: "greatcourses", commands: { check_session: "wondrium_check_session" } },
    { id: "thinkific", name: "Thinkific", color: "#4A90D9", icon: "thinkific", commands: { check_session: "thinkific_check_session" } },
    { id: "rocketseat", name: "Rocketseat", color: "#8257E5", icon: "rocketseat", commands: { check_session: "rocketseat_check_session" } },
  ];

  type PluginStatus =
    | "checking"
    | "ready"
    | "not-installed"
    | "needs-restart"
    | "incompatible"
    | "load-failed";
  type PluginLoadError = {
    message: string;
    kind: string;
    plugin_abi?: number | null;
    expected_abi?: number | null;
  };
  let pluginStatus = $state<PluginStatus>("checking");
  let loadError = $state<PluginLoadError | null>(null);

  let platforms: PlatformConfig[] = $state([]);
  let searchQuery = $state("");
  let authStatus: Record<string, { checked: boolean; email: string | null; error: boolean }> = $state({});
  let usingFallback = $state(false);

  let filteredPlatforms = $derived(
    searchQuery.trim() === ""
      ? platforms
      : platforms.filter((p) =>
          p.name.toLowerCase().includes(searchQuery.trim().toLowerCase())
        )
  );

  onMount(async () => {
    console.log("[courses] onMount start");
    try {
      const plugins = await invoke<{
        id: string;
        enabled: boolean;
        loaded: boolean;
        load_error?: PluginLoadError | null;
      }[]>("list_plugins");
      console.log("[courses] list_plugins:", JSON.stringify(plugins));
      const courses = plugins.find((p) => p.id === "courses");
      if (!courses || !courses.enabled) {
        console.log("[courses] plugin not installed or disabled");
        pluginStatus = "not-installed";
        return;
      }
      if (!courses.loaded) {
        if (courses.load_error) {
          loadError = courses.load_error;
          pluginStatus =
            courses.load_error.kind === "abi_mismatch" ||
            courses.load_error.kind === "missing_abi_symbol"
              ? "incompatible"
              : "load-failed";
          console.log("[courses] plugin load failed:", courses.load_error);
        } else {
          console.log("[courses] plugin enabled but not loaded — needs restart");
          pluginStatus = "needs-restart";
        }
        return;
      }
      console.log("[courses] plugin ready");
      pluginStatus = "ready";
    } catch (e) {
      console.error("[courses] list_plugins failed:", e);
      pluginStatus = "ready";
    }

    try {
      console.log("[courses] calling get_platforms...");
      platforms = await pluginInvoke<PlatformConfig[]>("courses", "get_platforms");
      console.log("[courses] get_platforms returned:", platforms.length, "platforms", JSON.stringify(platforms.map(p => p.id)));
    } catch (e) {
      console.error("[courses] get_platforms FAILED, using fallback:", e);
      platforms = FALLBACK_PLATFORMS;
      usingFallback = true;
    }

    if (!usingFallback) {
      for (const platform of platforms) {
        authStatus[platform.id] = { checked: false, email: null, error: false };
        pluginInvoke<string>("courses", platform.commands.check_session)
          .then((email) => {
            console.log(`[courses] ${platform.id} session:`, email);
            authStatus[platform.id] = { checked: true, email, error: false };
          })
          .catch((e) => {
            console.log(`[courses] ${platform.id} session check failed:`, e);
            authStatus[platform.id] = { checked: true, email: null, error: true };
          });
      }
    }
  });

  function handleCardClick(platform: PlatformConfig) {
    goto(`/courses/${platform.id}`);
  }

  function handleKeyDown(e: KeyboardEvent, platform: PlatformConfig) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleCardClick(platform);
    }
  }

  function getIconSvg(icon: string): string {
    return PLATFORM_ICONS[icon] ?? DEFAULT_ICON;
  }
</script>

{#if pluginStatus === "checking"}
  <div class="plugin-guard"><span class="spinner"></span></div>
{:else if pluginStatus === "not-installed"}
  <div class="plugin-guard">
    <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 19.5A2.5 2.5 0 016.5 17H20" />
      <path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z" />
    </svg>
    <h2>{$t("marketplace.plugin_not_installed")}</h2>
    <p>{$t("marketplace.plugin_install_hint")}</p>
    <a href="/marketplace" class="guard-link">{$t("marketplace.go_to_marketplace")}</a>
  </div>
{:else if pluginStatus === "needs-restart"}
  <div class="plugin-guard">
    <h2>{$t("marketplace.restart_required")}</h2>
    <p>{$t("marketplace.plugin_restart_hint")}</p>
  </div>
{:else if pluginStatus === "incompatible"}
  <div class="plugin-guard">
    <h2>{$t("marketplace.plugin_incompatible_title")}</h2>
    <p>{$t("marketplace.plugin_incompatible_hint")}</p>
    <a href="/marketplace" class="guard-link">{$t("marketplace.go_to_marketplace")}</a>
    {#if loadError}
      <p class="guard-detail"><code>{loadError.message}</code></p>
    {/if}
  </div>
{:else if pluginStatus === "load-failed"}
  <div class="plugin-guard">
    <h2>{$t("marketplace.plugin_load_failed_title")}</h2>
    <p>{$t("marketplace.plugin_load_failed_hint")}</p>
    <a href="/marketplace" class="guard-link">{$t("marketplace.go_to_marketplace")}</a>
    {#if loadError}
      <p class="guard-detail"><code>{loadError.message}</code></p>
    {/if}
  </div>
{:else}
<div class="courses-page">
  <h1 class="page-title">{$t("courses.title")}</h1>

  {#if usingFallback}
    <div class="fallback-banner">
      {$t("courses.update_plugin_hint")}
    </div>
  {/if}

  <input
    class="search-input"
    type="text"
    placeholder={$t("courses.search_placeholder")}
    bind:value={searchQuery}
  />

  <div class="platform-grid">
    {#each filteredPlatforms as platform (platform.id)}
      <div
        class="platform-card"
        role="button"
        tabindex={0}
        onclick={() => handleCardClick(platform)}
        onkeydown={(e) => handleKeyDown(e, platform)}
      >
        <div class="card-icon" style="--platform-color: {platform.color}">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            {@html getIconSvg(platform.icon)}
          </svg>
        </div>
        <span class="card-name">{platform.name}</span>
        <span class="card-status">
          {#if authStatus[platform.id]?.checked && authStatus[platform.id]?.error}
            <span class="status-dot error"></span>
            {$t("courses.connection_failed")}
          {:else if authStatus[platform.id]?.checked && authStatus[platform.id]?.email}
            <span class="status-dot connected"></span>
            <span class="status-email">{authStatus[platform.id].email}</span>
          {:else if authStatus[platform.id]?.checked}
            <span class="status-dot disconnected"></span>
            {$t("courses.not_connected")}
          {/if}
        </span>
      </div>
    {/each}
  </div>
</div>
{/if}

<style>
  .courses-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) * 2);
    width: 100%;
  }

  h1 {
    font-size: 20px;
    font-weight: 500;
    margin-block: 0;
    width: 100%;
    max-width: 900px;
  }

  .search-input {
    width: 100%;
    max-width: 900px;
    padding: 10px var(--padding);
    font-size: 14px;
    color: var(--secondary);
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    outline: none;
    box-sizing: border-box;
  }

  .search-input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .search-input::placeholder { color: var(--gray); }

  .fallback-banner {
    width: 100%;
    max-width: 900px;
    padding: 8px 14px;
    font-size: 12px;
    color: var(--gray);
    background: color-mix(in srgb, var(--cta) 8%, transparent);
    border-radius: var(--border-radius);
    text-align: center;
  }

  .platform-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--padding);
    width: 100%;
    max-width: 900px;
    justify-items: center;
  }

  .platform-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    gap: calc(var(--padding) * 0.75);
    padding: calc(var(--padding) * 2) var(--padding);
    background: var(--button-elevated);
    border-radius: var(--border-radius);
    cursor: pointer;
    transition: transform 0.15s, background 0.15s;
  }

  @media (hover: hover) {
    .platform-card:hover {
      background: var(--sidebar-highlight);
      transform: translateY(-2px);
    }
  }

  .platform-card:active { transform: translateY(0); }

  .platform-card:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .card-icon {
    width: 52px;
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: calc(var(--border-radius) - 2px);
    background: color-mix(in srgb, var(--platform-color) 15%, transparent);
    color: var(--platform-color);
  }

  .card-name {
    font-size: 14.5px;
    font-weight: 500;
    color: var(--secondary);
  }

  .card-status {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--gray);
    min-height: 16px;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.connected { background: var(--green); }
  .status-dot.disconnected { background: var(--gray); }
  .status-dot.error { background: var(--red); }

  .status-email {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 140px;
  }

  @media (max-width: 535px) {
    .platform-grid { grid-template-columns: repeat(2, 1fr); }
  }

  @media (prefers-reduced-motion: reduce) {
    .platform-card { transition: none; }
  }

  .plugin-guard {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - var(--padding) * 4);
    gap: calc(var(--padding) * 1.5);
    text-align: center;
    color: var(--gray);
  }
  .plugin-guard h2 { font-size: 18px; color: var(--secondary); }
  .plugin-guard p { font-size: 14px; max-width: 300px; }
  .guard-link { padding: 10px 24px; font-size: 14px; font-weight: 500; background: var(--cta); color: var(--on-cta); border-radius: var(--border-radius); text-decoration: none; }
  .guard-detail { font-size: 12px; color: var(--tertiary); max-width: 480px; margin-top: calc(var(--padding) * 0.5); }
  .guard-detail code { font-family: var(--font-mono, ui-monospace, monospace); font-size: 11px; word-break: break-word; background: var(--surface); padding: 2px 6px; border-radius: 4px; }
  .spinner { width: 24px; height: 24px; border: 2px solid var(--input-border); border-top-color: var(--secondary); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
