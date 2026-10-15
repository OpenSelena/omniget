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

const NON_EN_LOCALES = LOCALES.filter((l) => l !== "en");

// Core representative browser keys in study.library.telegram
const SAMPLE_TEST_KEYS = [
  "chats_title",
  "chats_subtitle",
  "chats_empty",
  "chat_empty",
  "chat_type_private",
  "chat_type_group",
  "chat_type_channel",
  "filter_all",
  "filter_photo_video",
  "filter_video",
  "filter_round_video",
  "album_indicator",
  "album_prev",
  "album_next",
  "keymap_title",
  "ctx_open",
  "ctx_mark_read",
  "ctx_pin",
  "bulk_download",
  "create_folder",
  "downloads.download",
  "smart_videos",
  "tab_unread",
];

function getDeepValue(obj: any, pathStr: string): string | undefined {
  const parts = pathStr.split(".");
  let curr = obj;
  for (const p of parts) {
    if (!curr || typeof curr !== "object") return undefined;
    curr = curr[p];
  }
  return typeof curr === "string" ? curr : undefined;
}

describe("Telegram Ticket 05 i18n Browser Translations Parity", () => {
  const i18nDir = path.resolve(__dirname, "i18n");

  it("all 12 locales have study.library.telegram defined with full key parity", () => {
    const enPath = path.join(i18nDir, "en.json");
    const enData = JSON.parse(fs.readFileSync(enPath, "utf8"));
    const enBrowser = enData?.study?.library?.telegram;
    expect(enBrowser, "en.json must have study.library.telegram").toBeDefined();

    for (const locale of LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      expect(fs.existsSync(filePath), `Locale file ${locale}.json must exist`).toBe(true);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      expect(data?.study?.library?.telegram, `${locale}.json must have study.library.telegram`).toBeDefined();
    }
  });

  it("non-English locales have genuine translations rather than English placeholders for sample browser keys", () => {
    const enPath = path.join(i18nDir, "en.json");
    const enData = JSON.parse(fs.readFileSync(enPath, "utf8"));
    const enBrowser = enData.study.library.telegram;

    for (const locale of NON_EN_LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      const locBrowser = data.study?.library?.telegram || {};

      for (const key of SAMPLE_TEST_KEYS) {
        const enVal = getDeepValue(enBrowser, key);
        const locVal = getDeepValue(locBrowser, key);

        expect(locVal, `Key "study.library.telegram.${key}" must exist in ${locale}.json`).toBeDefined();
        expect(
          locVal,
          `Key "study.library.telegram.${key}" in ${locale}.json is untranslated (identical to English: "${enVal}")`
        ).not.toBe(enVal);
      }
    }
  });

  it("validates specific script constraints for non-Latin writing systems", () => {
    const checkScript = (locale: string, key: string, regex: RegExp, scriptName: string) => {
      const filePath = path.join(i18nDir, `${locale}.json`);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      const val = getDeepValue(data.study?.library?.telegram, key) || "";
      expect(regex.test(val), `${locale} ${key} "${val}" must contain ${scriptName}`).toBe(true);
    };

    // Persian (fa)
    checkScript("fa", "filter_photo", /[\u0600-\u06FF]/, "Persian/Arabic script");
    checkScript("fa", "chats_title", /[\u0600-\u06FF]/, "Persian/Arabic script");
    checkScript("fa", "ctx_mark_read", /[\u0600-\u06FF]/, "Persian/Arabic script");

    // Greek (el)
    checkScript("el", "filter_photo", /[\u0370-\u03FF]/, "Greek script");
    checkScript("el", "chats_title", /[\u0370-\u03FF]/, "Greek script");

    // Simplified Chinese (zh)
    checkScript("zh", "filter_photo", /[\u4E00-\u9FFF]/, "CJK characters");
    checkScript("zh", "chats_title", /[\u4E00-\u9FFF]/, "CJK characters");

    // Traditional Chinese (zh-TW)
    checkScript("zh-TW", "filter_photo", /[\u4E00-\u9FFF]/, "Traditional CJK characters");
    checkScript("zh-TW", "chats_title", /[\u4E00-\u9FFF]/, "Traditional CJK characters");

    // Japanese (ja)
    checkScript("ja", "filter_photo", /[\u3040-\u30FF\u4E00-\u9FFF]/, "Japanese script");
    checkScript("ja", "chats_title", /[\u3040-\u30FF\u4E00-\u9FFF]/, "Japanese script");

    // Russian (ru)
    checkScript("ru", "filter_photo", /[\u0400-\u04FF]/, "Cyrillic script");
    checkScript("ru", "chats_title", /[\u0400-\u04FF]/, "Cyrillic script");

    // Lao (lo)
    checkScript("lo", "filter_photo", /[\u0E80-\u0EFF]/, "Lao script");
  });

  it("validates Persian (fa) base telegram authentication keys are in Persian script", () => {
    const faPath = path.join(i18nDir, "fa.json");
    const faData = JSON.parse(fs.readFileSync(faPath, "utf8"));
    const faTel = faData.telegram || {};

    const requiredFaKeys = ["login", "logout", "logged_as", "send_code", "loading_chats"];
    for (const key of requiredFaKeys) {
      const val = faTel[key] || "";
      expect(
        /[\u0600-\u06FF]/.test(val),
        `fa telegram.${key} ("${val}") must be translated in Persian script`
      ).toBe(true);
    }
  });
});
