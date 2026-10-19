<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type CatalogItem,
  } from "$lib/engine/workspace-api";
  import {
    Search,
    Star,
    Download,
    Check,
    Plus,
    Wand2,
    SlidersHorizontal,
    ExternalLink,
    Shield,
    Layers,
    Tag,
  } from "lucide-svelte";

  let loading = $state(true);
  let packages = $state<CatalogItem[]>([]);
  let searchQuery = $state("");
  let selectedCategory = $state<string>("all");
  let sortBy = $state<"popular" | "stars" | "recent">("popular");

  const categories = [
    { id: "all", label: "All Packages" },
    { id: "tools", label: "Tools" },
    { id: "skills", label: "Skills" },
    { id: "agents", label: "Agents" },
    { id: "models", label: "Models" },
    { id: "mcp", label: "MCP" },
  ];

  async function loadCatalog() {
    loading = true;
    try {
      const res = await workspaceRpc<CatalogItem[]>("catalog.list");
      packages = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadCatalog();
  });

  const filteredPackages = $derived(
    packages
      .filter((pkg) => {
        if (selectedCategory !== "all" && pkg.category !== selectedCategory) {
          return false;
        }
        if (!searchQuery.trim()) return true;
        const q = searchQuery.toLowerCase();
        return (
          pkg.name.toLowerCase().includes(q) ||
          pkg.description.toLowerCase().includes(q) ||
          pkg.tags.some((tg) => tg.toLowerCase().includes(q)) ||
          pkg.author.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        if (sortBy === "stars") return b.stars - a.stars;
        if (sortBy === "recent") return b.updated_at - a.updated_at;
        return b.downloads - a.downloads;
      })
  );

  async function toggleInstall(pkg: CatalogItem) {
    if (pkg.installed) {
      await workspaceRpc("catalog.uninstall", { id: pkg.id });
      pkg.installed = false;
      pkg.enabled = false;
    } else {
      await workspaceRpc("catalog.install", { id: pkg.id });
      pkg.installed = true;
      pkg.enabled = true;
    }
  }
</script>

<div class="catalog-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("catalog.title", "Extension & Agent Catalog")}</h1>
      <p class="page-subtitle">
        {t("catalog.subtitle", "Discover autonomous agent skills, tool protocols, and community extensions.")}
      </p>
    </div>
    <div class="head-actions">
      <a href="/llm/catalog/wizard" class="btn btn-primary">
        <Wand2 size={15} class="mr-1 inline" />
        {t("catalog.create_wizard", "Publish Wizard")}
      </a>
      <a href="/llm/installed" class="btn btn-secondary">
        <Layers size={15} class="mr-1 inline" />
        {t("catalog.view_installed", "Manage Installed")}
      </a>
    </div>
  </header>

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="search-box">
      <Search size={16} class="search-icon" />
      <input
        type="search"
        placeholder={t("catalog.search_placeholder", "Search skills, tools, MCP servers, or tags...")}
        bind:value={searchQuery}
        class="search-input"
      />
    </div>

    <!-- Category Pills -->
    <div class="category-pills" role="tablist">
      {#each categories as cat}
        <button
          type="button"
          class="pill-btn"
          class:active={selectedCategory === cat.id}
          onclick={() => (selectedCategory = cat.id)}
        >
          {cat.label}
        </button>
      {/each}
    </div>

    <!-- Sorting -->
    <div class="sort-box">
      <select bind:value={sortBy} class="sort-select">
        <option value="popular">{t("catalog.sort.popular", "Most Downloads")}</option>
        <option value="stars">{t("catalog.sort.stars", "Top Starred")}</option>
        <option value="recent">{t("catalog.sort.recent", "Recently Updated")}</option>
      </select>
    </div>
  </div>

  <!-- Catalog Grid -->
  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading catalog registry...")}</p>
    </div>
  {:else if filteredPackages.length === 0}
    <div class="empty-state">
      <Layers size={48} class="text-muted" />
      <h3>{t("catalog.no_results", "No packages matched your query")}</h3>
      <p class="text-muted">{t("catalog.no_results_hint", "Try clearing search filters or check your spelling.")}</p>
    </div>
  {:else}
    <div class="pkg-grid">
      {#each filteredPackages as pkg (pkg.id)}
        <article class="pkg-card">
          <div class="card-head">
            <div class="title-group">
              <span class="category-badge {pkg.category}">{pkg.category}</span>
              <h2 class="pkg-title">
                <a href="/llm/catalog/{pkg.id}">{pkg.name}</a>
              </h2>
              <span class="author">by {pkg.author}</span>
            </div>
            <div class="stars-counter">
              <Star size={13} class="fill-current text-yellow-400" />
              <span>{pkg.stars}</span>
            </div>
          </div>

          <p class="pkg-desc">{pkg.description}</p>

          <div class="tag-list">
            {#each pkg.tags as tag}
              <span class="chip">
                <Tag size={10} class="opacity-70 mr-0.5 inline" />
                {tag}
              </span>
            {/each}
          </div>

          <div class="card-footer">
            <div class="meta-row">
              <span class="ver">v{pkg.version}</span>
              <span class="lic">{pkg.license}</span>
              <span class="dls">{pkg.downloads.toLocaleString()} dls</span>
            </div>

            <div class="card-actions">
              <a href="/llm/catalog/{pkg.id}" class="btn-detail">
                {t("catalog.inspect", "Details")}
              </a>
              <button
                type="button"
                class="btn-install"
                class:installed={pkg.installed}
                onclick={() => toggleInstall(pkg)}
              >
                {#if pkg.installed}
                  <Check size={14} />
                  {t("catalog.installed", "Installed")}
                {:else}
                  <Download size={14} />
                  {t("catalog.install", "Install")}
                {/if}
              </button>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .catalog-page {
    padding: 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 1400px;
    margin: 0 auto;
    width: 100%;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
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

  .head-actions {
    display: flex;
    gap: 8px;
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
    transition: background 0.15s ease;
  }

  .btn-primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn-primary:hover {
    background: #388bfd;
  }

  .btn-secondary {
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: var(--text, #e6edf3);
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .controls-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    background: var(--surface, rgba(22, 27, 34, 0.6));
    padding: 12px 16px;
    border-radius: 12px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
  }

  .search-box {
    position: relative;
    flex: 1;
    min-width: 240px;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-input {
    width: 100%;
    padding: 8px 12px 8px 36px;
    border-radius: 8px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .search-input:focus {
    border-color: var(--accent, #58a6ff);
  }

  .category-pills {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .pill-btn {
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 12px;
    border: none;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .pill-btn:hover {
    color: var(--text, #fff);
    background: rgba(255, 255, 255, 0.1);
  }

  .pill-btn.active {
    background: var(--accent, #1f6feb);
    color: #fff;
    font-weight: 600;
  }

  .sort-select {
    padding: 7px 12px;
    border-radius: 8px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 12px;
    outline: none;
    cursor: pointer;
  }

  .pkg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }

  .pkg-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 18px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .pkg-card:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 8px;
  }

  .category-badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }

  .category-badge.tools { background: rgba(56, 139, 253, 0.18); color: #58a6ff; }
  .category-badge.skills { background: rgba(63, 185, 80, 0.18); color: #3fb950; }
  .category-badge.agents { background: rgba(163, 113, 247, 0.18); color: #a371f7; }
  .category-badge.models { background: rgba(240, 136, 62, 0.18); color: #f0883e; }
  .category-badge.mcp { background: rgba(219, 97, 162, 0.18); color: #db61a2; }

  .pkg-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .pkg-title a {
    color: var(--text, #fff);
    text-decoration: none;
  }

  .pkg-title a:hover {
    color: var(--accent-hi, #58a6ff);
  }

  .author {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .stars-counter {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted, #8b949e);
  }

  .pkg-desc {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 8px 0 12px;
    line-height: 1.5;
    flex: 1;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 14px;
  }

  .chip {
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 12px;
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    font-size: 12px;
  }

  .meta-row {
    display: flex;
    gap: 8px;
    color: var(--text-muted, #8b949e);
    font-size: 11px;
  }

  .card-actions {
    display: flex;
    gap: 6px;
  }

  .btn-detail {
    padding: 5px 10px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.06));
    color: var(--text, #fff);
    text-decoration: none;
    font-size: 12px;
  }

  .btn-detail:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .btn-install {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 12px;
    border-radius: 6px;
    background: var(--cta, #1f6feb);
    color: #fff;
    border: none;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .btn-install:hover {
    background: #388bfd;
  }

  .btn-install.installed {
    background: rgba(63, 185, 80, 0.18);
    color: #3fb950;
    border: 1px solid rgba(63, 185, 80, 0.3);
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
