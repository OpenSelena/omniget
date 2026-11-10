import { invoke } from "@tauri-apps/api/core";
import {
  loadNamiHistory,
  reconcileNamiDisk,
  deleteNamiHistoryRecord,
  type NamiMediaRecord,
  type ReconcileResult,
} from "$lib/api/nami";

let records = $state<NamiMediaRecord[]>([]);
let loading = $state(false);
let reconciling = $state(false);
let platformFilter = $state<string>("all");
let searchQuery = $state("");
let reconcileStats = $state<ReconcileResult | null>(null);
let error = $state<string | null>(null);

export function getRecords(): NamiMediaRecord[] {
  return records;
}

export function isLoading(): boolean {
  return loading;
}

export function isReconciling(): boolean {
  return reconciling;
}

export function getPlatformFilter(): string {
  return platformFilter;
}

export function setPlatformFilter(filter: string) {
  platformFilter = filter;
}

export function getSearchQuery(): string {
  return searchQuery;
}

export function setSearchQuery(query: string) {
  searchQuery = query;
}

export function getReconcileStats(): ReconcileResult | null {
  return reconcileStats;
}

export function clearReconcileStats() {
  reconcileStats = null;
}

export function getError(): string | null {
  return error;
}

export function clearError() {
  error = null;
}

export function getFilteredRecords(): NamiMediaRecord[] {
  return records.filter((r) => {
    if (platformFilter !== "all" && r.platform.toLowerCase() !== platformFilter.toLowerCase()) {
      return false;
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      const matchAuthor = r.author?.toLowerCase().includes(q) ?? false;
      const matchPostId = r.post_id.toLowerCase().includes(q);
      const matchFilePath = r.file_path.toLowerCase().includes(q);
      const matchUrl = r.url.toLowerCase().includes(q);
      return matchAuthor || matchPostId || matchFilePath || matchUrl;
    }
    return true;
  });
}

export async function loadHistory(platform?: string | null, limit = 100, offset = 0): Promise<NamiMediaRecord[]> {
  loading = true;
  error = null;
  try {
    const plat = platform && platform !== "all" ? platform : null;
    const result = await loadNamiHistory(plat, limit, offset);
    records = result;
    return result;
  } catch (err) {
    error = typeof err === "string" ? err : err instanceof Error ? err.message : "Failed to load history";
    return [];
  } finally {
    loading = false;
  }
}

export async function reconcileDisk(): Promise<ReconcileResult | null> {
  reconciling = true;
  error = null;
  try {
    const stats = await reconcileNamiDisk();
    reconcileStats = stats;
    await loadHistory(platformFilter);
    return stats;
  } catch (err) {
    error = typeof err === "string" ? err : err instanceof Error ? err.message : "Disk reconciliation failed";
    return null;
  } finally {
    reconciling = false;
  }
}

export async function deleteRecord(id: string): Promise<boolean> {
  try {
    const success = await deleteNamiHistoryRecord(id);
    if (success) {
      records = records.filter((r) => r.id !== id);
    }
    return success;
  } catch (err) {
    error = typeof err === "string" ? err : err instanceof Error ? err.message : "Failed to delete record";
    return false;
  }
}

export async function openFile(filePath: string): Promise<void> {
  try {
    await invoke("open_path_default", { path: filePath });
  } catch (err) {
    error = typeof err === "string" ? err : err instanceof Error ? err.message : "Failed to open file";
  }
}

export async function openFolder(filePath: string): Promise<void> {
  try {
    const lastSlash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
    const dir = lastSlash > 0 ? filePath.substring(0, lastSlash) : filePath;
    await invoke("open_path_default", { path: dir });
  } catch (err) {
    error = typeof err === "string" ? err : err instanceof Error ? err.message : "Failed to open directory";
  }
}

export function formatFileSize(bytes?: number | null): string {
  if (bytes === null || bytes === undefined) return "—";
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const idx = Math.min(i, units.length - 1);
  if (idx === 0) return `${bytes} B`;
  const val = bytes / Math.pow(1024, idx);
  return `${val.toFixed(1)} ${units[idx]}`;
}
