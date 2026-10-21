import { describe, it, expect } from "vitest";
import fs from "fs";
import path from "path";

const LOCALES = [
  "en",
  "ru",
  "el",
  "pt",
  "zh",
  "zh-TW",
  "ja",
  "it",
  "fr",
  "es",
  "fa",
  "lo",
];

const BANNED_HARDCODED_PORTUGUESE_STRINGS = [
  "Gerenciar contas",
  "Gerenciar canal",
  "Gerenciar",
  "Clonar canais",
  "Transferências",
  "Performance de download",
  "Busca global",
  "Nova pasta",
  "Todos",
  "Criando...",
  "Buscar pasta...",
  "Erro ao criar pasta",
  "Cria um canal Telegram com sufixo",
];

const TELEGRAM_COMPONENTS = [
  "src/lib/study-components/TelegramPerfPanel.svelte",
  "src/lib/study-components/TelegramSyncIndicator.svelte",
  "src/lib/study-components/TelegramAccountPanel.svelte",
  "src/lib/study-components/TelegramTransferPanel.svelte",
  "src/lib/study-components/TelegramChannelDrawer.svelte",
  "src/lib/study-components/TelegramCloneWizard.svelte",
  "src/lib/study-components/TelegramGlobalSearchModal.svelte",
  "src/routes/telegram/+page.svelte",
];

function extractPlaceholders(str: string): string[] {
  if (typeof str !== "string") return [];
  const matches = str.match(/\{\{([^}]+)\}\}/g);
  return matches ? Array.from(new Set(matches)).sort() : [];
}

describe("Telegram i18n Invariants & Safety Net", () => {
  const i18nDir = path.resolve(__dirname, "i18n");
  const enData = JSON.parse(fs.readFileSync(path.join(i18nDir, "en.json"), "utf8"));

  it("enforces 100% key parity across all 12 locale files with en.json", () => {
    function collectKeys(obj: any, prefix = "", targetSet: Set<string>): void {
      for (const k of Object.keys(obj)) {
        const full = prefix ? `${prefix}.${k}` : k;
        if (obj[k] && typeof obj[k] === "object" && !Array.isArray(obj[k])) {
          collectKeys(obj[k], full, targetSet);
        } else {
          targetSet.add(full);
        }
      }
    }

    const enKeySet = new Set<string>();
    collectKeys(enData, "", enKeySet);

    for (const locale of LOCALES) {
      if (locale === "en") continue;
      const locPath = path.join(i18nDir, `${locale}.json`);
      expect(fs.existsSync(locPath), `${locale}.json must exist`).toBe(true);
      const locData = JSON.parse(fs.readFileSync(locPath, "utf8"));
      const locKeySet = new Set<string>();
      collectKeys(locData, "", locKeySet);

      const missing: string[] = [];
      const extra: string[] = [];

      for (const k of enKeySet) {
        if (!locKeySet.has(k)) missing.push(k);
      }
      for (const k of locKeySet) {
        if (!enKeySet.has(k)) extra.push(k);
      }

      expect(missing, `Locale ${locale}.json is missing keys`).toEqual([]);
      expect(extra, `Locale ${locale}.json has extraneous keys`).toEqual([]);
    }
  }, 15000);

  it("verifies no Telegram component contains unlocalized Portuguese user-facing strings", () => {
    for (const relPath of TELEGRAM_COMPONENTS) {
      const fullPath = path.resolve(relPath);
      expect(fs.existsSync(fullPath), `${relPath} must exist`).toBe(true);
      const content = fs.readFileSync(fullPath, "utf8");

      // Strip <script> and <style> tags to only inspect markup templates
      let templateOnly = content;
      let prev: string;
      do {
        prev = templateOnly;
        templateOnly = templateOnly
          .replace(/<script\b[^>]*>[\s\S]*?<\/script\s*>/gi, "")
          .replace(/<style\b[^>]*>[\s\S]*?<\/style\s*>/gi, "");
      } while (templateOnly !== prev);

      for (const banned of BANNED_HARDCODED_PORTUGUESE_STRINGS) {
        expect(
          templateOnly.includes(banned),
          `Component ${relPath} contains hardcoded unlocalized string: "${banned}"`
        ).toBe(false);
      }
    }
  });

  it("verifies interpolation placeholder tokens match en.json across all Telegram keys", () => {
    function checkPlaceholders(enObj: any, locObj: any, locale: string, pathPrefix = "") {
      for (const k of Object.keys(enObj)) {
        const fullKey = pathPrefix ? `${pathPrefix}.${k}` : k;
        if (enObj[k] && typeof enObj[k] === "object" && !Array.isArray(enObj[k])) {
          checkPlaceholders(enObj[k], locObj?.[k] || {}, locale, fullKey);
        } else {
          const enTokens = extractPlaceholders(enObj[k]);
          const locTokens = extractPlaceholders(locObj?.[k] || "");

          // Every variable required by en.json must be present in the translated string
          // (allowing non-English languages to omit optional plurals like {{plural}})
          for (const token of enTokens) {
            if (token === "{{plural}}") continue;
            expect(
              locTokens.includes(token),
              `Locale ${locale} key "${fullKey}" is missing interpolation token "${token}". Found: [${locTokens.join(", ")}], expected: [${enTokens.join(", ")}]`
            ).toBe(true);
          }
        }
      }
    }

    for (const locale of LOCALES) {
      if (locale === "en") continue;
      const locData = JSON.parse(fs.readFileSync(path.join(i18nDir, `${locale}.json`), "utf8"));
      checkPlaceholders(enData.telegram, locData.telegram, locale, "telegram");
      checkPlaceholders(enData.study.library.telegram, locData.study.library.telegram, locale, "study.library.telegram");
    }
  });

  it("verifies Persian (fa) RTL text direction and authentic script presence", () => {
    const faPath = path.join(i18nDir, "fa.json");
    const faData = JSON.parse(fs.readFileSync(faPath, "utf8"));

    const sampleFaStrings = [
      faData.telegram.login,
      faData.telegram.logout,
      faData.telegram.chats_title,
      faData.telegram.send_code,
      faData.study.library.telegram.chats_title,
      faData.study.library.telegram.filter_photo,
      faData.study.library.telegram.filter_video,
    ];

    for (const str of sampleFaStrings) {
      expect(typeof str).toBe("string");
      expect(/[\u0600-\u06FF]/.test(str), `String "${str}" must contain Persian/Arabic RTL characters`).toBe(true);
    }
  });
});
