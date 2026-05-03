<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import {
    studyDownloadsList,
    studyDownloadsUpdate,
    studyDownloadsRemove,
    studyDownloadsClearFinished,
    studyIdForTelegramId,
    studyIdForBatchItem,
    studyIdForUploadId,
    clearLiveDownload,
    clearLiveUpload,
    studyReadRegisterFile,
    type StudyDownloadRow,
  } from "$lib/study-telegram-bridge";

  let { open = $bindable(false), activeCount = $bindable(0) } = $props<{
    open?: boolean;
    activeCount?: number;
  }>();

  let downloads = $state<StudyDownloadRow[]>([]);
  let loading = $state(false);
  let error = $state("");
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenBatch: UnlistenFn | null = null;
  let unlistenUploadProgress: UnlistenFn | null = null;
  let unlistenUploadComplete: UnlistenFn | null = null;

  type ProgressEvent = {
    payload: {
      id: number;
      title: string;
      platform: string;
      percent: number;
    };
  };
  type CompleteEvent = {
    payload: {
      id: number;
      title: string;
      platform: string;
      success: boolean;
      error?: string;
      file_path?: string;
      file_size_bytes?: number;
    };
  };
  type BatchEvent = {
    payload: {
      batch_id: number;
      message_id: number;
      status: string;
      percent: number;
      error?: string;
    };
  };
  type UploadCompleteEvent = {
    payload: {
      id: number;
      title: string;
      platform: string;
      success: boolean;
      error?: string;
      message_id?: number;
    };
  };

  function recomputeActiveCount() {
    activeCount = downloads.filter(
      (d) => d.status === "queued" || d.status === "downloading",
    ).length;
  }

  async function refresh() {
    loading = true;
    error = "";
    try {
      const res = await studyDownloadsList();
      downloads = res.downloads;
      recomputeActiveCount();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function patchRow(id: number, patch: Partial<StudyDownloadRow>) {
    downloads = downloads.map((d) => (d.id === id ? { ...d, ...patch } : d));
    recomputeActiveCount();
  }

  async function onProgress(ev: ProgressEvent) {
    if (ev.payload.platform !== "telegram") return;
    const studyId = studyIdForTelegramId(ev.payload.id);
    if (!studyId) return;
    patchRow(studyId, { progress_pct: ev.payload.percent });
    try {
      await studyDownloadsUpdate({
        id: studyId,
        progressPct: ev.payload.percent,
        status: "downloading",
      });
    } catch {
      /* noop */
    }
  }

  async function onComplete(ev: CompleteEvent) {
    if (ev.payload.platform !== "telegram") return;
    const studyId = studyIdForTelegramId(ev.payload.id);
    if (!studyId) return;
    clearLiveDownload(ev.payload.id);
    const status = ev.payload.success ? "done" : "error";
    patchRow(studyId, {
      status,
      progress_pct: ev.payload.success ? 100 : downloads.find((d) => d.id === studyId)?.progress_pct ?? 0,
      error_msg: ev.payload.error ?? null,
      output_path: ev.payload.file_path ?? null,
    });
    try {
      await studyDownloadsUpdate({
        id: studyId,
        status,
        errorMsg: ev.payload.error,
        outputPath: ev.payload.file_path,
        progressPct: ev.payload.success ? 100 : undefined,
      });
    } catch {
      /* noop */
    }
  }

  async function onUploadProgress(ev: ProgressEvent) {
    if (ev.payload.platform !== "telegram") return;
    const studyId = studyIdForUploadId(ev.payload.id);
    if (!studyId) return;
    patchRow(studyId, { progress_pct: ev.payload.percent, status: "downloading" });
    try {
      await studyDownloadsUpdate({
        id: studyId,
        progressPct: ev.payload.percent,
        status: "downloading",
      });
    } catch {
      /* noop */
    }
  }

  async function onUploadComplete(ev: UploadCompleteEvent) {
    if (ev.payload.platform !== "telegram") return;
    const studyId = studyIdForUploadId(ev.payload.id);
    if (!studyId) return;
    clearLiveUpload(ev.payload.id);
    const status = ev.payload.success ? "done" : "error";
    patchRow(studyId, {
      status,
      progress_pct: ev.payload.success ? 100 : downloads.find((d) => d.id === studyId)?.progress_pct ?? 0,
      error_msg: ev.payload.error ?? null,
    });
    try {
      await studyDownloadsUpdate({
        id: studyId,
        status,
        errorMsg: ev.payload.error,
        progressPct: ev.payload.success ? 100 : undefined,
      });
    } catch {
      /* noop */
    }
  }

  async function onBatchStatus(ev: BatchEvent) {
    const studyId = studyIdForBatchItem(ev.payload.batch_id, ev.payload.message_id);
    if (!studyId) return;
    if (ev.payload.status === "downloading") {
      patchRow(studyId, {
        progress_pct: ev.payload.percent,
        status: "downloading",
      });
      try {
        await studyDownloadsUpdate({
          id: studyId,
          status: "downloading",
          progressPct: ev.payload.percent,
        });
      } catch {
        /* noop */
      }
      return;
    }
    if (ev.payload.status === "done") {
      patchRow(studyId, { status: "done", progress_pct: 100 });
      try {
        await studyDownloadsUpdate({
          id: studyId,
          status: "done",
          progressPct: 100,
        });
      } catch {
        /* noop */
      }
      return;
    }
    if (ev.payload.status === "error") {
      patchRow(studyId, {
        status: "error",
        error_msg: ev.payload.error ?? null,
      });
      try {
        await studyDownloadsUpdate({
          id: studyId,
          status: "error",
          errorMsg: ev.payload.error,
        });
      } catch {
        /* noop */
      }
      return;
    }
    if (ev.payload.status === "skipped") {
      patchRow(studyId, { status: "cancelled" });
      try {
        await studyDownloadsUpdate({ id: studyId, status: "cancelled" });
      } catch {
        /* noop */
      }
    }
  }

  async function removeRow(id: number) {
    try {
      await studyDownloadsRemove(id);
      downloads = downloads.filter((d) => d.id !== id);
      recomputeActiveCount();
    } catch {
      /* noop */
    }
  }

  async function clearFinished() {
    try {
      await studyDownloadsClearFinished();
      downloads = downloads.filter(
        (d) => d.status === "queued" || d.status === "downloading",
      );
      recomputeActiveCount();
    } catch {
      /* noop */
    }
  }

  function formatBytes(n: number) {
    if (!n) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = n;
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function statusLabel(status: string) {
    return $t(`study.library.telegram.downloads.status_${status}`);
  }

  const READABLE_EXTS = ["pdf", "epub", "djvu", "mobi", "azw3", "azw", "fb2", "txt", "html", "htm", "rtf"];

  function isReadable(d: StudyDownloadRow): boolean {
    if (d.direction !== "download") return false;
    if (d.status !== "done") return false;
    if (!d.output_path) return false;
    const ext = d.file_name.split(".").pop()?.toLowerCase() ?? "";
    return READABLE_EXTS.includes(ext);
  }

  async function openInReader(d: StudyDownloadRow) {
    if (!d.output_path) return;
    try {
      const book = await studyReadRegisterFile(d.output_path);
      open = false;
      void goto(`/study/read/${book.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(async () => {
    unlistenProgress = await listen<ProgressEvent["payload"]>(
      "generic-download-progress",
      (ev) => void onProgress(ev as ProgressEvent),
    );
    unlistenComplete = await listen<CompleteEvent["payload"]>(
      "generic-download-complete",
      (ev) => void onComplete(ev as CompleteEvent),
    );
    unlistenBatch = await listen<BatchEvent["payload"]>(
      "telegram-batch-file-status",
      (ev) => void onBatchStatus(ev as BatchEvent),
    );
    unlistenUploadProgress = await listen<ProgressEvent["payload"]>(
      "generic-upload-progress",
      (ev) => void onUploadProgress(ev as ProgressEvent),
    );
    unlistenUploadComplete = await listen<UploadCompleteEvent["payload"]>(
      "generic-upload-complete",
      (ev) => void onUploadComplete(ev as UploadCompleteEvent),
    );
    await refresh();
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenBatch?.();
    unlistenUploadProgress?.();
    unlistenUploadComplete?.();
  });

  export async function reload() {
    await refresh();
  }
</script>

{#if open}
  <aside class="drawer" aria-label="downloads">
    <header class="head">
      <strong>{$t("study.library.telegram.downloads.title")}</strong>
      <span class="muted small">
        {downloads.length}
      </span>
      <button
        type="button"
        class="ghost-icon"
        onclick={() => (open = false)}
        aria-label={$t("study.common.close")}
      >
        ✕
      </button>
    </header>

    {#if error}
      <p class="error small">{error}</p>
    {/if}

    {#if loading && downloads.length === 0}
      <p class="muted small">{$t("study.common.loading")}</p>
    {:else if downloads.length === 0}
      <p class="muted small">{$t("study.library.telegram.downloads.empty")}</p>
    {:else}
      <ul class="list">
        {#each downloads as d (d.id)}
          <li class="row" data-status={d.status}>
            <div class="row-head">
              <span class="dir-icon" aria-hidden="true">
                {d.direction === "upload" ? "↑" : "↓"}
              </span>
              <span class="title" title={d.file_name}>{d.file_name}</span>
              <button
                type="button"
                class="ghost-icon"
                onclick={() => removeRow(d.id)}
                aria-label={$t("study.common.delete")}
              >
                ✕
              </button>
            </div>
            <div class="row-meta muted small">
              <span>{statusLabel(d.status)}</span>
              {#if d.expected_size > 0}
                <span aria-hidden="true">·</span>
                <span>{formatBytes(d.expected_size)}</span>
              {/if}
              {#if d.status === "downloading" || d.status === "queued"}
                <span aria-hidden="true">·</span>
                <span class="mono">{Math.round(d.progress_pct)}%</span>
              {/if}
            </div>
            {#if d.status === "downloading" || d.status === "queued"}
              <div class="bar-track">
                <div
                  class="bar-fill"
                  style:width={`${Math.max(0, Math.min(100, d.progress_pct))}%`}
                ></div>
              </div>
            {/if}
            {#if d.error_msg}
              <p class="error small">{d.error_msg}</p>
            {/if}
            {#if isReadable(d)}
              <button
                type="button"
                class="ghost-btn small"
                onclick={() => void openInReader(d)}
              >
                📖 {$t("study.library.telegram.downloads.open_in_reader")}
              </button>
            {/if}
          </li>
        {/each}
      </ul>

      <footer class="foot">
        <button type="button" class="ghost-btn small" onclick={clearFinished}>
          {$t("study.library.telegram.downloads.clear_finished")}
        </button>
      </footer>
    {/if}
  </aside>
{/if}

<style>
  .drawer {
    position: fixed;
    right: 16px;
    bottom: 16px;
    width: 360px;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--button-elevated);
    border: 1px solid var(--content-border);
    border-radius: calc(var(--border-radius) * 1.4);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    z-index: 50;
    overflow: hidden;
    animation: slide-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slide-in {
    from { transform: translateY(12px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--content-border);
  }
  .head strong {
    color: var(--secondary);
    font-size: 13px;
  }
  .head .muted {
    margin-left: auto;
    margin-right: 4px;
  }
  .ghost-icon {
    background: transparent;
    border: none;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: var(--border-radius);
  }
  .ghost-icon:hover {
    background: var(--sidebar-highlight);
    color: var(--secondary);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    max-height: 50vh;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
  }
  .row:last-child {
    border-bottom: none;
  }
  .row-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dir-icon {
    color: var(--accent);
    font-weight: 700;
    width: 12px;
    flex-shrink: 0;
  }
  .row .title {
    flex: 1;
    color: var(--secondary);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-meta {
    display: flex;
    gap: 4px;
    align-items: baseline;
  }
  .bar-track {
    width: 100%;
    height: 3px;
    background: color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }
  .row[data-status="done"] .bar-fill {
    background: var(--success);
  }
  .row[data-status="error"] .bar-fill,
  .row[data-status="cancelled"] .bar-fill {
    background: var(--error);
  }
  .foot {
    padding: 8px 12px;
    border-top: 1px solid var(--content-border);
    display: flex;
    justify-content: flex-end;
  }
  .ghost-btn {
    background: transparent;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    color: var(--secondary);
    font-family: inherit;
    cursor: pointer;
    padding: 6px 12px;
    font-size: 12px;
  }
  .ghost-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .small {
    font-size: 11px;
  }
  .muted {
    color: var(--tertiary);
  }
  .error {
    color: var(--error);
  }
  .mono {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
  }
  @media (prefers-reduced-motion: reduce) {
    .drawer {
      animation: none;
    }
    .bar-fill {
      transition: none;
    }
  }
</style>
