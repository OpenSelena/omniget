import { pluginInvoke } from "$lib/plugin-invoke";

export type ReadingSessionOptions = {
  bookId: number;
  getLocation: () => string;
  tickSeconds?: number;
  maxIdleSeconds?: number;
};

export type ReadingSession = {
  start(): Promise<void>;
  stop(interrupted?: boolean): Promise<void>;
  notePageChange(): void;
};

export function createReadingSession(opts: ReadingSessionOptions): ReadingSession {
  const tickSeconds = opts.tickSeconds ?? 30;
  const maxIdleSeconds = opts.maxIdleSeconds ?? 120;

  let sessionId: number | null = null;
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let lastInteractionAt = Date.now();
  let pendingPages = 0;
  let stopping = false;
  let lastLocation = "";

  function onInteraction() {
    lastInteractionAt = Date.now();
  }

  function setupInteractionListeners() {
    window.addEventListener("pointerdown", onInteraction);
    window.addEventListener("keydown", onInteraction);
    window.addEventListener("wheel", onInteraction, { passive: true });
  }

  function teardownInteractionListeners() {
    window.removeEventListener("pointerdown", onInteraction);
    window.removeEventListener("keydown", onInteraction);
    window.removeEventListener("wheel", onInteraction);
  }

  async function tick() {
    if (sessionId == null || stopping) return;
    const hidden = typeof document !== "undefined" && document.visibilityState === "hidden";
    const idleMs = Date.now() - lastInteractionAt;
    if (hidden || idleMs > maxIdleSeconds * 1000) return;

    const pagesDelta = pendingPages;
    pendingPages = 0;
    try {
      await pluginInvoke("study", "study:read:session:tick", {
        sessionId,
        secondsDelta: tickSeconds,
        pagesDelta,
      });
    } catch (e) {
      console.error("session tick failed", e);
    }
  }

  return {
    async start() {
      if (sessionId != null) return;
      try {
        lastLocation = opts.getLocation();
        const res = await pluginInvoke<{ id: number }>("study", "study:read:session:start", {
          bookId: opts.bookId,
          location: lastLocation,
        });
        sessionId = res.id;
        lastInteractionAt = Date.now();
        setupInteractionListeners();
        intervalId = setInterval(() => {
          void tick();
        }, tickSeconds * 1000);
      } catch (e) {
        console.error("session start failed", e);
      }
    },

    async stop(interrupted = false) {
      if (sessionId == null || stopping) return;
      stopping = true;
      if (intervalId != null) {
        clearInterval(intervalId);
        intervalId = null;
      }
      teardownInteractionListeners();
      const finalLocation = opts.getLocation();
      const id = sessionId;
      sessionId = null;
      try {
        await pluginInvoke("study", "study:read:session:stop", {
          sessionId: id,
          locationEnd: finalLocation,
          interrupted,
        });
      } catch (e) {
        console.error("session stop failed", e);
      }
    },

    notePageChange() {
      const cur = opts.getLocation();
      if (cur !== lastLocation) {
        pendingPages += 1;
        lastLocation = cur;
      }
    },
  };
}
