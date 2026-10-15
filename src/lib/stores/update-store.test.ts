import { describe, it, expect, vi, beforeAll, beforeEach } from "vitest";

vi.mock("$lib/updater", () => ({
  checkForUpdate: vi.fn(),
  installUpdate: vi.fn(),
}));

type UpdateStore = typeof import("./update-store.svelte");
let store: UpdateStore;
let updater: typeof import("$lib/updater");

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  store = await import("./update-store.svelte");
  updater = await import("$lib/updater");
});

describe("update-store", () => {
  beforeEach(() => {
    store.resetUpdateDismissed();
    store.setUpdateInfoForTest({ available: false });
    vi.clearAllMocks();
  });

  it("initializes with update not available and not dismissed", () => {
    expect(store.getUpdateInfo()).toEqual({ available: false });
    expect(store.isUpdateDismissed()).toBe(false);
    expect(store.getIsUpdating()).toBe(false);
  });

  it("handles dismissing and resetting dismiss state", () => {
    store.dismissUpdate();
    expect(store.isUpdateDismissed()).toBe(true);
    store.resetUpdateDismissed();
    expect(store.isUpdateDismissed()).toBe(false);
  });

  it("refreshes update info from checkForUpdate", async () => {
    vi.mocked(updater.checkForUpdate).mockResolvedValue({
      available: true,
      version: "0.12.1",
      body: "Release notes",
    });

    await store.refreshUpdateInfo();
    expect(store.getUpdateInfo()).toEqual({
      available: true,
      version: "0.12.1",
      body: "Release notes",
    });
  });

  it("tracks updating state during performUpdate", async () => {
    let updatingWhileRunning = false;
    vi.mocked(updater.installUpdate).mockImplementation(async () => {
      updatingWhileRunning = store.getIsUpdating();
    });

    const promise = store.performUpdate();
    await promise;

    expect(updatingWhileRunning).toBe(true);
    expect(store.getIsUpdating()).toBe(false);
    expect(updater.installUpdate).toHaveBeenCalledOnce();
  });

  it("resets updating state even if installUpdate throws", async () => {
    vi.mocked(updater.installUpdate).mockRejectedValue(new Error("Network failed"));

    await expect(store.performUpdate()).rejects.toThrow("Network failed");
    expect(store.getIsUpdating()).toBe(false);
  });
});
