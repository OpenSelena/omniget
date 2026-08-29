import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve()) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => undefined)) }));

type VoiceStore = typeof import("./omnidisc-voice-store.svelte");

let store: VoiceStore;

beforeAll(async () => {
  vi.stubGlobal("$state", <T>(value: T) => value);
  vi.stubGlobal("$derived", <T>(value: T) => value);
  store = await import("./omnidisc-voice-store.svelte");
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("omnidisc voice volumes", () => {
  it("defaults to 100%, clamps to 0–200% and forgets the default", () => {
    expect(store.getVolume("u1")).toBe(1);
    store.setVolume("u1", 1.5);
    expect(store.getVolume("u1")).toBe(1.5);
    store.setVolume("u1", 9);
    expect(store.getVolume("u1")).toBe(2);
    store.setVolume("u1", -3);
    expect(store.getVolume("u1")).toBe(0);
    store.setVolume("u1", 1);
    expect(store.getVolume("u1")).toBe(1);
    store.setVolume("u2", Number.NaN);
    expect(store.getVolume("u2")).toBe(1);
  });

  it("starts idle with no session", () => {
    expect(store.getVoiceState()).toBe("idle");
    expect(store.getVoiceSession()).toBeNull();
    expect(store.isInVoiceChannel("v1")).toBe(false);
    expect(store.isSpeaking("u1")).toBe(false);
  });

  it("claims encryption only when the backend says a room key is in use", () => {
    expect(store.isE2ee()).toBe(false);
    expect(store.getIncomingCall()).toBeNull();
    expect(store.ringSecondsLeft()).toBe(0);
  });
});
