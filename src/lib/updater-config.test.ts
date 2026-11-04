import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { updaterConfig } from "../../scripts/updater-config.mjs";

const root = join(__dirname, "..", "..");

describe("updater config override", () => {
  it("keeps tauri.conf.json values when nothing is set", () => {
    expect(updaterConfig({}, { bundle: { createUpdaterArtifacts: false } })).toEqual({
      bundle: { createUpdaterArtifacts: false },
    });
  });

  it("uses the fork's endpoint and public key when set", () => {
    const c = updaterConfig({
      OMNIGET_UPDATER_ENDPOINT: "https://github.com/me/omniget/releases/latest/download/latest.json",
      OMNIGET_UPDATER_PUBKEY: "QUJD",
    }) as { plugins: { updater: { endpoints: string[]; pubkey: string } } };
    expect(c.plugins.updater.endpoints).toEqual([
      "https://github.com/me/omniget/releases/latest/download/latest.json",
    ]);
    expect(c.plugins.updater.pubkey).toBe("QUJD");
  });

  it("refuses only one of the two values", () => {
    expect(() =>
      updaterConfig({ OMNIGET_UPDATER_ENDPOINT: "https://example.com/latest.json" }),
    ).toThrow(/both/);
    expect(() => updaterConfig({ OMNIGET_UPDATER_PUBKEY: "QUJD" })).toThrow(/both/);
  });

  it("points the AppImage zsync updates at this repository unless overridden", () => {
    const yml = readFileSync(join(root, ".github/workflows/release.yml"), "utf8");
    const line = yml.split("\n").find((l) => l.includes("LDAI_UPDATE_INFORMATION:")) ?? "";
    expect(line).not.toContain("OpenSelena");
    expect(line).toContain("vars.OMNIGET_APPIMAGE_UPDATE_INFORMATION");
    expect(line).toContain("github.repository_owner");
  });

  it("refuses a non-https endpoint", () => {
    expect(() =>
      updaterConfig({ OMNIGET_UPDATER_ENDPOINT: "http://x/latest.json", OMNIGET_UPDATER_PUBKEY: "QUJD" }),
    ).toThrow(/https/);
  });

  it("is what the release build uses", () => {
    const yml = readFileSync(join(root, ".github/workflows/release.yml"), "utf8");
    expect(yml).toContain("node scripts/updater-config.mjs > src-tauri/tauri.release.conf.json");
    expect(yml).toContain("--config src-tauri/tauri.release.conf.json");
    expect(yml).toContain("vars.OMNIGET_UPDATER_ENDPOINT");
  });
});
