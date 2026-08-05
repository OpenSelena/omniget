import {
  runGalleryDlDownload,
  cancelDownload,
  listenToDownloadProgress,
  generateDownloadId,
  type DownloadProgress,
} from "$lib/api/open_omni";

export type StatusType = "success" | "error" | "info";

let active = $state(false);
let downloadId = $state<string | null>(null);
let filesDownloaded = $state(0);
let stage = $state<string | null>(null);
let stageIndex = $state<number | null>(null);
let stageTotal = $state<number | null>(null);
let lastMessage = $state("");
let status = $state("");
let statusType = $state<StatusType>("info");
let lastFilesCount = $state(0);
let unlisten: (() => void) | null = null;
let cancelledByUser = false;
let cancelling = $state(false);

export function isActive(): boolean {
  return active;
}

export function getDownloadId(): string | null {
  return downloadId;
}

export function getFilesDownloaded(): number {
  return filesDownloaded;
}

export function getStage(): string | null {
  return stage;
}

export function getStageIndex(): number | null {
  return stageIndex;
}

export function getStageTotal(): number | null {
  return stageTotal;
}

export function getLastMessage(): string {
  return lastMessage;
}

export function getStatus(): string {
  return status;
}

export function getStatusType(): StatusType {
  return statusType;
}

export function getLastFilesCount(): number {
  return lastFilesCount;
}

export function isCancelling(): boolean {
  return cancelling;
}

export function clearStatus() {
  status = "";
  statusType = "info";
}

function applyProgress(progressData: DownloadProgress) {
  // Prefer clean "Downloaded: …" messages; keep other non-empty messages
  // but never let an empty payload wipe the last useful text.
  if (progressData.message && progressData.message.trim()) {
    lastMessage = progressData.message.trim();
  }
  if (typeof progressData.files_downloaded === "number" && progressData.files_downloaded >= 0) {
    filesDownloaded = progressData.files_downloaded;
  }
  if (progressData.stage) {
    stage = progressData.stage;
  }
  if (typeof progressData.stage_index === "number") {
    stageIndex = progressData.stage_index;
  }
  if (typeof progressData.stage_total === "number") {
    stageTotal = progressData.stage_total;
  }
}

function resetProgressState() {
  filesDownloaded = 0;
  stage = null;
  stageIndex = null;
  stageTotal = null;
  lastMessage = "";
}

export async function startDownload(
  url: string,
  outputDir: string,
  cookiesFile: string,
  contentType: string,
) {
  if (active) {
    throw new Error("A download is already in progress");
  }

  cancelledByUser = false;
  cancelling = false;

  const cleanUrl = url.trim();
  const cleanOutputDir = outputDir.trim();
  const cleanCookiesFile = cookiesFile.trim();

  if (!cleanUrl || !cleanOutputDir) {
    status = "Please fill in all required fields";
    statusType = "error";
    return;
  }

  if ((contentType === "stories" || contentType === "highlights") && !cleanCookiesFile) {
    status = "Stories and Highlights require a cookies file — set one in Settings.";
    statusType = "error";
    return;
  }

  active = true;
  resetProgressState();
  status = "Starting download…";
  statusType = "info";

  const id = generateDownloadId();
  downloadId = id;

  try {
    unlisten = await listenToDownloadProgress(id, applyProgress);

    const result = await runGalleryDlDownload(
      cleanUrl,
      cleanOutputDir,
      cleanCookiesFile || null,
      contentType,
      id,
    );

    if (cancelledByUser || result.cancelled) {
      status = "Download cancelled";
      statusType = "info";
      filesDownloaded = result.files_count;
    } else if (result.success) {
      status = `Downloaded ${result.files_count} files successfully!`;
      statusType = "success";
      filesDownloaded = result.files_count;
      lastFilesCount = result.files_count;
    } else {
      status = result.message || "Download failed";
      statusType = "error";
      filesDownloaded = result.files_count;
    }
  } catch (error) {
    if (cancelledByUser) {
      status = "Download cancelled";
      statusType = "info";
    } else {
      status = `Download failed: ${error}`;
      statusType = "error";
    }
  } finally {
    active = false;
    cancelling = false;
    if (unlisten) {
      try {
        unlisten();
      } catch {
        // ignore – listener may already be gone
      }
      unlisten = null;
    }
    downloadId = null;
  }
}

export async function stopDownload() {
  if (!active || !downloadId) return;
  if (cancelling) return;

  cancelledByUser = true;
  cancelling = true;
  status = "Cancelling download…";
  statusType = "info";

  try {
    await cancelDownload(downloadId);
    // The run function owns final cleanup. We keep active=true until it
    // returns so a new download cannot race the cancelled task's listener.
  } catch (error) {
    console.error("Failed to cancel download:", error);
    status = `Failed to cancel: ${error}`;
    statusType = "error";
    cancelling = false;
  }
}

export async function reattachIfActive() {
  if (!active || !downloadId || unlisten) return;

  try {
    unlisten = await listenToDownloadProgress(downloadId, applyProgress);
  } catch (error) {
    console.error("Failed to reattach progress listener:", error);
  }
}
