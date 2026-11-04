export type SettingsCategory =
  | "downloads"
  | "appearance"
  | "typography"
  | "network"
  | "cookies"
  | "channels"
  | "ai"
  | "plugins"
  | "advanced";

export const SETTINGS_CATEGORIES: readonly SettingsCategory[] = [
  "downloads",
  "appearance",
  "typography",
  "network",
  "cookies",
  "channels",
  "ai",
  "plugins",
  "advanced",
];

/** Sections that can be linked to, and the tab that holds each one. */
const SECTION_TABS: Record<string, SettingsCategory> = {
  dependencies: "plugins",
};

/** Link to the yt-dlp / FFmpeg install table. */
export const DEPENDENCIES_SETTINGS_HREF = "/settings?tab=plugins#dependencies";

/**
 * Tab and section to show for a settings URL. `#dependencies` alone also
 * opens the right tab (older links used only the hash).
 */
export function settingsTarget(
  search: string,
  hash: string,
): { tab: SettingsCategory | null; section: string | null } {
  const section = hash.replace(/^#/, "") || null;
  const raw = new URLSearchParams(search).get("tab");
  const tab = SETTINGS_CATEGORIES.includes(raw as SettingsCategory)
    ? (raw as SettingsCategory)
    : section && SECTION_TABS[section]
      ? SECTION_TABS[section]
      : null;
  return { tab, section: section && SECTION_TABS[section] ? section : null };
}
