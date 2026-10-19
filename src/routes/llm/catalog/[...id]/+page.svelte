<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import {
    workspaceRpc,
    t,
    type CatalogItem,
  } from "$lib/engine/workspace-api";
  import {
    ChevronLeft,
    Star,
    Download,
    Check,
    Shield,
    Terminal,
    Copy,
    ExternalLink,
    FileText,
    Settings,
    Tag,
  } from "lucide-svelte";

  let loading = $state(true);
  let pkg = $state<CatalogItem | null>(null);
  let copied = $state(false);

  const pkgId = $derived($page.params.id || "tools/web-browser");

  async function loadPackage() {
    loading = true;
    try {
      const res = await workspaceRpc<CatalogItem>("catalog.get", { id: pkgId });
      pkg = res || null;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadPackage();
  });

  async function toggleInstall() {
    if (!pkg) return;
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

  function copyCli() {
    if (!pkg) return;
    navigator.clipboard.writeText(`omniget pkg install ${pkg.id}`);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="pkg-detail-page">
  <div class="nav-back">
    <a href="/llm/catalog" class="back-link">
      <ChevronLeft size={16} />
      {t("catalog.back", "Back to Catalog")}
    </a>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading package details...")}</p>
    </div>
  {:else if !pkg}
    <div class="error-state">
      <h2>{t("catalog.not_found", "Package not found")}</h2>
      <p>{t("catalog.not_found_hint", "The requested extension or tool could not be loaded from registry.")}</p>
      <a href="/llm/catalog" class="btn btn-primary">{t("catalog.view_catalog", "Browse Catalog")}</a>
    </div>
  {:else}
    <header class="detail-head">
      <div class="head-info">
        <div class="badge-row">
          <span class="category-pill {pkg.category}">{pkg.category}</span>
          <span class="version-pill">v{pkg.version}</span>
          <span class="license-pill">{pkg.license}</span>
        </div>
        <h1 class="pkg-title">{pkg.name}</h1>
        <p class="pkg-subtitle">by <span class="author-name">{pkg.author}</span> &bull; {pkg.description}</p>
      </div>

      <div class="head-cta">
        <button
          type="button"
          class="btn-install-large"
          class:installed={pkg.installed}
          onclick={toggleInstall}
        >
          {#if pkg.installed}
            <Check size={18} />
            {t("catalog.installed_btn", "Installed & Active")}
          {:else}
            <Download size={18} />
            {t("catalog.install_btn", "Install Package")}
          {/if}
        </button>
      </div>
    </header>

    <div class="detail-body">
      <!-- Main Content (Readme & Specs) -->
      <main class="readme-col">
        <section class="card readme-card">
          <div class="card-head">
            <FileText size={16} class="text-blue-400 mr-2 inline" />
            <h2>Documentation & Usage</h2>
          </div>
          <div class="markdown-preview">
            <pre class="readme-text">{pkg.readme}</pre>
          </div>
        </section>

        <!-- Permissions Section -->
        <section class="card permissions-card">
          <div class="card-head">
            <Shield size={16} class="text-orange-400 mr-2 inline" />
            <h2>Permissions & Security Sandbox</h2>
          </div>
          <p class="section-hint">This package requests the following host capabilities:</p>
          <ul class="permission-list">
            {#each pkg.permissions as perm}
              <li class="perm-item">
                <code>{perm}</code>
                <span class="perm-status">sandboxed</span>
              </li>
            {/each}
          </ul>
        </section>
      </main>

      <!-- Sidebar -->
      <aside class="sidebar-col">
        <div class="card cli-card">
          <h3>Terminal Install</h3>
          <div class="cli-box">
            <code>omniget pkg install {pkg.id}</code>
            <button type="button" class="btn-copy" onclick={copyCli} title="Copy command">
              {#if copied}
                <Check size={14} class="text-green-400" />
              {:else}
                <Copy size={14} />
              {/if}
            </button>
          </div>
        </div>

        <div class="card stats-card">
          <h3>Package Telemetry</h3>
          <dl class="stat-list">
            <div class="stat-pair">
              <dt>Stars</dt>
              <dd>
                <Star size={13} class="fill-current text-yellow-400 inline mr-1" />
                {pkg.stars}
              </dd>
            </div>
            <div class="stat-pair">
              <dt>Downloads</dt>
              <dd>{pkg.downloads.toLocaleString()}</dd>
            </div>
            <div class="stat-pair">
              <dt>License</dt>
              <dd>{pkg.license}</dd>
            </div>
            <div class="stat-pair">
              <dt>Updated</dt>
              <dd>{new Date(pkg.updated_at).toLocaleDateString()}</dd>
            </div>
          </dl>
        </div>

        <div class="card tags-card">
          <h3>Tags</h3>
          <div class="tags-container">
            {#each pkg.tags as tag}
              <span class="tag-pill">
                <Tag size={10} class="mr-1 inline" />
                {tag}
              </span>
            {/each}
          </div>
        </div>
      </aside>
    </div>
  {/if}
</div>

<style>
  .pkg-detail-page {
    padding: 24px 32px;
    max-width: 1300px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .nav-back {
    display: flex;
    align-items: center;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted, #8b949e);
    text-decoration: none;
    font-size: 13px;
    transition: color 0.15s ease;
  }

  .back-link:hover {
    color: var(--accent-hi, #58a6ff);
  }

  .detail-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    padding: 24px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    flex-wrap: wrap;
  }

  .badge-row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .category-pill {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
  }
  .category-pill.tools { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .category-pill.skills { background: rgba(63, 185, 80, 0.2); color: #3fb950; }
  .category-pill.agents { background: rgba(163, 113, 247, 0.2); color: #a371f7; }
  .category-pill.models { background: rgba(240, 136, 62, 0.2); color: #f0883e; }

  .version-pill, .license-pill {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .pkg-title {
    font-size: 24px;
    font-weight: 700;
    margin: 0 0 6px;
  }

  .pkg-subtitle {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .author-name {
    color: var(--text, #fff);
    font-weight: 500;
  }

  .btn-install-large {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 24px;
    border-radius: 8px;
    background: var(--cta, #1f6feb);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .btn-install-large:hover {
    background: #388bfd;
  }

  .btn-install-large.installed {
    background: rgba(63, 185, 80, 0.18);
    color: #3fb950;
    border: 1px solid rgba(63, 185, 80, 0.3);
  }

  .detail-body {
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: 20px;
  }

  .card {
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .card-head {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
  }

  .card-head h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .card h3 {
    font-size: 13px;
    font-weight: 600;
    margin: 0 0 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #8b949e);
  }

  .readme-text {
    font-family: var(--font-sans, system-ui);
    white-space: pre-wrap;
    line-height: 1.6;
    font-size: 14px;
    color: var(--text, #e6edf3);
    margin: 0;
  }

  .section-hint {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
    margin: 0 0 12px;
  }

  .permission-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .perm-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--fill-1, rgba(255, 255, 255, 0.04));
    border-radius: 6px;
  }

  .perm-item code {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: #e6edf3;
  }

  .perm-status {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(240, 136, 62, 0.15);
    color: #f0883e;
  }

  .cli-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--surface-mut, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 6px;
  }

  .cli-box code {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: #3fb950;
  }

  .btn-copy {
    border: none;
    background: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .stat-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .stat-pair {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }

  .stat-pair dt {
    color: var(--text-muted, #8b949e);
  }

  .stat-pair dd {
    margin: 0;
    font-weight: 500;
  }

  .tags-container {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag-pill {
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  @media (max-width: 900px) {
    .detail-body {
      grid-template-columns: 1fr;
    }
  }
</style>
