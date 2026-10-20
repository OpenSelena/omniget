import { describe, it, expect } from "vitest";
import { isDisallowedYtdlpFlag, validateYtdlpArgs } from "./ytdlp-args";

describe("yt-dlp argument security validation", () => {
  it("rejects dangerous execution flags", () => {
    expect(isDisallowedYtdlpFlag("--exec")).toBe(true);
    expect(isDisallowedYtdlpFlag("--exec=rm -rf /")).toBe(true);
    expect(isDisallowedYtdlpFlag("--exec-before-download")).toBe(true);
    expect(isDisallowedYtdlpFlag("--external-downloader")).toBe(true);
    expect(isDisallowedYtdlpFlag("--downloader=aria2c")).toBe(true);
    expect(isDisallowedYtdlpFlag("--config-locations")).toBe(true);
    expect(isDisallowedYtdlpFlag("--config-location=/tmp/evil.conf")).toBe(true);
    expect(isDisallowedYtdlpFlag("--alias")).toBe(true);
    expect(isDisallowedYtdlpFlag("--print-to-file")).toBe(true);
  });

  it("permits safe download arguments", () => {
    expect(isDisallowedYtdlpFlag("--format bestvideo+bestaudio")).toBe(false);
    expect(isDisallowedYtdlpFlag("--sub-lang en")).toBe(false);
    expect(isDisallowedYtdlpFlag("--write-sub")).toBe(false);
    expect(isDisallowedYtdlpFlag("--no-playlist")).toBe(false);
  });

  it("validates argument batches", () => {
    const safeResult = validateYtdlpArgs(["--no-playlist", "--sub-lang", "en"]);
    expect(safeResult.valid).toBe(true);

    const maliciousResult = validateYtdlpArgs(["--no-playlist", "--exec=calc.exe"]);
    expect(maliciousResult.valid).toBe(false);
    expect(maliciousResult.disallowed).toBe("--exec");
  });
});
