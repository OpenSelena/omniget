<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type CatalogItem,
  } from "$lib/v100/workspace-api";
  import {
    Layers,
    Search,
    Trash2,
    Settings,
    CheckCircle2,
    XCircle,
    Plus,
    RefreshCw,
    ExternalLink,
    Shield,
  } from "lucide-svelte";

  let loading = $state(true);
  let packages = $state<CatalogItem[]>([]);
  let searchQuery = $state("");

  async function loadInstalled() {
    loading = true;
    try {
      const res = await workspaceRpc<CatalogItem[]>("catalog.list");
      packages = (Array.isArray(res) ? res : []).filter((p) => p.installed);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadInstalled();
  });

  const filtered = $derived(
    packages.filter((p) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        p.name.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q)
      );
    })
  );

  async function toggleEnable(pkg: CatalogItem) {
    pkg.enabled = !pkg.enabled;
    await workspaceRpc("catalog.toggle", { id: pkg.id });
  }

  async function uninstall(pkg: CatalogItem) {
    if (confirm(`Uninstall ${pkg.name}?`)) {
      await workspaceRpc("catalog.uninstall", { id: pkg.id });
      packages = packages.filter((p) => p.id !== pkg.id);
    }
  }
</script>

<div class="installed-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("installed.title", "Installed Extensions & Packages")}</h1>
      <p class="page-subtitle">
        {t("installed.subtitle", "Manage installed tools, autonomous agent skills, and active sandbox configurations.")}
      </p>
    </div>
    <div class="head-actions">
      <a href="/llm/catalog" class="btn btn-primary">
        <Plus size={15} class="mr-1 inline" />
        {t("installed.explore_catalog", "Install More from Catalog")}
      </a>
    </div>
  </header>

  <!-- Filter & Search -->
  <div class="toolbar">
    <div class="search-box">
      <Search size={16} class="search-icon" />
      <input
        type="search"
        placeholder={t("installed.search", "Search installed extensions...")}
        bind:value={searchQuery}
        class="search-input"
      />
    </div>
    <div class="count-badge">
      <span>{filtered.length} installed</span>
    </div>
  </div>

  <!-- Package List -->
  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading installed packages...")}</p>
    </div>
  {:else if filtered.length === 0}
    <div class="empty-state">
      <Layers size={44} class="text-muted" />
      <h3>{t("installed.none", "No packages installed")}</h3>
      <p class="text-muted">{t("installed.none_hint", "Browse the catalog to add tools, skills, and agents.")}</p>
      <a href="/llm/catalog" class="btn btn-primary mt-3">{t("catalog.browse", "Browse Catalog")}</a>
    </div>
  {:else}
    <div class="pkg-list">
      {#each filtered as pkg (pkg.id)}
        <div class="pkg-row" class:disabled={!pkg.enabled}>
          <div class="pkg-info">
            <div class="pkg-title-line">
              <span class="cat-pill {pkg.category}">{pkg.category}</span>
              <a href="/llm/catalog/{pkg.id}" class="pkg-name">{pkg.name}</a>
              <span class="ver">v{pkg.version}</span>
            </div>
            <p class="pkg-desc">{pkg.description}</p>
            <div class="perm-chips">
              {#each pkg.permissions as perm}
                <span class="chip"><code>{perm}</code></span>
              {/each}
            </div>
          </div>

          <div class="pkg-controls">
            <!-- Toggle Enable Switch -->
            <button
              type="button"
              class="switch-btn"
              class:on={pkg.enabled}
              role="switch"
              aria-checked={pkg.enabled}
              onclick={() => toggleEnable(pkg)}
              title={pkg.enabled ? "Disable extension" : "Enable extension"}
            >
              <span class="switch-dot"></span>
            </button>

            <!-- Uninstall -->
            <button
              type="button"
              class="btn-icon danger"
              onclick={() => uninstall(pkg)}
              title="Uninstall package"
            >
              <Trash2 size={16} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .installed-page {
    padding: 24px 32px;
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .page-title {
    font-size: 22px;
    font-weight: 700;
    margin: 0 0 4px;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    text-decoration: none;
    border: none;
  }

  .btn-primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn-primary:hover {
    background: #388bfd;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    padding: 10px 16px;
  }

  .search-box {
    position: relative;
    flex: 1;
    max-width: 400px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-input {
    width: 100%;
    padding: 6px 12px 6px 32px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .count-badge {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
  }

  .pkg-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .pkg-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    transition: opacity 0.15s ease, border-color 0.15s ease;
  }

  .pkg-row.disabled {
    opacity: 0.6;
  }

  .pkg-info {
    flex: 1;
    min-width: 0;
  }

  .pkg-title-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .cat-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .cat-pill.tools { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .cat-pill.skills { background: rgba(63, 185, 80, 0.2); color: #3fb950; }
  .cat-pill.agents { background: rgba(163, 113, 247, 0.2); color: #a371f7; }

  .pkg-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text, #fff);
    text-decoration: none;
  }

  .pkg-name:hover {
    color: var(--accent-hi, #58a6ff);
  }

  .ver {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .pkg-desc {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 4px 0 8px;
  }

  .perm-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chip code {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-muted, #8b949e);
  }

  .pkg-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .switch-btn {
    width: 44px;
    height: 24px;
    border-radius: 12px;
    background: var(--fill-2, rgba(255, 255, 255, 0.15));
    border: none;
    cursor: pointer;
    position: relative;
    padding: 2px;
    transition: background 0.2s ease;
  }

  .switch-btn.on {
    background: #3fb950;
  }

  .switch-dot {
    display: block;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.2s ease;
  }

  .switch-btn.on .switch-dot {
    transform: translateX(20px);
  }

  .btn-icon {
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.06));
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .btn-icon.danger:hover {
    color: #f85149;
    background: rgba(248, 81, 73, 0.15);
  }

  .loading-state, .empty-state {
    text-align: center;
    padding: 60px 20px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: var(--accent, #58a6ff);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 12px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
