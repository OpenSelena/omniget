import { describe, expect, it } from "vitest";
import { isPrefillAction, mayAutoStart } from "./external-url-actions";

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
});
