import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PetVibe } from "./types";

export type PetSource = "bundle" | "manifest" | "manual";

export type LocalPetEntry = {
  slug: string;
  display_name: string;
  source: PetSource;
  installed_at: string;
  vibes: PetVibe[];
  tags: string[];
  kind?: string | null;
  sprite_ext: string;
  file_sizes: Record<string, number>;
  remote_checked_at?: string | null;
};

export type LocalIndex = {
  pets: LocalPetEntry[];
  last_synced_at: string | null;
  total: number;
};

export type RemotePet = {
  slug: string;
  display_name: string;
  description?: string | null;
  kind?: string | null;
  vibes: PetVibe[];
  tags: string[];
  featured: boolean;
  spritesheet_url?: string | null;
  pet_json_url?: string | null;
  zip_url?: string | null;
  submitted_by?: string | null;
};

export type RemoteManifest = {
  generated_at: string | null;
  total: number;
  featured: number;
  all_pets_pack_path: string | null;
  pets: RemotePet[];
};

export type DiffReport = {
  new_remote: string[];
  in_both: string[];
  only_local: string[];
  total_remote: number;
  total_local: number;
};

export type FailedInstall = {
  slug: string;
  reason: string;
};

export type InstallReport = {
  added: string[];
  skipped: string[];
  failed: FailedInstall[];
  total_bytes: number;
  duration_ms: number;
};

export type SyncStartedEvent = {
  kind: "bundle" | "incremental";
  expected: number;
};

export type SyncProgressEvent = {
  slug: string;
  current: number;
  total: number;
  bytes_downloaded: number;
  bytes_total: number;
};

export type SyncErrorEvent = {
  slug: string;
  message: string;
  retryable: boolean;
};

export function petsGetLocalIndex(): Promise<LocalIndex> {
  return invoke<LocalIndex>("pets_get_local_index");
}

export function petsFetchRemoteManifest(): Promise<RemoteManifest> {
  return invoke<RemoteManifest>("pets_fetch_remote_manifest");
}

export function petsDiff(): Promise<DiffReport> {
  return invoke<DiffReport>("pets_diff");
}

export function petsInstallBundle(): Promise<InstallReport> {
  return invoke<InstallReport>("pets_install_bundle");
}

export function petsInstallMissing(slugs?: string[]): Promise<InstallReport> {
  return invoke<InstallReport>("pets_install_missing", { slugs: slugs ?? null });
}

export function petsForceRefresh(slug: string): Promise<InstallReport> {
  return invoke<InstallReport>("pets_force_refresh", { slug });
}

export function petsUninstall(slug: string): Promise<void> {
  return invoke<void>("pets_uninstall", { slug });
}

export function petsOpenFolder(): Promise<void> {
  return invoke<void>("pets_open_folder");
}

export function petsSetActive(slug: string): Promise<void> {
  return invoke<void>("pets_set_active", { slug });
}

export function petsGetActive(): Promise<string | null> {
  return invoke<string | null>("pets_get_active");
}

export function petsResolvePath(slug: string): Promise<string> {
  return invoke<string>("pets_resolve_path", { slug });
}

export async function petsResolveAssetUrl(slug: string): Promise<string> {
  const path = await petsResolvePath(slug);
  return convertFileSrc(path);
}

export type SyncListeners = {
  onStarted?: (e: SyncStartedEvent) => void;
  onProgress?: (e: SyncProgressEvent) => void;
  onFinished?: (e: InstallReport) => void;
  onError?: (e: SyncErrorEvent) => void;
};

export async function attachSyncListeners(handlers: SyncListeners): Promise<() => void> {
  const offs: UnlistenFn[] = [];
  if (handlers.onStarted) {
    offs.push(await listen<SyncStartedEvent>("pets:sync:started", (e) => handlers.onStarted?.(e.payload)));
  }
  if (handlers.onProgress) {
    offs.push(await listen<SyncProgressEvent>("pets:sync:progress", (e) => handlers.onProgress?.(e.payload)));
  }
  if (handlers.onFinished) {
    offs.push(await listen<InstallReport>("pets:sync:finished", (e) => handlers.onFinished?.(e.payload)));
  }
  if (handlers.onError) {
    offs.push(await listen<SyncErrorEvent>("pets:sync:error", (e) => handlers.onError?.(e.payload)));
  }
  return () => {
    for (const off of offs) off();
  };
}
