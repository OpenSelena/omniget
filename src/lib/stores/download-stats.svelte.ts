import { formatBytes } from "./download-store.svelte";

const STORAGE_KEY = "omniget_download_stats";

type Stats = {
  totalDownloads: number;
  totalBytes: number;
};

function load(): Stats {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return { totalDownloads: 0, totalBytes: 0 };
}

function save(stats: Stats) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stats));
  } catch {}
}

let stats = $state<Stats>(load());

export function getDownloadStats(): Stats {
  return stats;
}

export function recordDownloadComplete(bytes: number) {
  stats = {
    totalDownloads: stats.totalDownloads + 1,
    totalBytes: stats.totalBytes + Math.max(0, bytes),
  };
  save(stats);
}

export function formatStatsLine(
  tr: (key: string, params?: Record<string, string>) => string,
): string {
  if (stats.totalDownloads === 0) return "";
  return tr("downloads.stats_line", {
    count: String(stats.totalDownloads),
    size: formatBytes(stats.totalBytes),
  });
}
