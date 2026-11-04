import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { checkTauriVersions } from "../../scripts/check-tauri-versions.mjs";

const root = join(__dirname, "..", "..");
const read = (p: string) => readFileSync(join(root, p), "utf8");

const pnpm = (notification: string) => `lockfileVersion: '9.0'

importers:

  .:
    dependencies:
      '@tauri-apps/api':
        specifier: ^2.12.4
        version: 2.12.4
      '@tauri-apps/plugin-notification':
        specifier: ^${notification}
        version: ${notification}

packages:
`;

const pkg = (name: string, version: string, deps: string[] = []) =>
  `[[package]]\nname = "${name}"\nversion = "${version}"\n` +
  (deps.length ? `dependencies = [\n${deps.map((d) => ` "${d}",`).join("\n")}\n]\n` : "") +
  "\n";

const cargo = (opts: { notification: string; allocatorWindows: string }) =>
  "version = 4\n\n" +
  pkg("tauri", "2.12.0") +
  pkg("tauri-plugin-notification", opts.notification) +
  pkg("windows", "0.56.0") +
  pkg("windows", "0.62.2") +
  pkg("gpu-allocator", "0.28.0", [`windows ${opts.allocatorWindows}`]) +
  pkg("wgpu-hal", "30.0.1", ["gpu-allocator", "windows 0.62.2"]);

describe("Tauri build inputs (upstream issue #143)", () => {
  it("agree in this repository", () => {
    expect(checkTauriVersions(read("src-tauri/Cargo.lock"), read("pnpm-lock.yaml"))).toEqual([]);
  });

  it("flag a notification plugin that drifted between npm and Cargo", () => {
    const problems = checkTauriVersions(
      cargo({ notification: "2.3.3", allocatorWindows: "0.62.2" }),
      pnpm("2.8.0"),
    );
    expect(problems).toEqual([
      "tauri-plugin-notification (v2.3.3) does not match @tauri-apps/plugin-notification (v2.8.0)",
    ]);
  });

  it("flag gpu-allocator and wgpu-hal on different windows crates", () => {
    const problems = checkTauriVersions(
      cargo({ notification: "2.8.0", allocatorWindows: "0.56.0" }),
      pnpm("2.8.0"),
    );
    expect(problems).toEqual(["gpu-allocator uses windows 0.56.0 but wgpu-hal uses windows 0.62.2"]);
  });

  it("flag a dependency edge that points at a missing lock entry", () => {
    const problems = checkTauriVersions(
      cargo({ notification: "2.8.0", allocatorWindows: "0.61.3" }),
      pnpm("2.8.0"),
    );
    expect(problems).toContain(
      'gpu-allocator 0.28.0 depends on "windows 0.61.3", which Cargo.lock does not define',
    );
  });
});
