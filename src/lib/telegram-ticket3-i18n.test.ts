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

const REQUIRED_DRAWER_KEYS = [
  "aria_label",
  "type_channel",
  "type_group",
  "type_private",
  "members_count",
  "tab_info",
  "tab_members",
  "about",
  "mute",
  "unmute",
  "pin",
  "unpin",
  "archive",
  "unarchive",
  "block",
  "unblock",
  "actions",
  "clear_history",
  "report",
  "leave_channel",
  "leave_group",
  "delete_channel",
  "delete_group",
  "filter_all",
  "filter_admins",
  "filter_bots",
  "filter_banned",
  "search_member_placeholder",
  "no_members",
  "members_total",
  "role_creator",
  "role_admin",
  "role_member",
  "role_banned",
  "role_left",
  "role_self",
  "leave_title_channel",
  "leave_title_group",
  "leave_desc",
  "leave_btn",
  "leaving",
  "delete_title_channel",
  "delete_title_group",
  "delete_warn",
  "delete_confirm",
  "delete_btn",
  "deleting",
  "clear_title",
  "clear_prompt",
  "clear_me",
  "clear_me_desc",
  "clear_all",
  "clear_all_desc",
  "clear_leave_channel",
  "clear_leave_group",
  "clear_leave_channel_desc",
  "clear_leave_group_desc",
  "clear_confirm_btn",
  "clear_applying",
  "report_title",
  "report_desc",
  "report_placeholder",
  "report_btn",
  "report_sending",
  "toast_muted",
  "toast_unmuted",
  "toast_pinned",
  "toast_unpinned",
  "toast_archived",
  "toast_unarchived",
  "toast_blocked",
  "toast_unblocked",
  "toast_left_channel",
  "toast_left_chat",
  "toast_deleted",
  "toast_history_cleared",
  "toast_history_cleared_all",
  "toast_reported",
];

const REQUIRED_CLONER_KEYS = [
  "title",
  "subtitle",
  "tab_sessions",
  "tab_new",
  "field_source",
  "select_source_placeholder",
  "field_dest",
  "dest_create_new",
  "dest_create_new_desc",
  "dest_name_placeholder",
  "dest_existing",
  "dest_existing_desc",
  "dest_select_placeholder",
  "advanced_options",
  "delay_label",
  "batch_size_label",
  "limit_enable",
  "limit_max",
  "drop_author",
  "drop_captions",
  "start_btn",
  "starting",
  "no_sessions",
  "create_new_btn",
  "status_running",
  "status_paused",
  "status_completed",
  "status_error",
  "status_cancelled",
  "meta_messages",
  "meta_failed",
  "meta_delay",
  "btn_pause",
  "btn_resume",
  "btn_cancel",
  "btn_remove",
  "toast_select_source",
  "toast_started",
];

const REQUIRED_SEARCH_KEYS = [
  "aria_label",
  "placeholder",
  "type_to_search",
  "no_results",
  "results_count",
  "kbd_hint",
];

describe("Telegram Ticket 03 i18n Dictionary Parity", () => {
  const i18nDir = path.resolve(__dirname, "i18n");

  it("all 12 locales contain telegram.channel_drawer with required keys", () => {
    for (const locale of LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      expect(fs.existsSync(filePath), `Locale file ${locale}.json must exist`).toBe(true);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      expect(data.telegram, `${locale}.json must have telegram object`).toBeDefined();
      expect(data.telegram.channel_drawer, `${locale}.json must have telegram.channel_drawer`).toBeDefined();

      for (const key of REQUIRED_DRAWER_KEYS) {
        expect(
          typeof data.telegram.channel_drawer[key],
          `Missing or non-string key "telegram.channel_drawer.${key}" in ${locale}.json`,
        ).toBe("string");
        expect(
          data.telegram.channel_drawer[key].trim().length,
          `Empty key "telegram.channel_drawer.${key}" in ${locale}.json`,
        ).toBeGreaterThan(0);
      }
    }
  });

  it("all 12 locales contain telegram.cloner with required keys", () => {
    for (const locale of LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      expect(data.telegram.cloner, `${locale}.json must have telegram.cloner`).toBeDefined();

      for (const key of REQUIRED_CLONER_KEYS) {
        expect(
          typeof data.telegram.cloner[key],
          `Missing or non-string key "telegram.cloner.${key}" in ${locale}.json`,
        ).toBe("string");
        expect(
          data.telegram.cloner[key].trim().length,
          `Empty key "telegram.cloner.${key}" in ${locale}.json`,
        ).toBeGreaterThan(0);
      }
    }
  });

  it("all 12 locales contain telegram.global_search with required keys", () => {
    for (const locale of LOCALES) {
      const filePath = path.join(i18nDir, `${locale}.json`);
      const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
      expect(data.telegram.global_search, `${locale}.json must have telegram.global_search`).toBeDefined();

      for (const key of REQUIRED_SEARCH_KEYS) {
        expect(
          typeof data.telegram.global_search[key],
          `Missing or non-string key "telegram.global_search.${key}" in ${locale}.json`,
        ).toBe("string");
        expect(
          data.telegram.global_search[key].trim().length,
          `Empty key "telegram.global_search.${key}" in ${locale}.json`,
        ).toBeGreaterThan(0);
      }
    }
  });
});
