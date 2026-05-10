import { invoke } from "@tauri-apps/api/core";
import { getDownloadStats } from "$lib/stores/download-stats.svelte";

export type RpcSource = "focus" | "music" | "video" | "course" | "reading";

export type RpcSetSourceArgs = {
  source: RpcSource;
  details: string;
  state: string;
  duration?: number;
  position?: number;
  paused?: boolean;
  largeImageKey?: string | null;
};

export async function rpcSetSource(args: RpcSetSourceArgs): Promise<void> {
  try {
    await invoke("rpc_set_source", {
      source: args.source,
      details: args.details,
      state: args.state,
      duration: args.duration ?? 0,
      position: args.position ?? 0,
      paused: args.paused ?? false,
      largeImageKey: args.largeImageKey ?? null,
    });
  } catch {}
}

export async function rpcClearSource(source: RpcSource): Promise<void> {
  try {
    await invoke("rpc_clear_source", { source });
  } catch {}
}

export async function rpcSyncIdleStats(): Promise<void> {
  const stats = getDownloadStats();
  try {
    await invoke("rpc_set_idle_stats", {
      downloadsCount: stats.totalDownloads,
      totalBytes: stats.totalBytes,
    });
  } catch {}
}

export async function rpcTestConnection(): Promise<{ ok: boolean; reason?: string }> {
  return await invoke("rpc_test_connection");
}
