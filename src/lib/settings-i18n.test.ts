import { describe, it, expect } from "vitest";
import fs from "fs";
import path from "path";

const LOCALES = ["fr", "es", "it", "pt", "ru", "ja", "zh", "zh-TW", "el", "fa", "lo"];

// Words that are legitimately the same as in English: brand names,
// technical terms, URL or file examples, and cognates.
const SHARED_TERMS = new Set([
  "settings.download.telegram_plugin_section",
  "settings.download.bilibili_section",
  "settings.download.bilibili_naming_help",
  "settings.download.bilibili_cdn_hosts_placeholder",
  "settings.download.sb_cat_sponsor",
  "settings.download.sb_cat_intro",
  "settings.download.preset_compact",
  "settings.download.section_output",
  "settings.rules.value_placeholder",
  "settings.rules.quality_placeholder",
  "settings.dependencies.col_version",
  "settings.dependencies.col_action",
  "settings.dependencies.col_status",
  "settings.typography.mono_label",
  "settings.typography.preset_compact_name",
  "settings.typography.preset_tech_minimal_name",
  "settings.proxy.title",
  "settings.proxy.type",
  "settings.proxy.port",
  "settings.proxy.host",
  "settings.proxy.password",
  "settings.advanced.section_debug",
  "settings.advanced.cookies_from_browser_placeholder",
  "settings.general.title",
  "settings.group_general",
  "settings.group_media",
  "settings.cat_plugins",
  "settings.cat_downloads",
  "settings.cat_ai",
  "settings.cat_cookies",
  "settings.rpc.title",
  "settings.cookies.title",
  "settings.cookies.count_cookies",
  "settings.cookies.modal_total",
  "settings.cookies.via_label",
  "settings.cookies.import_file_title",
  "settings.cookies.paste_content_placeholder",
  "settings.cookies.paste_source_placeholder",
]);

type Tree = { [key: string]: string | Tree };

function load(locale: string): Map<string, string> {
  const file = path.resolve(__dirname, "i18n", `${locale}.json`);
  const out = new Map<string, string>();
  const walk = (node: Tree, prefix: string) => {
    for (const [k, v] of Object.entries(node)) {
      const key = prefix ? `${prefix}.${k}` : k;
      if (typeof v === "string") out.set(key, v);
      else walk(v, key);
    }
  };
  walk(JSON.parse(fs.readFileSync(file, "utf8")), "");
  return out;
}

const en = load("en");
const scoped = [...en.keys()].filter(
  (k) =>
    (k.startsWith("settings.") || k.startsWith("home.")) &&
    !k.startsWith("settings.appearance.lang_"),
);

describe("settings and home strings are translated", () => {
  for (const locale of LOCALES) {
    it(`${locale} has no English leftovers`, () => {
      const strings = load(locale);
      const leftovers = scoped.filter(
        (k) => strings.get(k) === en.get(k) && !SHARED_TERMS.has(k),
      );
      expect(leftovers).toEqual([]);
    });
  }

  it("every locale names Traditional Chinese in its own script", () => {
    for (const locale of ["en", ...LOCALES]) {
      expect(load(locale).get("settings.appearance.lang_zh_tw")).toBe("繁體中文");
    }
  });
});

describe("French settings use proper accents", () => {
  const fr = load("fr");

  it("category names are accented", () => {
    expect(fr.get("settings.cat_downloads")).toBe("Téléchargements");
    expect(fr.get("settings.cat_network")).toBe("Réseau");
    expect(fr.get("settings.cat_advanced")).toBe("Avancé");
    expect(fr.get("settings.cat_channels")).toBe("Chaînes");
    expect(fr.get("settings.rpc.advanced")).toBe("Avancé");
  });

  it("no unaccented spellings remain", () => {
    const unaccented =
      /\b(Telecharg\w*|telecharg\w*|Reseau|reseau|Chaines?|chaines?|Avance|Parametres|parametres|Qualite|qualite|Echec|Reessayer|Desactiver|Redemarrez|activite|Systeme)\b/;
    const offenders = [...fr.entries()]
      .filter(([k, v]) => {
        if (!/^(settings|home|toast|omnibox|marketplace|onboarding)\./.test(k)) return false;
        return unaccented.test(v);
      })
      .map(([k]) => k);
    expect(offenders).toEqual([]);
  });
});
