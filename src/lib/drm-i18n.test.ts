import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const dir = join(__dirname, "i18n");
const locales = readdirSync(dir).filter((f) => f.endsWith(".json"));
const load = (f: string) => JSON.parse(readFileSync(join(dir, f), "utf8"));
const en = load("en.json");

describe("DRM messages", () => {
  it("say in every locale that DRM content cannot be downloaded", () => {
    expect(locales.length).toBe(12);
    for (const f of locales) {
      const msg: string = load(f).errors.drm_protected;
      expect(msg, f).toContain("DRM");
      expect(msg, f).toContain("OmniGet");
      for (const system of ["Widevine", "PlayReady", "FairPlay"]) {
        expect(msg, `${f} ${system}`).toContain(system);
      }
    }
  });

  it("are translated outside English", () => {
    for (const f of locales.filter((l) => l !== "en.json")) {
      const data = load(f);
      expect(data.errors.drm_protected, f).not.toBe(en.errors.drm_protected);
      expect(data.courses.sections_drm, f).not.toBe(en.courses.sections_drm);
    }
  });
});
