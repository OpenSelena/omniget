import { describe, it, expect } from "vitest";
import { rawErrorText, translateBackendError } from "./error-translate";

const mockT = (key: string) => `T[${key}]`;

describe("translateBackendError", () => {
  it("returns common.unknown_error for empty input", () => {
    expect(translateBackendError("", mockT)).toBe("T[common.unknown_error]");
  });

  it("maps known backend strings exactly", () => {
    expect(
      translateBackendError("Video unavailable or removed.", mockT),
    ).toBe("T[errors.video_unavailable]");
    expect(translateBackendError("This video is private.", mockT)).toBe(
      "T[errors.video_private]",
    );
    expect(
      translateBackendError(
        "Server returned error 429 (too many requests). Try again later.",
        mockT,
      ),
    ).toBe("T[errors.rate_limited]");
  });

  it("strips the 'Failed to get formats:' prefix before matching", () => {
    expect(
      translateBackendError(
        "Failed to get formats: Video unavailable or removed.",
        mockT,
      ),
    ).toBe("T[errors.video_unavailable]");
  });

  it("falls back to heuristic cookie_database match", () => {
    expect(
      translateBackendError("Could not copy Chrome cookie database", mockT),
    ).toBe("T[errors.cookie_database]");
  });

  it("falls back to heuristic size_mismatch match", () => {
    expect(
      translateBackendError("Got a size mismatch on chunk 3", mockT),
    ).toBe("T[errors.size_mismatch]");
  });

  it("falls back to heuristic disk_full match", () => {
    expect(translateBackendError("Disk full on volume D:", mockT)).toBe(
      "T[errors.disk_full]",
    );
    expect(
      translateBackendError("write error when flushing buffer", mockT),
    ).toBe("T[errors.disk_full]");
  });

  it("falls back to heuristic tiktok_blocked match", () => {
    expect(
      translateBackendError("TikTok is blocking requests from this IP", mockT),
    ).toBe("T[errors.tiktok_blocked]");
  });

  it("returns the original stripped message when no mapping matches", () => {
    expect(translateBackendError("some unknown error detail", mockT)).toBe(
      "some unknown error detail",
    );
  });

  it("preserves punctuation in unmapped messages after strip", () => {
    expect(
      translateBackendError(
        "Failed to get formats: weird backend msg: x=1",
        mockT,
      ),
    ).toBe("weird backend msg: x=1");
  });

  it("translates the local network block, even when wrapped", () => {
    expect(
      translateBackendError(
        "error sending request for url (http://127.0.0.1/a.zip): private_network_blocked: this link points to this computer",
        mockT,
      ),
    ).toBe("T[errors.private_network_blocked]");
  });
});

describe("translateBackendError security and integrity messages", () => {
  const withValues = (key: string, opts: { values: Record<string, string | number> }) =>
    `T[${key}|${Object.values(opts.values).join(",")}]`;

  it("translates a checksum failure from the dependency installer", () => {
    expect(
      translateBackendError(
        "yt-dlp: hash nao confere — esperado abc, obtido def. Download descartado.",
        mockT,
      ),
    ).toBe("T[errors.integrity_failed]");
  });

  it("translates a refused yt-dlp argument and keeps the argument", () => {
    expect(
      translateBackendError("Disallowed yt-dlp argument for security: --exec", mockT, withValues),
    ).toBe("T[errors.ytdlp_argument_refused|--exec]");
    expect(
      translateBackendError("Disallowed yt-dlp argument for security: --exec", mockT),
    ).toBe("T[errors.ytdlp_argument_refused_generic]");
  });

  it("translates URL validation errors", () => {
    expect(translateBackendError("URL must not start with '-'", mockT)).toBe("T[errors.invalid_url]");
    expect(translateBackendError("unsupported URL scheme: file", mockT)).toBe("T[errors.invalid_url]");
    expect(translateBackendError("Invalid external URL", mockT)).toBe("T[errors.invalid_url]");
  });
});

describe("rawErrorText", () => {
  it("reads strings, errors and plain objects", () => {
    expect(rawErrorText("boom")).toBe("boom");
    expect(rawErrorText(new Error("bad"))).toBe("bad");
    expect(rawErrorText({ message: "obj" })).toBe("obj");
    expect(rawErrorText(42)).toBe("");
  });
});
