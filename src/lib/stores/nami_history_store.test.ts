import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: any) => {
    if (cmd === "nami_get_history") {
      const records = [
        {
          id: "ig_123",
          platform: "instagram",
          post_id: "123",
          author: "nature_photographer",
          url: "https://instagram.com/p/123",
          file_path: "/downloads/Nami/instagram/nature/123.jpg",
          file_size: 2048576,
          sha256: null,
          downloaded_at: 1731283200,
        },
        {
          id: "tt_456",
          platform: "tiktok",
          post_id: "456",
          author: "dance_star",
          url: "https://tiktok.com/@dance_star/video/456",
          file_path: "/downloads/Nami/tiktok/dance/456.mp4",
          file_size: 15728640,
          sha256: null,
          downloaded_at: 1731284200,
        },
        {
          id: "x_789",
          platform: "x",
          post_id: "789",
          author: "news_feed",
          url: "https://x.com/news_feed/status/789",
          file_path: "/downloads/Nami/x/news/789.png",
          file_size: 512000,
          sha256: null,
          downloaded_at: 1731285200,
        },
      ];
      if (args?.platform) {
        return records.filter((r) => r.platform === args.platform);
      }
      return records;
    }
    if (cmd === "nami_reconcile_disk") {
      return { checked: 3, removed: 1 };
    }
    if (cmd === "nami_delete_history_record") {
      return true;
    }
    if (cmd === "open_path_default") {
      return undefined;
    }
    return null;
  }),
}));

type HistoryStore = typeof import("./nami_history_store.svelte");

let store: HistoryStore;

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  store = await import("./nami_history_store.svelte");
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("nami_history_store", () => {
  it("loads history records successfully", async () => {
    await store.loadHistory();
    const records = store.getRecords();
    expect(records.length).toBe(3);
    expect(records[0].id).toBe("ig_123");
  });

  it("filters records by platform", async () => {
    store.setPlatformFilter("tiktok");
    expect(store.getPlatformFilter()).toBe("tiktok");
    const filtered = store.getFilteredRecords();
    expect(filtered.length).toBe(1);
    expect(filtered[0].platform).toBe("tiktok");
    expect(filtered[0].author).toBe("dance_star");

    // Reset to all
    store.setPlatformFilter("all");
    expect(store.getFilteredRecords().length).toBe(3);
  });

  it("filters records by search query across author, post_id and path", () => {
    store.setPlatformFilter("all");

    // Search author
    store.setSearchQuery("nature");
    expect(store.getFilteredRecords().length).toBe(1);
    expect(store.getFilteredRecords()[0].author).toBe("nature_photographer");

    // Search post_id
    store.setSearchQuery("456");
    expect(store.getFilteredRecords().length).toBe(1);
    expect(store.getFilteredRecords()[0].id).toBe("tt_456");

    // Search file extension / path
    store.setSearchQuery(".png");
    expect(store.getFilteredRecords().length).toBe(1);
    expect(store.getFilteredRecords()[0].id).toBe("x_789");

    // Clear search
    store.setSearchQuery("");
    expect(store.getFilteredRecords().length).toBe(3);
  });

  it("formats file size human-readably", () => {
    expect(store.formatFileSize(undefined)).toBe("—");
    expect(store.formatFileSize(null)).toBe("—");
    expect(store.formatFileSize(0)).toBe("0 B");
    expect(store.formatFileSize(1024)).toBe("1.0 KB");
    expect(store.formatFileSize(2048576)).toBe("2.0 MB");
    expect(store.formatFileSize(15728640)).toBe("15.0 MB");
  });

  it("reconciles disk and updates message", async () => {
    const result = await store.reconcileDisk();
    expect(result).toEqual({ checked: 3, removed: 1 });
    expect(store.getReconcileStats()).toEqual({ checked: 3, removed: 1 });
  });

  it("deletes a record from local state", async () => {
    await store.loadHistory();
    expect(store.getRecords().length).toBe(3);
    const success = await store.deleteRecord("ig_123");
    expect(success).toBe(true);
    expect(store.getRecords().some((r) => r.id === "ig_123")).toBe(false);
  });
});
