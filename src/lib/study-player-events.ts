import {
  studyPlayerEvent,
  studyPlayerAnalyticsContext,
  type AnalyticsContext,
} from "$lib/study-bridge";

type CachedContext = {
  lessonId: number;
  context: AnalyticsContext;
  fetchedAt: number;
};

const CACHE_TTL_MS = 60_000;
let cached: CachedContext | null = null;
let inflight: Promise<AnalyticsContext> | null = null;

async function loadContext(lessonId: number): Promise<AnalyticsContext> {
  if (cached && cached.lessonId === lessonId && Date.now() - cached.fetchedAt < CACHE_TTL_MS) {
    return cached.context;
  }
  if (inflight) return inflight;
  inflight = studyPlayerAnalyticsContext({ lessonId })
    .then((ctx) => {
      cached = { lessonId, context: ctx, fetchedAt: Date.now() };
      return ctx;
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function invalidatePlayerEventCache(lessonId?: number) {
  if (lessonId == null || cached?.lessonId === lessonId) {
    cached = null;
  }
}

export type PlayerEventKind =
  | "loading"
  | "playing"
  | "paused"
  | "ended"
  | "next_lesson";

export type LocalEventKind =
  | "play"
  | "pause"
  | "seek"
  | "ended"
  | "completed"
  | "skipped_intro"
  | "skipped_outro";

const LOCAL_TO_BACKEND: Record<LocalEventKind, PlayerEventKind> = {
  play: "playing",
  pause: "paused",
  seek: "playing",
  ended: "ended",
  completed: "ended",
  skipped_intro: "playing",
  skipped_outro: "playing",
};

export function firePlayerEvent(
  localKind: LocalEventKind,
  lessonId: number,
  extras: Record<string, unknown> = {},
): void {
  void (async () => {
    try {
      const ctx = await loadContext(lessonId);
      const backendKind = LOCAL_TO_BACKEND[localKind];
      const payload: Record<string, unknown> = {
        ...extras,
        local_kind: localKind,
        context: ctx,
        time_ms: extras.time_ms ?? ctx.time_ms,
      };
      studyPlayerEvent(backendKind, payload);
    } catch {
      void 0;
    }
  })();
}
