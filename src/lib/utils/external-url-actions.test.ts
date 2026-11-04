import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { armsAutoDownload, isPrefillAction, mayAutoStart } from "./external-url-actions";

describe("external URL actions", () => {
  it("never auto-starts a link opened through omniget://", () => {
    expect(mayAutoStart("confirm", true)).toBe(false);
    expect(mayAutoStart("confirm", false)).toBe(false);
  });

  it("keeps download-on-paste for extension prefills", () => {
    expect(mayAutoStart("prefill", true)).toBe(true);
    expect(mayAutoStart("prefill", false)).toBe(false);
  });

  it("shows confirm links in the omnibox", () => {
    expect(isPrefillAction("confirm")).toBe(true);
    expect(isPrefillAction("prefill")).toBe(true);
    expect(isPrefillAction("queued")).toBe(false);
  });

  it("never arms download-on-paste for an omniget:// link put in the omnibox", () => {
    expect(armsAutoDownload("confirm", true)).toBe(false);
    expect(armsAutoDownload("prefill", true)).toBe(true);
    expect(armsAutoDownload("typed", true)).toBe(true);
    expect(armsAutoDownload("typed", false)).toBe(false);
  });

  it("the home page hands the received action to handleInput", () => {
    const page = readFileSync(resolve(__dirname, "../../routes/+page.svelte"), "utf8");
    const effect = page.slice(page.indexOf("clearPendingExternalPrefill(incoming.id)"));
    const call = effect.slice(0, effect.indexOf("});"));
    expect(call).toContain("handleInput(incoming.action)");
    expect(page).toContain("armsAutoDownload(source,");
  });
});
