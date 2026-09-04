import {
  runGalleryDlDownload,
  cancelDownload,
  listenToDownloadProgress,
  generateDownloadId,
  type DownloadProgress,
  type DownloadResult,
} from "$lib/api/open_omni";

export type StatusType = "success" | "error" | "info";

export interface OpenOmniDraft {
  url: string;
  contentType: string;
  selectedProfileUrl: string | null;
}

let draft = $state<OpenOmniDraft>({
  url: "",
  contentType: "all",
  selectedProfileUrl: null,
});

let active = $state(false);
let downloadId = $state<string | null>(null);
let filesDownloaded = $state(0);
let stage = $state<string | null>(null);
let stageIndex = $state<number | null>(null);
let stageTotal = $state<number | null>(null);
let lastMessage = $state("");
let status = $state("");
let statusType = $state<StatusType>("info");
let statusDetail = $state("");
let cancelling = $state(false);
let unlisten: (() => void) | null = null;

export function getDraft(): OpenOmniDraft {
  return draft;
}

export function setDraft(newDraft: Partial<OpenOmniDraft>) {
  if (newDraft.url !== undefined) draft.url = newDraft.url;
  if (newDraft.contentType !== undefined) draft.contentType = newDraft.contentType;
  if (newDraft.selectedProfileUrl !== undefined) draft.selectedProfileUrl = newDraft.selectedProfileUrl;
}

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

export function getStatusDetail(): string {
  return statusDetail;
}

export function isCancelling(): boolean {
  return cancelling;
}

export function clearStatus() {
  status = "";
  statusType = "info";
  statusDetail = "";
}

function applyProgress(progressData: DownloadProgress) {
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
): Promise<DownloadResult | null> {
  if (active) {
    throw new Error("A download is already in progress");
  }

  const cleanUrl = url.trim();
  const cleanOutputDir = outputDir.trim();
  const cleanCookiesFile = cookiesFile.trim();

  if (!cleanUrl || !cleanOutputDir) {
    status = "Please fill in all required fields";
    statusType = "error";
    statusDetail = "";
    return null;
  }

  cancelling = false;
  active = true;
  resetProgressState();
  status = "Starting download...";
  statusType = "info";
  statusDetail = "";

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

    filesDownloaded = result.files_count;

    if (result.cancelled) {
      status = `Download cancelled — ${result.files_count} file(s) saved before stopping.`;
      statusType = "info";
      statusDetail = "";
    } else if (result.success) {
      status = `Downloaded ${result.files_count} file(s) successfully!`;
      statusType = "success";
      statusDetail = "";
    } else {
      const lines = (result.message || "Download failed").split("\n");
      status = lines[0] || "Download failed";
      statusType = "error";
      statusDetail = lines.length > 1 ? lines.slice(1).join("\n") : "";
    }

    return result;
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    status = `Download failed: ${msg}`;
    statusType = "error";
    statusDetail = "";
    return null;
  } finally {
    active = false;
    cancelling = false;
    if (unlisten) {
      try {
        unlisten();
      } catch {
        // ignore
      }
      unlisten = null;
    }
    downloadId = null;
  }
}

export async function stopDownload() {
  if (!active || !downloadId) return;
  if (cancelling) return;

  cancelling = true;
  status = "Cancelling download...";
  statusType = "info";

  try {
    await cancelDownload(downloadId);
  } catch (error) {
    console.error("Failed to cancel download:", error);
    status = `Failed to cancel: ${error}`;
    statusType = "error";
    cancelling = false;
  }
}
