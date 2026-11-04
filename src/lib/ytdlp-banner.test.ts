import { describe, expect, it } from "vitest";
import { ytdlpBanner } from "./ytdlp-banner.ts";

describe("ytdlpBanner", () => {
  it("stays hidden when yt-dlp works", () => {
    expect(ytdlpBanner(true, true, { state: "failed", offline: false, error: "x" })).toEqual({ kind: "none" });
  });

  it("shows install progress instead of the missing error", () => {
    expect(ytdlpBanner(true, false, { state: "installing", downloaded: 5, total: 20 })).toEqual({
      kind: "installing",
      percent: 25,
    });
    expect(ytdlpBanner(true, false, { state: "installing", downloaded: 5, total: null })).toEqual({
      kind: "installing",
      percent: null,
    });
    expect(ytdlpBanner(true, false, { state: "idle" })).toEqual({ kind: "installing", percent: null });
  });

  it("tells the user it will install when the network is back", () => {
    expect(
      ytdlpBanner(true, false, { state: "waiting", attempt: 2, retry_in_secs: 30, offline: true }),
    ).toEqual({ kind: "offline" });
    expect(
      ytdlpBanner(true, false, { state: "waiting", attempt: 1, retry_in_secs: 3, offline: false }),
    ).toEqual({ kind: "retrying", seconds: 3 });
  });

  it("reports a failure only once every attempt failed", () => {
    expect(ytdlpBanner(true, false, { state: "failed", offline: false, error: "HTTP 500" })).toEqual({
      kind: "failed",
    });
  });

  it("shows nothing before the first check", () => {
    expect(ytdlpBanner(false, false, { state: "idle" })).toEqual({ kind: "none" });
  });
});
