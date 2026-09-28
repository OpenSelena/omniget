import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

type AnnouncementStore = typeof import("./announcement-store.svelte");

let store: AnnouncementStore;

const storage = new Map<string, string>();
const mockLocalStorage = {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, String(value)),
  removeItem: (key: string) => storage.delete(key),
  clear: () => storage.clear(),
};

const mockAnnouncement = {
  id: "test-announcement-1",
  active: true,
  badge: "Update",
  title: "Test Announcement",
  body: "This is a test announcement body.",
  highlights: ["Feature A", "Feature B"],
  cta: { label: "Learn More", url: "https://example.com" },
  expires_at: null,
};

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  vi.stubGlobal("localStorage", mockLocalStorage);
  store = await import("./announcement-store.svelte");
});

beforeEach(() => {
  mockLocalStorage.clear();
  store.resetAnnouncementState();
});

afterEach(() => {
  mockLocalStorage.clear();
  store.resetAnnouncementState();
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("announcement-store", () => {
  it("initializes with dialog closed and null announcement", () => {
    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();
  });

  it("opens announcement when unread and active", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockAnnouncement,
    });

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);

    expect(store.getAnnouncementOpen()).toBe(true);
    expect(store.getActiveAnnouncement()?.id).toBe("test-announcement-1");
    expect(store.getActiveAnnouncement()?.title).toBe("Test Announcement");
  });

  it("ignores announcement if marked active: false", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ ...mockAnnouncement, active: false }),
    });

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);

    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();
  });

  it("ignores announcement if already seen in localStorage", async () => {
    mockLocalStorage.setItem(
      store.ANNOUNCEMENT_STORAGE_KEY,
      JSON.stringify(["test-announcement-1"])
    );

    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockAnnouncement,
    });

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);

    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();
  });

  it("ignores expired announcements", async () => {
    const expiredAnnouncement = {
      ...mockAnnouncement,
      expires_at: "2020-01-01T00:00:00Z",
    };

    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => expiredAnnouncement,
    });

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);

    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();
  });

  it("dismisses and stores seen id into localStorage", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockAnnouncement,
    });

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);
    expect(store.getAnnouncementOpen()).toBe(true);

    store.dismissAnnouncement(true);

    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();

    const seen = JSON.parse(
      mockLocalStorage.getItem(store.ANNOUNCEMENT_STORAGE_KEY) || "[]"
    );
    expect(seen).toContain("test-announcement-1");
  });

  it("fails silently when fetch rejects or errors", async () => {
    const mockFetch = vi.fn().mockRejectedValue(new Error("Network offline"));

    await store.checkAnnouncements("https://dummy.json", mockFetch as any);

    expect(store.getAnnouncementOpen()).toBe(false);
    expect(store.getActiveAnnouncement()).toBeNull();
  });
});
