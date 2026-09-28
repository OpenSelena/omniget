import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

type OpenNamiStore = typeof import("./open_nami_download_store.svelte");

let store: OpenNamiStore;

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  store = await import("./open_nami_download_store.svelte");
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("open_nami_download_store", () => {
  it("updates draft properties via setDraft without resetting others", () => {
    store.setDraft({ url: "https://instagram.com/test", contentType: "photos" });
    expect(store.getDraft().url).toBe("https://instagram.com/test");
    expect(store.getDraft().contentType).toBe("photos");

    store.setDraft({ selectedProfileUrl: "https://instagram.com/selected" });
    expect(store.getDraft().url).toBe("https://instagram.com/test");
    expect(store.getDraft().selectedProfileUrl).toBe("https://instagram.com/selected");
  });

  it("handles empty draft resets cleanly", () => {
    store.setDraft({ url: "", selectedProfileUrl: null });
    expect(store.getDraft().url).toBe("");
    expect(store.getDraft().selectedProfileUrl).toBeNull();
  });
});
