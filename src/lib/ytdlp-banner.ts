/** Mirrors `SetupState` in omniget-core/src/core/ytdlp_setup.rs. */
export type YtdlpSetup =
  | { state: "idle" }
  | { state: "installing"; downloaded: number; total: number | null }
  | { state: "waiting"; attempt: number; retry_in_secs: number; offline: boolean }
  | { state: "ready" }
  | { state: "failed"; offline: boolean; error: string };

export type YtdlpBanner =
  | { kind: "none" }
  | { kind: "installing"; percent: number | null }
  | { kind: "retrying"; seconds: number }
  | { kind: "offline" }
  | { kind: "failed" };

/**
 * What the home banner shows. Nothing while yt-dlp works or before the
 * first check; progress while the automatic install runs; an error only
 * once every attempt failed.
 */
export function ytdlpBanner(checked: boolean, available: boolean, setup: YtdlpSetup): YtdlpBanner {
  if (available) return { kind: "none" };
  switch (setup.state) {
    case "installing": {
      const { downloaded, total } = setup;
      const percent =
        total && total > 0 ? Math.min(100, Math.max(0, Math.round((downloaded / total) * 100))) : null;
      return { kind: "installing", percent };
    }
    case "waiting":
      return setup.offline ? { kind: "offline" } : { kind: "retrying", seconds: setup.retry_in_secs };
    case "failed":
      return setup.offline ? { kind: "offline" } : { kind: "failed" };
    case "ready":
      // Installed, but the availability check has not caught up yet.
      return { kind: "none" };
    case "idle":
      return checked ? { kind: "installing", percent: null } : { kind: "none" };
  }
}
