import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..", "..");
const readme = readFileSync(join(root, "README.md"), "utf8");

describe("README release claims", () => {
  it("does not offer a winget command before the first submission is merged", () => {
    // Update this test together with the README once OpenSelena.OmniGet is live.
    expect(readme).not.toMatch(/winget install[^\n]*OpenSelena\.OmniGet/);
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
