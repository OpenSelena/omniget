<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { invokeV100 } from "$lib/v100/ipc-bridge";

  interface MediaNode {
    id: string;
    title: string;
    kind: string;
    drm?: boolean;
    note?: string;
    source?: { format?: string; url?: string };
    children?: MediaNode[];
  }

  interface DiscoveryResult {
    tree: MediaNode;
    drm: number;
    connector?: string;
    logs: string[];
    ms: number;
  }

  interface SniffedItem {
    id: string;
    tab_id?: string;
    kind: string;
    url: string;
    size?: number;
  }

  interface DownloadJobItem {
    id: string;
    title: string;
    status: "queued" | "running" | "active" | "done" | "cancelled" | "failed" | "skipped" | "drm" | "handed_off";
    bytes?: number;
    error?: string;
  }

  interface DownloadJob {
    id: string;
    title: string;
    status: string;
    total: number;
    done: number;
    skipped: number;
    drm: number;
    failed: number;
    handed_off: number;
    bytes?: number;
    out_dir: string;
    items: DownloadJobItem[];
  }

  interface BrowserTab {
    id: string;
    title?: string;
    url: string;
  }

  interface BrowserSource {
    id: string;
    name: string;
    running?: boolean;
    profiles?: { dir: string; name: string; email?: string }[];
  }

  interface ConnectorItem {
    manifest: {
      id: string;
      name: string;
      version: string;
      domains: string[];
      options?: any[];
    };
    dev?: boolean;
  }

  let isBrowserOpen = $state(false);
  let browserMode = $state("embedded");
  let browserState = $state("");
  let installedBrowsers = $state<any[]>([]);
  let browserTabs = $state<BrowserTab[]>([]);
  let sniffedMedia = $state<SniffedItem[]>([]);
  let activeJobs = $state<DownloadJob[]>([]);
  let connectors = $state<ConnectorItem[]>([]);
  let outDir = $state("");
  let targetUrl = $state("");
  let advUrl = $state("");
  let activeAction = $state("");
  let errorMessage = $state("");
  let discovered = $state<DiscoveryResult | null>(null);
  let selectedIds = $state<Set<string>>(new Set());
  let collapsedIds = $state<Set<string>>(new Set());
  let targetQuality = $state<number | null>(null);
  let showLogs = $state(false);
  let activeTabFilter = $state<string | null>(null);
  let showLoginCard = $state(false);
  let showImportCard = $state(false);
  let browserSources = $state<BrowserSource[]>([]);
  let selectedSourceId = $state("");
  let selectedProfileDir = $state("");
  let lastImported = $state<any>(null);
  let recentImported = $state<any>(null);
  let activeConnectorId = $state<string | null>(null);

  const isWindowActive = $derived(isBrowserOpen && browserMode === "window");
  const filteredSniffed = $derived(
    activeTabFilter ? sniffedMedia.filter((m) => m.tab_id === activeTabFilter) : sniffedMedia
  );
  const activeSource = $derived(browserSources.find((s) => s.id === selectedSourceId));

  function extractAllIds(node: MediaNode): string[] {
    if (node.children && node.children.length > 0) {
      return node.children.flatMap(extractAllIds);
    }
    return node.source && !node.drm ? [node.id] : [];
  }

  const allLeafIds = $derived(discovered ? extractAllIds(discovered.tree) : []);

  function formatBytes(bytes?: number): string {
    if (!bytes) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = bytes;
    let idx = 0;
    while (size >= 1024 && idx < units.length - 1) {
      size /= 1024;
      idx++;
    }
    return `${size.toFixed(size >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`;
  }

  function normalizeUrl(u: string): string {
    const trimmed = u.trim();
    if (!trimmed) return "";
    if (/^[a-z]+:\/\//i.test(trimmed) || trimmed.startsWith("about:")) return trimmed;
    return `https://${trimmed}`;
  }

  function formatHostname(u: string): string {
    try {
      return new URL(u).host;
    } catch {
      return u;
    }
  }

  function formatCompactPath(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : path;
  }

  async function refreshStatus() {
    try {
      const res = await invokeV100("gd_status", {});
      if (res && res.browser) {
        isBrowserOpen = !!res.browser.open;
        browserMode = res.browser.mode ?? "embedded";
        installedBrowsers = res.browser.installed ?? [];
        browserTabs = res.browser.tabs ?? [];
        connectors = res.connectors ?? [];
        activeJobs = res.jobs ?? [];
        if (!outDir) outDir = res.out_dir ?? "";
      }
      const sniff = await invokeV100("gd_sniffed", { tabId: null });
      sniffedMedia = Array.isArray(sniff) ? sniff : [];
    } catch {}
  }

  async function executeAction(name: string, fn: () => Promise<any>) {
    activeAction = name;
    errorMessage = "";
    try {
      return await fn();
    } catch (err: any) {
      errorMessage = typeof err === "string" ? err : err?.message ?? String(err);
    } finally {
      activeAction = "";
    }
  }

  async function startFind(reload = false) {
    const url = normalizeUrl(targetUrl);
    if (!url) return;
    discovered = null;
    const res = await executeAction("find", () =>
      invokeV100("gd_find", { url, reload })
    );
    if (res && res.tree) {
      discovered = res;
      selectedIds = new Set(extractAllIds(res.tree));
      collapsedIds = new Set();
      showLogs = false;
    }
    await refreshStatus();
  }

  async function openSiteBrowser() {
    const url = normalizeUrl(targetUrl);
    if (!url) return;
    showImportCard = false;
    await executeAction("window", () =>
      invokeV100("gd_browser_show", { url })
    );
    if (!errorMessage) showLoginCard = true;
  }

  async function hideSiteBrowser() {
    await executeAction("window", () =>
      invokeV100("gd_browser_hide", {})
    );
    showLoginCard = false;
  }

  async function openImportDialog() {
    if (showImportCard) {
      showImportCard = false;
      return;
    }
    showLoginCard = false;
    const res = await executeAction("import", () =>
      invokeV100("gd_browser_sources", {})
    );
    if (res) {
      browserSources = res.sources ?? [];
      lastImported = res.imported?.from ? res.imported : null;
      selectedSourceId = browserSources[0]?.id ?? "";
      selectedProfileDir = browserSources[0]?.profiles?.[0]?.dir ?? "";
      showImportCard = true;
    }
  }

  async function importCookies() {
    if (!selectedSourceId) return;
    const res = await executeAction("import", () =>
      invokeV100("gd_browser_import", {
        source: selectedSourceId,
        profile: selectedProfileDir || null
      })
    );
    if (res) {
      recentImported = res;
      lastImported = res;
      await refreshStatus();
    }
  }

  async function downloadSelected() {
    const current = discovered;
    if (!current || selectedIds.size === 0) return;
    await executeAction("download", async () => {
      await invokeV100("gd_download", {
        tree: current.tree,
        selected: [...selectedIds],
        outDir,
        maxHeight: targetQuality
      });
      await refreshStatus();
    });
  }

  async function quickDownloadSniffed(item: SniffedItem) {
    await executeAction("download", async () => {
      await invokeV100("gd_download_sniffed", { id: item.id, outDir });
      await refreshStatus();
    });
  }

  async function chooseOutDirectory() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, defaultPath: outDir || undefined });
      if (typeof selected === "string") outDir = selected;
    } catch {}
  }

  async function openPathFolder(path: string) {
    try {
      await invokeV100("open_path_default", { path });
    } catch {
      await invokeV100("reveal_file", { path });
    }
  }

  function toggleSelectNode(node: MediaNode) {
    const leafIds = extractAllIds(node);
    const next = new Set(selectedIds);
    const allSelected = leafIds.every((id) => next.has(id));
    for (const id of leafIds) {
      if (allSelected) next.delete(id);
      else next.add(id);
    }
    selectedIds = next;
  }

  function toggleCollapseNode(id: string) {
    const next = new Set(collapsedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsedIds = next;
  }

  function selectAll() {
    selectedIds = new Set(allLeafIds);
  }

  function selectNone() {
    selectedIds = new Set();
  }

  onMount(() => {
    refreshStatus();
    let unlisten: (() => void) | null = null;

    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<any>("global-dl:event", (event) => {
        const payload = event.payload;
        if (payload?.kind === "browser") {
          browserState = payload.state;
          if (payload.state === "open" || payload.state === "closed") refreshStatus();
        } else if (payload?.kind === "tabs") {
          browserTabs = payload.tabs ?? [];
          isBrowserOpen = true;
        } else if (payload?.kind === "sniffed") {
          sniffedMedia = [...sniffedMedia, payload.item].slice(-600);
        } else if (payload?.kind === "job") {
          const idx = activeJobs.findIndex((j) => j.id === payload.job?.id);
          if (idx < 0) activeJobs = [...activeJobs, payload.job];
          else activeJobs = activeJobs.map((j) => (j.id === payload.job.id ? payload.job : j));
        }
      }).then((un) => {
        unlisten = un;
      });
    }).catch(() => {});

    return () => {
      unlisten?.();
    };
  });
</script>

<div class="gd">
  <!-- Header -->
  <header class="gd-head">
    <div class="gd-icon">
      <svg viewBox="0 0 24 24" width="38" height="38" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10" />
        <line x1="2" y1="12" x2="22" y2="12" />
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
      </svg>
    </div>
    <div class="gd-head-copy">
      <h1>{$t("global_dl.title") || "Universal Media Downloader"}</h1>
      <p>{$t("global_dl.subtitle") || "Deep media extraction, web sniffers, and authenticated streams."}</p>
    </div>
    <div class="gd-head-actions">
      <span class="pill" class:pill-on={isBrowserOpen}>
        {isBrowserOpen ? ($t("global_dl.browser_open") || "Browser Active") : ($t("global_dl.browser_closed") || "Browser Idle")}
      </span>
      {#if isWindowActive}
        <button type="button" class="btn btn-secondary btn-sm" onclick={hideSiteBrowser}>
          {$t("global_dl.hide_window") || "Hide Window"}
        </button>
      {/if}
    </div>
  </header>

  <!-- Hero Input Form -->
  <form class="hero card" onsubmit={(e) => { e.preventDefault(); startFind(); }}>
    <label for="gd-url" class="hero-label">
      {$t("global_dl.hero_label") || "Media Page URL"}
    </label>
    <div class="hero-row">
      <input
        id="gd-url"
        type="text"
        inputmode="url"
        spellcheck="false"
        autocomplete="off"
        placeholder={$t("global_dl.url_placeholder") || "Paste video link, course portal, or livestream URL…"}
        bind:value={targetUrl}
      />
      <button
        class="btn btn-primary btn-lg"
        type="submit"
        disabled={!targetUrl.trim() || activeAction === "find"}
      >
        {activeAction === "find" ? ($t("global_dl.finding") || "Searching…") : ($t("global_dl.find") || "Find Media")}
      </button>
    </div>

    <!-- Secondary Helper Actions -->
    <div class="hero-secondary">
      <button
        class="btn btn-ghost btn-sm"
        type="button"
        disabled={!targetUrl.trim() || activeAction === "window"}
        onclick={openSiteBrowser}
      >
        {$t("global_dl.login_site") || "Log In to Site"}
      </button>
      <button
        class="btn btn-ghost btn-sm"
        type="button"
        disabled={activeAction === "import"}
        onclick={openImportDialog}
      >
        {$t("global_dl.import_logins") || "Import Browser Cookies"}
      </button>
    </div>
  </form>

  <!-- Login Modal / Callout -->
  {#if showLoginCard}
    <section class="card callout" aria-live="polite">
      <h2>{$t("global_dl.login_title") || "Log In in Browser Window"}</h2>
      <p>{$t("global_dl.login_hint") || "Sign in to the site in the opened browser window, then click Done to refresh media detection."}</p>
      <div class="row">
        <button
          type="button"
          class="btn btn-primary"
          disabled={activeAction === "find"}
          onclick={() => { showLoginCard = false; startFind(true); }}
        >
          {$t("global_dl.login_done") || "Done, Find Media"}
        </button>
        <button type="button" class="btn btn-ghost" onclick={hideSiteBrowser}>
          {$t("global_dl.hide_window") || "Hide Window"}
        </button>
      </div>
    </section>
  {/if}

  <!-- Import Cookies Modal / Callout -->
  {#if showImportCard}
    <section class="card callout">
      <h2>{$t("global_dl.import_title") || "Import Logins & Cookies"}</h2>
      <p>{$t("global_dl.import_hint") || "Import session cookies directly from your installed desktop browser to bypass logins."}</p>

      {#if browserSources.length === 0}
        <p class="empty">{$t("global_dl.import_none") || "No browser profiles detected on this machine."}</p>
      {:else}
        <div class="row">
          <label class="field">
            <span>{$t("global_dl.import_browser") || "Browser"}</span>
            <select bind:value={selectedSourceId}>
              {#each browserSources as src (src.id)}
                <option value={src.id}>{src.name}</option>
              {/each}
            </select>
          </label>

          {#if activeSource && activeSource.profiles && activeSource.profiles.length > 1}
            <label class="field">
              <span>{$t("global_dl.import_profile") || "Profile"}</span>
              <select bind:value={selectedProfileDir}>
                {#each activeSource.profiles as p (p.dir)}
                  <option value={p.dir}>{p.email ? `${p.name} (${p.email})` : p.name}</option>
                {/each}
              </select>
            </label>
          {/if}

          <button
            type="button"
            class="btn btn-primary"
            disabled={activeAction === "import"}
            onclick={importCookies}
          >
            {$t("global_dl.import_button") || "Import"}
          </button>
          <button type="button" class="btn btn-ghost" onclick={() => (showImportCard = false)}>
            {$t("global_dl.close") || "Close"}
          </button>
        </div>
      {/if}

      {#if recentImported}
        <p class="ok">
          {$t("global_dl.import_done", { from: recentImported.from, profile: recentImported.profile }) || `Imported session from ${recentImported.from}!`}
        </p>
      {:else if lastImported}
        <p class="hint">
          {$t("global_dl.import_last", { from: lastImported.from, profile: lastImported.profile }) || `Last imported from ${lastImported.from}`}
        </p>
      {/if}
    </section>
  {/if}

  <!-- Error Alert -->
  {#if errorMessage}
    <section class="card error" role="alert">
      <h2>Detection Error</h2>
      <p>{errorMessage}</p>
      <div class="row">
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => startFind(true)}>
          {$t("global_dl.retry") || "Retry"}
        </button>
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => (errorMessage = "")}>
          {$t("global_dl.close") || "Dismiss"}
        </button>
      </div>
    </section>
  {/if}

  <!-- Discovered Media Tree -->
  {#if discovered}
    <section class="card card-tree">
      <header class="card-head">
        <div class="tree-head">
          <h2>{discovered.tree.title || "Detected Streams"}</h2>
          <span class="note">
            {discovered.connector ? `via ${discovered.connector}` : "via page extraction"} · {allLeafIds.length} items found
          </span>
        </div>
        {#if allLeafIds.length > 1}
          <div class="row">
            <button type="button" class="btn btn-ghost btn-sm" onclick={selectAll}>
              {$t("global_dl.select_all") || "Select All"}
            </button>
            <button type="button" class="btn btn-ghost btn-sm" onclick={selectNone}>
              {$t("global_dl.select_none") || "Select None"}
            </button>
          </div>
        {/if}
      </header>

      <!-- Media Items List -->
      <ul class="tree">
        {#each discovered.tree.children ?? [discovered.tree] as node (node.id)}
          <li class="node-row" class:drm={node.drm}>
            {#if node.children && node.children.length > 0}
              <button
                type="button"
                class="twisty"
                onclick={() => toggleCollapseNode(node.id)}
              >
                {collapsedIds.has(node.id) ? "▸" : "▾"}
              </button>
            {/if}
            <label class="node-label">
              <input
                type="checkbox"
                checked={extractAllIds(node).every((id) => selectedIds.has(id))}
                disabled={node.drm}
                onchange={() => toggleSelectNode(node)}
              />
              <span class="kind kind-{node.kind}">{node.kind}</span>
              <span class="node-title">{node.title}</span>
            </label>
            {#if node.drm}
              <span class="badge-warn" title="Encrypted DRM content">DRM</span>
            {/if}
          </li>
        {/each}
      </ul>

      <!-- Tree Footer Actions -->
      <footer class="tree-foot">
        <div class="field-inline">
          <span class="note">Folder:</span>
          <button type="button" class="btn btn-ghost btn-sm path" onclick={chooseOutDirectory}>
            {formatCompactPath(outDir) || "Default Downloads"}
          </button>
        </div>

        <label class="field-inline">
          <span class="note">Quality:</span>
          <select bind:value={targetQuality}>
            <option value={null}>Best</option>
            <option value={1080}>1080p</option>
            <option value={720}>720p</option>
            <option value={480}>480p</option>
          </select>
        </label>

        <button
          type="button"
          class="btn btn-primary"
          disabled={selectedIds.size === 0 || activeAction === "download"}
          onclick={downloadSelected}
        >
          {activeAction === "download" ? "Starting…" : `Download Selected (${selectedIds.size})`}
        </button>
      </footer>
    </section>
  {/if}

  <!-- Active Jobs Progress -->
  {#if activeJobs.length > 0}
    <section class="card">
      <header class="card-head">
        <h2>Active Downloads ({activeJobs.length})</h2>
      </header>
      <ul class="jobs">
        {#each [...activeJobs].reverse() as job (job.id)}
          <li class="job">
            <div class="job-row">
              <span class="tab-title">{job.title}</span>
              <span class="pill" class:pill-on={job.status === "done"}>{job.status}</span>
              <span class="note">{formatBytes(job.bytes)}</span>
            </div>
            <progress max={job.total} value={job.done + job.skipped}></progress>
            <div class="row">
              <span class="note">Done: {job.done} / {job.total}</span>
              {#if job.out_dir}
                <button
                  type="button"
                  class="btn btn-ghost btn-sm"
                  onclick={() => openPathFolder(job.out_dir)}
                >
                  Open Folder
                </button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <!-- Advanced Panel Collapsible -->
  <details class="advanced card">
    <summary>
      <span class="adv-title">{$t("global_dl.advanced") || "Advanced Sniffing & Connectors"}</span>
      <span class="note">Inspect active network streams and tabs</span>
    </summary>
    <div class="adv-body">
      <!-- Embedded Browser Controls -->
      <div class="row">
        <span>Browser: <strong>{isBrowserOpen ? "Running" : "Stopped"}</strong></span>
        {#if isBrowserOpen}
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            onclick={() => invokeV100("gd_browser_stop", {}).then(refreshStatus)}
          >
            Close Browser
          </button>
        {:else}
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            onclick={() => invokeV100("gd_browser_start", {}).then(refreshStatus)}
          >
            Start Embedded Browser
          </button>
        {/if}
      </div>

      <!-- Sniffed Network Streams -->
      <section class="card">
        <header class="card-head">
          <h2>Network Sniffer Streams ({filteredSniffed.length})</h2>
          <button
            type="button"
            class="btn btn-ghost btn-sm"
            onclick={() => invokeV100("gd_sniffed_clear", {}).then(refreshStatus)}
          >
            Clear Sniffer
          </button>
        </header>

        {#if filteredSniffed.length === 0}
          <p class="empty">No active network streams detected yet.</p>
        {:else}
          <ul class="sniffed">
            {#each [...filteredSniffed].reverse().slice(0, 30) as item (item.id)}
              <li>
                <span class="kind kind-{item.kind}">{item.kind}</span>
                <span class="sniff-url" title={item.url}>{item.url}</span>
                <span class="size">{formatBytes(item.size)}</span>
                <button
                  type="button"
                  class="btn btn-secondary btn-sm"
                  onclick={() => quickDownloadSniffed(item)}
                >
                  Download
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  </details>
</div>

<style>
  .gd {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    max-width: 1100px;
    margin: 0 auto;
    color: var(--text, #f5f5f7);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
  }

  .gd-head {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .gd-icon {
    display: grid;
    place-items: center;
    color: var(--accent, #6366f1);
  }
  .gd-head-copy {
    flex: 1;
    min-width: 0;
  }
  .gd-head h1 {
    font-size: 24px;
    font-weight: 700;
    margin: 0;
    letter-spacing: -0.02em;
  }
  .gd-head p {
    margin: 4px 0 0;
    color: var(--text-dim, #98989d);
    font-size: 13px;
  }
  .gd-head-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .pill {
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-dim, #98989d);
  }
  .pill-on {
    background: rgba(52, 199, 89, 0.2);
    color: #34c759;
  }

  .card {
    background: var(--surface, #1c1c1e);
    border: 1px solid var(--border, rgba(255, 255, 255, 0.08));
    border-radius: 14px;
    padding: 16px 20px;
  }

  .hero {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .hero-label {
    font-size: 14px;
    font-weight: 600;
  }
  .hero-row {
    display: flex;
    gap: 8px;
  }
  .hero-row input {
    flex: 1;
    min-width: 0;
    height: 42px;
    padding: 0 14px;
    border-radius: 10px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    background: rgba(255, 255, 255, 0.05);
    color: inherit;
    font-size: 14px;
    outline: none;
  }
  .hero-row input:focus {
    border-color: var(--accent, #6366f1);
  }

  .hero-secondary {
    display: flex;
    gap: 8px;
  }

  .btn {
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s ease;
  }
  .btn:hover { filter: brightness(1.1); }
  .btn:disabled { opacity: 0.5; cursor: default; }

  .btn-primary {
    background: var(--accent, #6366f1);
    color: #fff;
  }
  .btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: inherit;
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-dim, #98989d);
  }
  .btn-ghost:hover {
    color: var(--text, #f5f5f7);
    background: rgba(255, 255, 255, 0.06);
  }
  .btn-sm { padding: 4px 10px; font-size: 11.5px; }
  .btn-lg { padding: 10px 22px; font-size: 14px; }

  .callout {
    background: rgba(99, 102, 241, 0.06);
    border-color: rgba(99, 102, 241, 0.2);
  }
  .callout h2 {
    font-size: 15px;
    margin: 0 0 4px;
  }
  .callout p {
    font-size: 13px;
    color: var(--text-dim, #98989d);
    margin: 0 0 12px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--text-dim, #98989d);
  }
  select {
    background: var(--surface, #1c1c1e);
    color: inherit;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 12.5px;
  }

  .ok { color: #34c759; font-size: 12.5px; }
  .hint { color: var(--text-dim, #98989d); font-size: 12px; }

  .error {
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.25);
  }
  .error h2 { font-size: 14px; margin: 0 0 4px; color: #ef4444; }
  .error p { margin: 0 0 10px; font-size: 12.5px; }

  .card-tree {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .card-head h2 {
    font-size: 15px;
    margin: 0;
  }
  .tree-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .note {
    font-size: 11.5px;
    color: var(--text-dim, #98989d);
  }

  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 380px;
    overflow-y: auto;
  }
  .node-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
  }
  .node-row.drm { opacity: 0.5; }
  .node-label {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    cursor: pointer;
  }
  .node-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }
  .twisty {
    background: none;
    border: none;
    color: var(--text-dim, #98989d);
    cursor: pointer;
  }
  .badge-warn {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .kind {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-dim, #98989d);
  }
  .kind-video, .kind-hls { background: rgba(99, 102, 241, 0.2); color: #818cf8; }
  .kind-audio { background: rgba(52, 199, 89, 0.2); color: #34c759; }

  .tree-foot {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border, rgba(255, 255, 255, 0.08));
  }
  .field-inline {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .path {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .jobs {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .job {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
  }
  .job-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .tab-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 13px;
  }
  progress {
    width: 100%;
    height: 6px;
    border-radius: 3px;
  }

  .advanced summary {
    cursor: pointer;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .adv-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 16px;
  }
  .sniffed {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 280px;
    overflow-y: auto;
  }
  .sniffed li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    padding: 4px 6px;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 6px;
  }
  .sniff-url {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, monospace;
  }
  .size {
    color: var(--text-dim, #98989d);
  }
  .empty {
    color: var(--text-dim, #98989d);
    font-size: 12.5px;
    margin: 8px 0;
  }
</style>
