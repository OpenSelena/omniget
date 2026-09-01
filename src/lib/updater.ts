import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { storeChangelogForUpdate } from "$lib/stores/changelog-store.svelte";
import { getSettings } from "$lib/stores/settings-store.svelte";

export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
  percent: number;
  stage: "downloading" | "installing" | "relaunching";
}

function proxyUrl(): string | undefined {
  const proxy = getSettings()?.proxy;
  if (!proxy?.enabled || !proxy.host || !proxy.port) return undefined;
  const scheme =
    proxy.proxy_type === "socks5" || proxy.proxy_type === "https"
      ? proxy.proxy_type
      : "http";
  const auth = proxy.username
    ? `${encodeURIComponent(proxy.username)}:${encodeURIComponent(proxy.password ?? "")}@`
    : "";
  return `${scheme}://${auth}${proxy.host}:${proxy.port}`;
}

export async function checkForUpdate(): Promise<UpdateInfo> {
  try {
    const update = await check({ proxy: proxyUrl() });
    if (update) {
      return {
        available: true,
        version: update.version,
        body: update.body ?? undefined,
      };
    }
    return { available: false };
  } catch (e) {
    console.warn("Check for update failed:", e);
    return { available: false };
  }
}

export async function installUpdate(
  onProgress?: (progress: UpdateProgress) => void
): Promise<void> {
  const update = await check({ proxy: proxyUrl() });
  if (!update) {
    throw new Error("No update available to install");
  }

  if (update.body && update.version) {
    storeChangelogForUpdate(update.body, update.version);
  }

  let contentLength = 0;
  let downloadedBytes = 0;

  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength ?? 0;
      downloadedBytes = 0;
      onProgress?.({
        downloaded: 0,
        total: contentLength,
        percent: 0,
        stage: "downloading",
      });
    } else if (event.event === "Progress") {
      downloadedBytes += event.data.chunkLength;
      const percent =
        contentLength > 0
          ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
          : 0;
      onProgress?.({
        downloaded: downloadedBytes,
        total: contentLength,
        percent,
        stage: "downloading",
      });
    } else if (event.event === "Finished") {
      onProgress?.({
        downloaded: contentLength || downloadedBytes,
        total: contentLength || downloadedBytes,
        percent: 100,
        stage: "installing",
      });
    }
  });

  onProgress?.({
    downloaded: contentLength || downloadedBytes,
    total: contentLength || downloadedBytes,
    percent: 100,
    stage: "relaunching",
  });

  await relaunch();
}
