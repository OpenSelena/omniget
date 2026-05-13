import { pluginInvoke } from "$lib/plugin-invoke";

export type Syllable = { time_ms: number; text: string };
export type LyricsLine = {
  time_ms: number;
  text: string;
  syllables?: Syllable[] | null;
};
export type LyricsKind = "synced" | "plain" | "notfound" | "error";
export type LyricsStatus = "idle" | "loading" | LyricsKind;
export type TranslationStatus =
  | "idle"
  | "loading"
  | "available"
  | "skipped"
  | "no_translator"
  | "error";

export type TranslationLine = {
  original: string;
  translation: string;
  edited: boolean;
};

export type TranslationSettings = {
  translator: "libretranslate" | "deepl" | "none";
  libretranslate_url: string;
  deepl_api_key_set: boolean;
  target_lang: string;
  show_translation: boolean;
};

type FetchResult = {
  track_id: number;
  found: boolean;
  kind: LyricsKind;
  source: string | null;
  lines: LyricsLine[] | null;
  plain: string | null;
  meta: Record<string, string> | null;
  fetched_at: number;
  cached?: boolean;
  error?: string | null;
  synced?: string | null;
};

type TranslateResult = {
  track_id: number;
  target_lang: string;
  translator: string;
  cached: boolean;
  skipped: boolean;
  error?: string | null;
  lines: Array<{
    time_ms: number;
    original: string;
    translation: string;
    edited_by_user: boolean;
  }>;
};

class LyricsStore {
  trackId = $state<number | null>(null);
  lines = $state<LyricsLine[]>([]);
  plain = $state<string | null>(null);
  loading = $state(false);
  status = $state<LyricsStatus>("idle");
  source = $state<string | null>(null);
  meta = $state<Record<string, string>>({});
  error = $state<string | null>(null);

  translations = $state<Map<number, TranslationLine>>(new Map());
  translationStatus = $state<TranslationStatus>("idle");
  translationError = $state<string | null>(null);
  translator = $state<string | null>(null);
  showTranslation = $state(false);

  private _lastIdx = -1;
  private _lastTranslationKey: string | null = null;

  get notFound(): boolean {
    return this.status === "notfound";
  }

  async loadFor(trackId: number, force = false) {
    if (
      this.trackId === trackId &&
      !force &&
      this.status !== "idle" &&
      this.status !== "loading"
    ) {
      return;
    }
    this.trackId = trackId;
    this.lines = [];
    this.plain = null;
    this.meta = {};
    this.source = null;
    this.error = null;
    this._lastIdx = -1;
    this._resetTranslation();
    this.loading = true;
    this.status = "loading";
    try {
      const res = await pluginInvoke<FetchResult>(
        "study",
        "study:music:lyrics:fetch",
        { id: trackId, force },
      );
      if (this.trackId !== trackId) return;
      this._applyResult(res);
    } catch (e) {
      if (this.trackId === trackId) {
        this.status = "error";
        this.error = e instanceof Error ? e.message : String(e);
      }
    } finally {
      if (this.trackId === trackId) {
        this.loading = false;
      }
    }
  }

  async setLocal(trackId: number, lrcText: string): Promise<boolean> {
    try {
      const res = await pluginInvoke<FetchResult>(
        "study",
        "study:music:lyrics:set_local",
        { track_id: trackId, lrc_text: lrcText },
      );
      if (this.trackId === trackId) {
        this._applyResult(res);
      }
      return true;
    } catch (e) {
      if (this.trackId === trackId) {
        this.status = "error";
        this.error = e instanceof Error ? e.message : String(e);
      }
      return false;
    }
  }

  async clear(trackId: number) {
    try {
      await pluginInvoke("study", "study:music:lyrics:clear", { id: trackId });
    } catch {
      /* swallow — clear is best-effort */
    }
    if (this.trackId === trackId) {
      this.reset();
    }
  }

  reset() {
    this.trackId = null;
    this.lines = [];
    this.plain = null;
    this.meta = {};
    this.source = null;
    this.error = null;
    this.status = "idle";
    this.loading = false;
    this._lastIdx = -1;
    this._resetTranslation();
  }

  private _resetTranslation() {
    this.translations = new Map();
    this.translationStatus = "idle";
    this.translationError = null;
    this.translator = null;
    this._lastTranslationKey = null;
  }

  async loadTranslation(target = "pt"): Promise<void> {
    const trackId = this.trackId;
    if (trackId === null) return;
    if (this.status !== "synced" || this.lines.length === 0) {
      this.translationStatus = "idle";
      return;
    }
    const key = `${trackId}:${target}`;
    if (this._lastTranslationKey === key && this.translationStatus === "available") {
      return;
    }
    this._lastTranslationKey = key;
    this.translationStatus = "loading";
    this.translationError = null;
    try {
      const res = await pluginInvoke<TranslateResult>(
        "study",
        "study:music:lyrics:translate",
        { track_id: trackId, target_lang: target },
      );
      if (this.trackId !== trackId) return;
      if (res.error === "no_translator") {
        this.translationStatus = "no_translator";
        this.translations = new Map();
        this.translator = null;
        return;
      }
      if (res.error) {
        this.translationStatus = "error";
        this.translationError = res.error;
        return;
      }
      if (res.skipped) {
        this.translationStatus = "skipped";
        this.translations = new Map();
        this.translator = "skipped";
        return;
      }
      const map = new Map<number, TranslationLine>();
      for (let i = 0; i < res.lines.length; i++) {
        const item = res.lines[i];
        map.set(i, {
          original: item.original,
          translation: item.translation,
          edited: item.edited_by_user,
        });
      }
      this.translations = map;
      this.translator = res.translator || null;
      this.translationStatus = "available";
    } catch (e) {
      if (this.trackId === trackId) {
        this.translationStatus = "error";
        this.translationError = e instanceof Error ? e.message : String(e);
      }
    }
  }

  async setTranslationLine(idx: number, text: string): Promise<boolean> {
    const trackId = this.trackId;
    if (trackId === null) return false;
    const existing = this.translations.get(idx);
    const original = existing?.original ?? this.lines[idx]?.text ?? "";
    try {
      await pluginInvoke("study", "study:music:lyrics:set_translation_line", {
        track_id: trackId,
        line_idx: idx,
        text,
      });
      const map = new Map(this.translations);
      map.set(idx, { original, translation: text, edited: true });
      this.translations = map;
      return true;
    } catch (e) {
      this.translationError = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  toggleTranslation(target = "pt"): void {
    this.showTranslation = !this.showTranslation;
    if (this.showTranslation) {
      void this.loadTranslation(target);
    }
  }

  activeIndex(currentTime: number): number {
    const lines = this.lines;
    if (lines.length === 0) return -1;
    const targetMs = currentTime * 1000;
    const last = this._lastIdx;
    if (last >= 0 && last < lines.length) {
      const at = lines[last].time_ms;
      const next = last + 1 < lines.length ? lines[last + 1].time_ms : Infinity;
      if (at <= targetMs && targetMs < next) {
        return last;
      }
      if (last + 1 < lines.length && lines[last + 1].time_ms <= targetMs) {
        const nextNext =
          last + 2 < lines.length ? lines[last + 2].time_ms : Infinity;
        if (targetMs < nextNext) {
          this._lastIdx = last + 1;
          return last + 1;
        }
      }
    }
    let lo = 0;
    let hi = lines.length - 1;
    let result = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (lines[mid].time_ms <= targetMs) {
        result = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    this._lastIdx = result;
    return result;
  }

  private _applyResult(res: FetchResult) {
    this.source = res.source;
    this.meta = res.meta ?? {};
    this.error = res.error ?? null;
    if (res.kind === "synced" && Array.isArray(res.lines) && res.lines.length > 0) {
      this.lines = res.lines;
      this.plain = res.plain;
      this.status = "synced";
    } else if (res.kind === "plain" && res.plain) {
      this.lines = [];
      this.plain = res.plain;
      this.status = "plain";
    } else if (res.kind === "error") {
      this.lines = [];
      this.plain = null;
      this.status = "error";
    } else {
      this.lines = [];
      this.plain = null;
      this.status = "notfound";
    }
    this._lastIdx = -1;
  }
}

export const lyricsStore = new LyricsStore();
