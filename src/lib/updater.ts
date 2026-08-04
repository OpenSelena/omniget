import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { storeChangelogForUpdate } from "$lib/stores/changelog-store.svelte";
import { getSettings } from "$lib/stores/settings-store.svelte";

export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
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
  } catch {
    return { available: false };
  }
}

export async function installUpdate(): Promise<void> {
  const update = await check({ proxy: proxyUrl() });
  if (update) {
    if (update.body && update.version) {
      storeChangelogForUpdate(update.body, update.version);
    }
    await update.downloadAndInstall();
    await relaunch();
  }
}
