import { describe, expect, it } from "vitest";
import { DEPENDENCIES_SETTINGS_HREF, settingsTarget } from "./settings-location.ts";

describe("settingsTarget", () => {
  it("opens the dependencies table from the yt-dlp banner link", () => {
    const url = new URL(DEPENDENCIES_SETTINGS_HREF, "http://app");
    expect(settingsTarget(url.search, url.hash)).toEqual({ tab: "plugins", section: "dependencies" });
  });

  it("maps the old hash-only link to the right tab", () => {
    expect(settingsTarget("", "#dependencies")).toEqual({ tab: "plugins", section: "dependencies" });
  });

  it("keeps plain tab links and ignores unknown values", () => {
    expect(settingsTarget("?tab=network", "")).toEqual({ tab: "network", section: null });
    expect(settingsTarget("?tab=nope", "#nope")).toEqual({ tab: null, section: null });
  });
});
