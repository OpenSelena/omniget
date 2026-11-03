import { describe, expect, it } from "vitest";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..", "..");
const readme = readFileSync(join(root, "README.md"), "utf8");
const wingetDir = join(root, "packaging", "winget", "manifests", "o", "OpenSelena", "OmniGet");

describe("README release claims", () => {
  it("does not offer a winget command before the first submission is merged", () => {
    // Update this test together with the README once OpenSelena.OmniGet is live.
    expect(readme).not.toMatch(/winget install[^\n]*OpenSelena\.OmniGet/);
    expect(existsSync(wingetDir)).toBe(true);
  });

  it("does not claim that Windows releases are signed", () => {
    expect(readme).not.toMatch(/Free code signing on Windows provided by/);
  });

  it("does not document plugins that are not in the source tree", () => {
    const plugins = readdirSync(join(root, "src-tauri", "plugins"));
    expect(plugins).not.toContain("study");
    expect(readme).not.toMatch(/Plugins: [^\n]*Study/);
    expect(readme).not.toMatch(/alt="Study"/);
  });

  it("uses the installer file names that the release publishes", () => {
    expect(readme).toContain("OmniGet_x.y.z_x64-setup.exe");
    expect(readme).not.toContain("omniget_x.y.z_x64-setup.exe");
  });
});

describe("winget manifest", () => {
  it("points at the GitHub release installer with a SHA-256", () => {
    const versions = readdirSync(wingetDir);
    expect(versions.length).toBeGreaterThan(0);
    for (const v of versions) {
      const installer = readFileSync(join(wingetDir, v, "OpenSelena.OmniGet.installer.yaml"), "utf8");
      expect(installer).toContain(`https://github.com/OpenSelena/omniget/releases/download/v${v}/OmniGet_${v}_x64-setup.exe`);
      expect(installer).toMatch(/InstallerSha256: [0-9A-F]{64}\n/);
      expect(installer).toContain(`PackageVersion: ${v}`);
    }
  });
});
