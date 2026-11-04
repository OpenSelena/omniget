import { describe, it, expect } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse } from "yaml";
import { filesToSign, filesToUpload, macArchive } from "../../scripts/release-assets.mjs";
import { checkIntegrity, lockedPackage } from "../../scripts/pinned-tauri-cli.mjs";

const root = join(__dirname, "..", "..");
const workflow = parse(readFileSync(join(root, ".github/workflows/release.yml"), "utf8"));
const version: string = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;

type Step = { name?: string; uses?: string; run?: string; env?: Record<string, string> };
const steps: Step[] = Object.values(workflow.jobs as Record<string, { steps: Step[] }>).flatMap(
  (j) => j.steps,
);
const envOf = (s: Step) => Object.keys(s.env ?? {});
const builds = (s: Step) =>
  (s.uses ?? "").startsWith("tauri-apps/tauri-action") || /tauri\s+build/.test(s.run ?? "");

describe("release workflow secrets", () => {
  it("never gives the updater key or GITHUB_TOKEN to the build", () => {
    const buildSteps = steps.filter(builds);
    expect(buildSteps.length).toBeGreaterThan(0);
    for (const s of buildSteps) {
      expect(envOf(s), s.name).not.toContain("TAURI_SIGNING_PRIVATE_KEY");
      expect(envOf(s), s.name).not.toContain("GITHUB_TOKEN");
      expect(JSON.stringify(s.env ?? {}), s.name).not.toContain("secrets.");
    }
  });

  it("never puts the updater key and GITHUB_TOKEN in the same step", () => {
    for (const s of steps) {
      const env = envOf(s);
      expect(
        env.includes("TAURI_SIGNING_PRIVATE_KEY") && env.includes("GITHUB_TOKEN"),
        s.name,
      ).toBe(false);
    }
  });
});

describe("updater signing job", () => {
  const jobs = workflow.jobs as Record<
    string,
    { steps: Step[]; needs?: string | string[]; permissions?: unknown }
  >;
  const usesKey = (j: { steps: Step[] }) =>
    j.steps.some((s) => envOf(s).includes("TAURI_SIGNING_PRIVATE_KEY"));

  it("signs only in its own job on a fresh runner", () => {
    const withKey = Object.entries(jobs).filter(([, j]) => usesKey(j));
    expect(withKey.map(([n]) => n)).toEqual(["sign-updater"]);
    const job = jobs["sign-updater"];
    expect([job.needs].flat()).toContain("publish-tauri");
    expect(job.permissions).toEqual({});
    const text = JSON.stringify(job.steps);
    expect(text).not.toMatch(/pnpm (install|tauri|run)|tauri build|cargo |GITHUB_TOKEN/);
    expect(text).toContain("scripts/pinned-tauri-cli.mjs");
  });

  it("uses the CLI pinned by the lockfile and checks its integrity", () => {
    const lock = readFileSync(join(root, "pnpm-lock.yaml"), "utf8");
    const cli = lockedPackage(lock, "@tauri-apps/cli");
    expect(cli.version).toMatch(/^\d+\.\d+\.\d+$/);
    expect(cli.integrity).toMatch(/^sha512-/);
    expect(() => checkIntegrity(Buffer.from("x"), cli.integrity)).toThrow(/integrity/);
  });
});

/** Bundle files Tauri writes for each release platform. */
function fakeBuild(dir: string, files: Record<string, string[]>) {
  for (const [kind, names] of Object.entries(files)) {
    mkdirSync(join(dir, kind), { recursive: true });
    for (const n of names) {
      if (n.endsWith(".app")) mkdirSync(join(dir, kind, n), { recursive: true });
      else writeFileSync(join(dir, kind, n), "x");
    }
  }
}

describe("release assets", () => {
  it("produces the same asset set as v0.14.0 for the current version", () => {
    const v = version;
    const platforms: Record<string, { files: Record<string, string[]>; arch?: string }> = {
      macArm: { files: { dmg: [`OmniGet_${v}_aarch64.dmg`], macos: ["OmniGet.app"] }, arch: "aarch64" },
      macIntel: { files: { dmg: [`OmniGet_${v}_x64.dmg`], macos: ["OmniGet.app"] }, arch: "x86_64" },
      linuxX64: {
        files: {
          appimage: [`OmniGet_${v}_amd64.AppImage`],
          deb: [`OmniGet_${v}_amd64.deb`],
          rpm: [`OmniGet-${v}-1.x86_64.rpm`],
        },
      },
      linuxArm: {
        files: {
          appimage: [`OmniGet_${v}_aarch64.AppImage`],
          deb: [`OmniGet_${v}_arm64.deb`],
          rpm: [`OmniGet-${v}-1.aarch64.rpm`],
        },
      },
      windows: {
        files: { nsis: [`OmniGet_${v}_x64-setup.exe`], msi: [`OmniGet_${v}_x64_en-US.msi`] },
      },
    };
    const names = new Set<string>();
    for (const p of Object.values(platforms)) {
      const dir = mkdtempSync(join(tmpdir(), "omniget-rel-"));
      fakeBuild(dir, p.files);
      if (p.arch) {
        const m = macArchive(dir, v, p.arch);
        expect(m).not.toBeNull();
        writeFileSync(m!.archive, "x");
      }
      for (const f of filesToSign(dir)) writeFileSync(`${f}.sig`, "sig");
      for (const f of filesToUpload(dir)) names.add(f.split(/[\\/]/).pop()!);
      rmSync(dir, { recursive: true, force: true });
    }
    // Assets uploaded by the other workflow steps.
    const text = readFileSync(join(root, ".github/workflows/release.yml"), "utf8");
    expect(text).toContain("zsyncmake");
    for (const a of ["amd64", "aarch64"]) names.add(`OmniGet_${v}_${a}.AppImage.zsync`);
    expect(text).toContain("_x64-portable.exe");
    names.add(`omniget_${v}_x64-portable.exe`);
    const targets = (workflow.jobs["publish-tauri"].strategy.matrix.include as { cli_target: string }[])
      .map((m) => m.cli_target)
      .filter(Boolean);
    for (const t of targets) names.add(`omniget-cli-${v}-${t}.${t.includes("windows") ? "zip" : "tar.gz"}`);
    expect(text).toContain("omniget-chrome-extension-v${VERSION}.zip");
    names.add(`omniget-chrome-extension-v${v}.zip`);
    expect(text).toContain("gh release upload \"$TAG\" latest.json");
    names.add("latest.json");

    // v0.14.0 as published, minus the hand-uploaded screencast.
    const expected = [
      "OmniGet-V-1.aarch64.rpm", "OmniGet-V-1.aarch64.rpm.sig",
      "OmniGet-V-1.x86_64.rpm", "OmniGet-V-1.x86_64.rpm.sig",
      "OmniGet_V_aarch64.AppImage", "OmniGet_V_aarch64.AppImage.sig", "OmniGet_V_aarch64.AppImage.zsync",
      "OmniGet_V_aarch64.app.tar.gz", "OmniGet_V_aarch64.app.tar.gz.sig", "OmniGet_V_aarch64.dmg",
      "OmniGet_V_amd64.AppImage", "OmniGet_V_amd64.AppImage.sig", "OmniGet_V_amd64.AppImage.zsync",
      "OmniGet_V_amd64.deb", "OmniGet_V_amd64.deb.sig", "OmniGet_V_arm64.deb", "OmniGet_V_arm64.deb.sig",
      "OmniGet_V_x64-setup.exe", "OmniGet_V_x64-setup.exe.sig",
      "OmniGet_V_x64.app.tar.gz", "OmniGet_V_x64.app.tar.gz.sig", "OmniGet_V_x64.dmg",
      "OmniGet_V_x64_en-US.msi", "OmniGet_V_x64_en-US.msi.sig",
      "latest.json", "omniget-chrome-extension-vV.zip",
      "omniget-cli-V-aarch64-apple-darwin.tar.gz", "omniget-cli-V-x86_64-apple-darwin.tar.gz",
      "omniget-cli-V-x86_64-pc-windows-msvc.zip", "omniget-cli-V-x86_64-unknown-linux-gnu.tar.gz",
      "omniget_V_x64-portable.exe",
    ].map((n) => n.replace(/V/g, v));
    expect([...names].sort()).toEqual(expected.sort());
  });
});
