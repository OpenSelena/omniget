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

const REQUIRED_NAV_KEYS = [
  "manage_accounts",
  "clone_channels",
  "transfers",
  "global_search",
  "global_search_tip",
  "perf_settings",
  "perf_settings_tip",
  "refresh_chats",
  "tab_all",
  "tab_drive",
  "new_folder",
  "search_folders_placeholder",
  "search_chats_placeholder",
  "new_folder_modal_title",
  "new_folder_modal_desc",
  "folder_name_placeholder",
  "create_btn",
  "creating",
  "error_create_folder",
];

describe("Telegram Ticket 04 i18n Navigation Parity", () => {
  const i18nDir = path.resolve(__dirname, "i18n");

  it("all 12 locales contain telegram.nav with all required keys", () => {
    for (const locale of LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      expect(fs.existsSync(filePath), `Locale file ${locale}.json must exist`).toBe(true);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      expect(data.telegram, `${locale}.json must have telegram object`).toBeDefined();
      expect(data.telegram.nav, `${locale}.json must have telegram.nav`).toBeDefined();

      for (const key of REQUIRED_NAV_KEYS) {
        expect(
          typeof data.telegram.nav[key],
          `Missing or non-string key "telegram.nav.${key}" in ${locale}.json`,
        ).toBe("string");
        expect(
          data.telegram.nav[key].trim().length,
          `Empty key "telegram.nav.${key}" in ${locale}.json`,
        ).toBeGreaterThan(0);
      }
    }
  });
});
