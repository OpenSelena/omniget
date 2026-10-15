import { describe, it, expect } from "vitest";
import { formatDepVersion } from "./dep-version";

describe("formatDepVersion", () => {
  it("formats null or empty version as dash", () => {
    expect(formatDepVersion(null)).toEqual({ display: "—" });
    expect(formatDepVersion("")).toEqual({ display: "—" });
    expect(formatDepVersion("   ")).toEqual({ display: "—" });
  });

  it("handles standard semver and dates without doubling v", () => {
    expect(formatDepVersion("2026.08.19")).toEqual({ display: "v2026.08.19" });
    expect(formatDepVersion("v2026.08.19")).toEqual({ display: "v2026.08.19" });
  });

  it("handles ffmpeg git master hashes", () => {
    expect(formatDepVersion("N-127197-gf0c2c00a62-20261004")).toEqual({
      display: "vN-127197-gf0c2c00a62-20261004",
    });
  });

  it("sanitizes unparsed PDFium build string with archive name into clean build number and tooltip", () => {
    const raw = "MAJOR=156 MINOR=0 BUILD=8076 PATCH=0 (pdfium-win-x64.tgz)";
    const res = formatDepVersion(raw);
    expect(res.display).toBe("v8076");
    expect(res.tooltip).toBe("pdfium-win-x64.tgz");
  });

  it("handles clean PDFium build number", () => {
    expect(formatDepVersion("8076")).toEqual({ display: "v8076" });
    expect(formatDepVersion("8076 (pdfium-win-x64.tgz)")).toEqual({
      display: "v8076",
      tooltip: "pdfium-win-x64.tgz",
    });
  });
});
