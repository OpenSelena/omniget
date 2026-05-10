import { pluginInvoke } from "$lib/plugin-invoke";

export type LyricsLine = { time: number; text: string };

type FetchResult = {
  track_id: number;
  found: boolean;
  synced: string | null;
  plain: string | null;
  source: string | null;
  cached?: boolean;
};

function parseTimestamp(s: string): number | null {
  const parts = s.split(":");
  if (parts.length !== 2) return null;
  const m = parseFloat(parts[0]);
  const sec = parseFloat(parts[1]);
  if (!Number.isFinite(m) || !Number.isFinite(sec)) return null;
  return m * 60 + sec;
}

export function parseLrc(lrc: string): LyricsLine[] {
  const out: LyricsLine[] = [];
  for (const line of lrc.split(/\r?\n/)) {
    let i = 0;
    const stamps: number[] = [];
    while (i < line.length && line[i] === "[") {
      const close = line.indexOf("]", i);
      if (close < 0) break;
      const inner = line.slice(i + 1, close);
      const t = parseTimestamp(inner);
      if (t === null) break;
      stamps.push(t);
      i = close + 1;
    }
    if (stamps.length === 0) continue;
    const text = line.slice(i).trim();
    for (const t of stamps) {
      out.push({ time: t, text });
    }
  }
  out.sort((a, b) => a.time - b.time);
  return out;
}

class LyricsStore {
  trackId = $state<number | null>(null);
  lines = $state<LyricsLine[]>([]);
  plain = $state<string | null>(null);
  loading = $state(false);
  notFound = $state(false);
  source = $state<string | null>(null);

  async loadFor(trackId: number, force = false) {
    if (this.trackId === trackId && !force && (this.lines.length > 0 || this.plain || this.notFound)) {
      return;
    }
    this.trackId = trackId;
    this.lines = [];
    this.plain = null;
    this.notFound = false;
    this.source = null;
    this.loading = true;
    try {
      const res = await pluginInvoke<FetchResult>(
        "study",
        "study:music:lyrics:fetch",
        { id: trackId, force },
      );
      if (this.trackId !== trackId) return;
      if (!res.found) {
        this.notFound = true;
        return;
      }
      this.source = res.source;
      if (res.synced) {
        this.lines = parseLrc(res.synced);
      }
      if (res.plain) {
        this.plain = res.plain;
      }
    } catch {
      this.notFound = true;
    } finally {
      if (this.trackId === trackId) {
        this.loading = false;
      }
    }
  }

  async clear(trackId: number) {
    try {
      await pluginInvoke("study", "study:music:lyrics:clear", { id: trackId });
    } catch {
      /* ignore */
    }
    if (this.trackId === trackId) {
      this.lines = [];
      this.plain = null;
      this.notFound = false;
      this.source = null;
    }
  }

  reset() {
    this.trackId = null;
    this.lines = [];
    this.plain = null;
    this.notFound = false;
    this.source = null;
    this.loading = false;
  }

  activeIndex(currentTime: number): number {
    if (this.lines.length === 0) return -1;
    let lo = 0;
    let hi = this.lines.length - 1;
    let result = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (this.lines[mid].time <= currentTime) {
        result = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    return result;
  }
}

export const lyricsStore = new LyricsStore();
