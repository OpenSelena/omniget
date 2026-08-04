export function fmtDuration(ms: number | null | undefined): string {
  if (ms == null || ms <= 0 || !Number.isFinite(ms)) return "—";
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function fmtDurationLong(ms: number | null | undefined): string {
  if (ms == null || ms <= 0 || !Number.isFinite(ms)) return "—";
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  if (h > 0) return `${h}h ${m}min`;
  return `${m}min`;
}

export function fmtBitrate(kbps: number | null | undefined): string {
  if (!kbps || kbps <= 0) return "";
  return `${Math.round(kbps)} kbps`;
}

export function fmtSampleRate(hz: number | null | undefined): string {
  if (!hz || hz <= 0) return "";
  return `${(hz / 1000).toFixed(1)} kHz`;
}

export function colorFromString(s: string | null | undefined): string {
  if (!s) return "oklch(0.4 0.05 250)";
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = (hash * 31 + s.charCodeAt(i)) | 0;
  }
  const hue = Math.abs(hash) % 360;
  return `oklch(0.45 0.12 ${hue})`;
}

export function fmtSecondsToTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function trackDisplayTitle(track: { title?: string | null; path: string }): string {
  if (track.title && track.title.trim().length > 0) return track.title;
  const norm = track.path.replace(/\\/g, "/");
  const seg = norm.split("/").filter(Boolean).pop() ?? track.path;
  return seg.replace(/\.[^.]+$/, "");
}

export function fileNameOf(path: string): string {
  if (!path) return "";
  const norm = path.replace(/\\/g, "/");
  const seg = norm.split("/").filter(Boolean).pop() ?? path;
  return seg;
}
