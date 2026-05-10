<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { onClipboardUrl } from "$lib/stores/clipboard-monitor";
  import { getDownloads, type DownloadItem } from "$lib/stores/download-store.svelte";
  import { translateBackendError } from "$lib/error-translate";
  import { t } from "$lib/i18n";
  import OmniboxInput from "$components/omnibox/OmniboxInput.svelte";
  import MediaPreview from "$components/omnibox/MediaPreview.svelte";
  import BatchDownload from "$components/omnibox/BatchDownload.svelte";
  import SearchResults from "$components/omnibox/SearchResults.svelte";
  import { getMediaPreview, clearMediaPreview } from "$lib/stores/media-preview-store.svelte";

  type PlatformInfo = {
    platform: string;
    supported: boolean;
    content_id: string | null;
    content_type: string | null;
  };

  type SearchResult = {
    id: string;
    title: string;
    author: string;
    duration: number | null;
    thumbnail_url: string | null;
    url: string;
    platform: string;
  };

  type DownloadStarted = { id: number; title: string };

  type Root = {
    path: string;
    enabled: boolean;
    exists: boolean;
  };

  type OmniState =
    | { kind: "idle" }
    | { kind: "detecting" }
    | { kind: "detected"; info: PlatformInfo }
    | { kind: "unsupported" }
    | { kind: "preparing"; platform: string }
    | { kind: "batch"; urls: string[] }
    | { kind: "searching" }
    | { kind: "search-results"; results: SearchResult[] }
    | { kind: "search-empty" }
    | { kind: "error"; message: string; originalUrl: string };

  let url = $state("");
  let omniState = $state<OmniState>({ kind: "idle" });
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let quality = $state<"best" | "320" | "256" | "192" | "128">("best");
  let roots = $state<Root[]>([]);
  let targetRoot = $state("");
  let imageLoading = $state(true);
  const mediaPreview = $derived(getMediaPreview());
  const downloads = $derived(getDownloads());

  $effect(() => {
    if (mediaPreview) imageLoading = true;
  });

  const recentDownloads = $derived.by((): DownloadItem[] => {
    const arr = Array.from(downloads.values()) as DownloadItem[];
    return arr
      .filter((d) => d.kind === "generic")
      .sort((a, b) => (b.startedAt ?? 0) - (a.startedAt ?? 0))
      .slice(0, 6);
  });

  let lastCompleteCount = $state(0);
  $effect(() => {
    const completed = recentDownloads.filter((d) => d.status === "complete").length;
    if (completed > lastCompleteCount) {
      lastCompleteCount = completed;
      void pluginInvoke("study", "study:music:scan").catch(() => {});
    } else {
      lastCompleteCount = completed;
    }
  });

  async function loadRoots() {
    try {
      const r = await pluginInvoke<{ roots: Root[] }>(
        "study",
        "study:music:roots:list",
      );
      roots = (r.roots ?? []).filter((x) => x.enabled && x.exists);
      if (roots.length > 0 && !targetRoot) {
        targetRoot = roots[0].path;
      }
    } catch {
      /* ignore */
    }
  }

  function isUrl(value: string): boolean {
    return (
      value.startsWith("http://") ||
      value.startsWith("https://") ||
      value.startsWith("magnet:") ||
      value.endsWith(".torrent")
    );
  }

  function handleInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    clearMediaPreview();

    const trimmed = url.trim();
    if (!trimmed) {
      omniState = { kind: "idle" };
      return;
    }

    const urls = trimmed.split(/[\s\n]+/).filter(isUrl);
    if (urls.length > 1) {
      omniState = { kind: "batch", urls };
      return;
    }

    if (isUrl(trimmed)) {
      omniState = { kind: "detecting" };
      debounceTimer = setTimeout(() => detectPlatform(trimmed), 400);
      return;
    }

    if (trimmed.length >= 2) {
      omniState = { kind: "searching" };
      debounceTimer = setTimeout(() => performSearch(trimmed), 600);
    } else {
      omniState = { kind: "idle" };
    }
  }

  async function detectPlatform(value: string) {
    try {
      const result = await invoke<PlatformInfo>("detect_platform", { url: value });
      if (result.supported) {
        omniState = { kind: "detected", info: result };
        invoke("prefetch_media_info", { url: value }).catch(() => {});
      } else {
        omniState = { kind: "unsupported" };
      }
    } catch {
      omniState = { kind: "unsupported" };
    }
  }

  async function performSearch(query: string) {
    try {
      const results = await invoke<SearchResult[]>("search_videos", {
        query,
        platform: "youtube",
        maxResults: 8,
      });
      if (url.trim() !== query) return;
      omniState = results.length > 0
        ? { kind: "search-results", results }
        : { kind: "search-empty" };
    } catch {
      if (url.trim() === query) omniState = { kind: "search-empty" };
    }
  }

  function selectSearchResult(result: SearchResult) {
    url = result.url;
    omniState = { kind: "detecting" };
    void detectPlatform(result.url);
  }

  async function handleDownload() {
    if (omniState.kind !== "detected") return;
    if (!targetRoot) {
      showToast("error", $t("study.music.ytdlp_no_roots") as string);
      return;
    }
    const info = omniState.info;
    const currentUrl = url.trim();
    omniState = { kind: "preparing", platform: info.platform };
    url = "";
    try {
      await invoke<DownloadStarted>("download_from_url", {
        url: currentUrl,
        outputDir: targetRoot,
        downloadMode: "audio",
        quality,
        formatId: null,
        referer: null,
      });
      omniState = { kind: "idle" };
      showToast(
        "success",
        $t("study.music.dl_started", { title: info.platform }) as string,
      );
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message ?? "";
      omniState = {
        kind: "error",
        message: msg ? translateBackendError(msg, $t) : ($t("omnibox.error") as string),
        originalUrl: currentUrl,
      };
    }
  }

  async function handleBatch() {
    if (omniState.kind !== "batch" || !targetRoot) return;
    const batchUrls = omniState.urls;
    omniState = { kind: "idle" };
    url = "";
    const results = await Promise.allSettled(
      batchUrls.map((u) =>
        invoke<DownloadStarted>("download_from_url", {
          url: u,
          outputDir: targetRoot,
          downloadMode: "audio",
          quality,
          formatId: null,
          referer: null,
        }),
      ),
    );
    const ok = results.filter((r) => r.status === "fulfilled").length;
    const fail = results.length - ok;
    showToast(
      ok > 0 ? "success" : "error",
      $t("study.music.batch_done", { ok, fail }) as string,
    );
  }

  function fmtPct(item: DownloadItem): string {
    if (item.kind === "generic" && item.totalBytes && item.totalBytes > 0) {
      return `${Math.round(((item.downloadedBytes ?? 0) / item.totalBytes) * 100)}%`;
    }
    if (typeof item.percent === "number") {
      return `${Math.round(item.percent)}%`;
    }
    return item.status === "complete" ? "100%" : "—";
  }

  onMount(() => {
    void loadRoots();
    onClipboardUrl((detected) => {
      if (omniState.kind === "preparing") return;
      url = detected;
      handleInput();
      showToast("info", $t("toast.clipboard_url_detected") as string);
    });
    return () => {
      onClipboardUrl(null);
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });
</script>

<section class="page">
  <header class="hero">
    <h1>{$t("study.music.dl_hero_title")}</h1>
    <p>{$t("study.music.dl_hero_sub")}</p>
  </header>

  <div class="omnibar-card">
    <OmniboxInput bind:url onInput={handleInput} />

    <div class="opts">
      <div class="opt">
        <label for="root-select">{$t("study.music.ytdlp_target")}</label>
        <select id="root-select" bind:value={targetRoot}>
          {#if roots.length === 0}
            <option value="">{$t("study.music.ytdlp_no_roots")}</option>
          {:else}
            {#each roots as r (r.path)}
              <option value={r.path}>{r.path}</option>
            {/each}
          {/if}
        </select>
      </div>
      <div class="opt">
        <label for="quality-select">{$t("study.music.dl_quality")}</label>
        <select id="quality-select" bind:value={quality}>
          <option value="best">{$t("study.music.dl_quality_best")}</option>
          <option value="320">320 kbps</option>
          <option value="256">256 kbps</option>
          <option value="192">192 kbps</option>
          <option value="128">128 kbps</option>
        </select>
      </div>
    </div>

    <div class="state-area">
      {#if omniState.kind === "idle"}
        <p class="hint">{$t("study.music.dl_idle_hint")}</p>
      {:else if omniState.kind === "detecting"}
        <p class="hint pulse">{$t("study.music.dl_detecting")}</p>
      {:else if omniState.kind === "detected"}
        <MediaPreview
          {mediaPreview}
          bind:imageLoading
        />
        <button
          type="button"
          class="cta"
          onclick={handleDownload}
          disabled={!targetRoot}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4 M7 10l5 5 5-5 M12 15V3"/>
          </svg>
          {$t("study.music.dl_download_audio")}
        </button>
      {:else if omniState.kind === "unsupported"}
        <p class="warn">{$t("study.music.dl_unsupported")}</p>
      {:else if omniState.kind === "batch"}
        <BatchDownload count={omniState.urls.length} onDownload={handleBatch} />
      {:else if omniState.kind === "searching"}
        <p class="hint pulse">{$t("study.music.dl_searching")}</p>
      {:else if omniState.kind === "search-results"}
        <SearchResults results={omniState.results} onSelect={selectSearchResult} />
      {:else if omniState.kind === "search-empty"}
        <p class="hint">{$t("study.music.dl_search_empty")}</p>
      {:else if omniState.kind === "preparing"}
        <p class="hint pulse">{$t("study.music.dl_preparing")}</p>
      {:else if omniState.kind === "error"}
        <p class="warn">{omniState.message}</p>
      {/if}
    </div>
  </div>

  {#if recentDownloads.length > 0}
    <section class="downloads">
      <h2>{$t("study.music.dl_recent")}</h2>
      <ul class="dl-list">
        {#each recentDownloads as item (item.id)}
          <li class="dl-row" class:done={item.status === "complete"} class:fail={item.status === "error"}>
            <span class="dl-icon" aria-hidden="true">
              {#if item.status === "complete"}✓
              {:else if item.status === "error"}!
              {:else if item.status === "downloading"}
                <span class="spinner-tiny"></span>
              {:else}…
              {/if}
            </span>
            <span class="dl-title">{item.name}</span>
            <span class="dl-pct">{fmtPct(item)}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding-bottom: 60px;
    max-width: 760px;
    margin-inline: auto;
  }
  .hero {
    text-align: center;
    padding: 32px 16px 8px;
  }
  .hero h1 {
    margin: 0 0 6px;
    font-size: 32px;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: rgba(255, 255, 255, 0.95);
  }
  .hero p {
    margin: 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 14px;
  }
  .omnibar-card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 22px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 14px;
  }
  .opts {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
  }
  .opt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 200px;
  }
  .opt label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.45);
  }
  .opt select {
    padding: 8px 12px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    font-size: 13px;
    outline: none;
  }
  .opt select:focus { border-color: var(--accent); }
  .state-area {
    display: flex;
    flex-direction: column;
    gap: 14px;
    align-items: stretch;
    min-height: 56px;
    justify-content: center;
  }
  .hint {
    margin: 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
    text-align: center;
  }
  .hint.pulse {
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.55; }
    50% { opacity: 1; }
  }
  .warn {
    margin: 0;
    padding: 10px 14px;
    color: rgb(251, 191, 36);
    background: rgba(217, 119, 6, 0.1);
    border: 1px solid rgba(217, 119, 6, 0.25);
    border-radius: 8px;
    font-size: 13px;
  }
  .cta {
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: filter 120ms ease;
  }
  .cta:hover:not(:disabled) { filter: brightness(1.08); }
  .cta:disabled { opacity: 0.4; cursor: not-allowed; }
  .downloads h2 {
    margin: 0 0 12px;
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.65);
  }
  .dl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .dl-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.85);
    font-size: 12px;
  }
  .dl-row.done .dl-icon { color: rgb(74, 222, 128); }
  .dl-row.fail { background: rgba(220, 38, 38, 0.06); border-color: rgba(220, 38, 38, 0.2); }
  .dl-row.fail .dl-icon { color: rgb(248, 113, 113); }
  .dl-icon {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
    font-weight: 700;
  }
  .dl-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dl-pct {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    color: rgba(255, 255, 255, 0.5);
  }
  .spinner-tiny {
    width: 10px;
    height: 10px;
    border: 1.5px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
