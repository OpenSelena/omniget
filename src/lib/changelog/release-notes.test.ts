import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { RELEASE_NOTES, releaseNotesFor } from "./release-notes";

const root = join(__dirname, "..", "..", "..");
const read = (p: string) => readFileSync(join(root, p), "utf8");
const i18nDir = join(root, "src/lib/i18n");
const locales = readdirSync(i18nDir).filter((f) => f.endsWith(".json"));
const load = (f: string) => JSON.parse(readFileSync(join(i18nDir, f), "utf8"));
const lookup = (data: unknown, key: string): unknown =>
  key.split(".").reduce<unknown>((node, part) => (node as Record<string, unknown>)?.[part], data);

const version: string = JSON.parse(read("package.json")).version;

describe("app version", () => {
  it("is the same in every manifest", () => {
    expect(JSON.parse(read("src-tauri/tauri.conf.json")).version).toBe(version);
    const cargoPackage = read("src-tauri/Cargo.toml").split(/^\[/m).find((s) => s.startsWith("package]"));
    expect(cargoPackage).toMatch(new RegExp(`^version = "${version.replace(/\./g, "\\.")}"$`, "m"));
    expect(read("src-tauri/Cargo.lock")).toContain(`name = "omniget"\nversion = "${version}"`);
    expect(read("src/lib/stores/changelog-store.svelte.ts")).toContain(`currentVersion = "${version}";`);
    expect(read("SECURITY.md")).toContain(`| ${version.split(".").slice(0, 2).join(".")}.x`);
    expect(read("RELEASE_NOTES_DRAFT.md").split("\n")[0]).toContain(`v${version}`);
  });

  it("is newer than the published 0.14.0", () => {
    const [maj, min] = version.split(".").map(Number);
    expect(maj > 0 || min > 14).toBe(true);
  });
});

describe("bundled release notes", () => {
  const notes = releaseNotesFor(version);

  it("exist for the current version and cover this branch", () => {
    expect(notes).toBeDefined();
    const items = notes!.sections.flatMap((s) => s.items);
    for (const topic of ["ytdlp_auto", "drm", "publisher", "windows_build", "translations", "local_network"]) {
      expect(items.some((k) => k.endsWith(`.${topic}`)), topic).toBe(true);
    }
  });

  it("are translated in all 12 locales", () => {
    expect(locales.length).toBe(12);
    const en = load("en.json");
    const keys = [
      "changelog.notes.heading",
      ...RELEASE_NOTES.flatMap((n) => n.sections.flatMap((s) => [s.title, ...s.items])),
    ];
    for (const f of locales) {
      const data = load(f);
      for (const key of keys) {
        const value = lookup(data, key);
        expect(typeof value === "string" && value.trim().length > 0, `${f} ${key}`).toBe(true);
        if (f !== "en.json") expect(value, `${f} ${key}`).not.toBe(lookup(en, key));
      }
      expect(lookup(data, "changelog.notes.heading"), f).toContain("{{version}}");
    }
  });
});
