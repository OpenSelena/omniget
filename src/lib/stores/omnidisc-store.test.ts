import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

type OmnidiscStore = typeof import("./omnidisc-store.svelte");

let store: OmnidiscStore;

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  vi.stubGlobal("$derived", <T>(value: T) => value);
  store = await import("./omnidisc-store.svelte");
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("omnidisc instances", () => {
  it("adds an instance once per url and selects the first one", () => {
    const a = store.addInstance({ url: "https://chat.example.org", name: "Example" });
    const again = store.addInstance({ url: "https://chat.example.org", name: "Renamed" });
    expect(again.id).toBe(a.id);
    expect(again.name).toBe("Renamed");
    expect(store.getInstances()).toHaveLength(1);
    expect(store.getSelectedInstance()?.id).toBe(a.id);
  });

  it("removing the selected instance falls back to the next one", () => {
    const first = store.getInstances()[0];
    const second = store.addInstance({ url: "https://other.example.org", name: "Other" });
    store.selectInstance(first.id);
    store.removeInstance(first.id);
    expect(store.getSelectedInstance()?.id).toBe(second.id);
  });

  it("status updates keep the error only while in error", () => {
    const instance = store.getInstances()[0];
    store.setInstanceStatus(instance.id, "error", "boom");
    expect(store.getInstances()[0].error).toBe("boom");
    store.setInstanceStatus(instance.id, "connected");
    expect(store.getInstances()[0].error).toBeUndefined();
  });
});

describe("omnidisc drafts", () => {
  it("keeps one draft per channel key and drops empty ones", () => {
    store.setDraft("g1/c1", "hello");
    store.setDraft("g1/c2", "other");
    expect(store.getDraft("g1/c1")).toBe("hello");
    expect(store.getDraft("g1/c2")).toBe("other");
    store.setDraft("g1/c1", "");
    expect(store.getDraft("g1/c1")).toBe("");
    expect(store.getDraft("g1/c2")).toBe("other");
  });
});

describe("omnidisc url helpers", () => {
  it("hostLabel falls back to the raw string", () => {
    expect(store.hostLabel("https://chat.example.org:8443/x")).toBe("chat.example.org:8443");
    expect(store.hostLabel("not a url")).toBe("not a url");
  });

  it("unread count starts at zero", () => {
    expect(store.getUnreadCount()).toBe(0);
  });
});

describe("omnidisc voice states", () => {
  it("tracks who is in which voice channel and moves users between channels", () => {
    const inst = store.addInstance({ url: "https://voice.example.org", name: "Voice" });
    store.applyVoiceState(inst.id, { user_id: "u1", guild_id: "g1", channel_id: "v1", self_mute: true });
    store.applyVoiceState(inst.id, { user_id: "u2", guild_id: "g1", channel_id: "v1" });
    expect(store.getVoiceMemberCount("v1")).toBe(2);
    expect(store.getVoiceMembers("v1").find((m) => m.userId === "u1")?.selfMute).toBe(true);
    expect(store.getVoiceChannelOfUser(inst.id, "u1")).toBe("v1");
    store.applyVoiceState(inst.id, { user_id: "u1", guild_id: "g1", channel_id: "v2" });
    expect(store.getVoiceMemberCount("v1")).toBe(1);
    expect(store.getVoiceMemberCount("v2")).toBe(1);
    store.applyVoiceState(inst.id, { user_id: "u1", guild_id: "g1", channel_id: null });
    expect(store.getVoiceMemberCount("v2")).toBe(0);
    expect(store.getVoiceChannelOfUser(inst.id, "u1")).toBeNull();
    store.applyVoiceState(inst.id, { channel_id: "v1" });
    expect(store.getVoiceMemberCount("v1")).toBe(1);
  });
});
