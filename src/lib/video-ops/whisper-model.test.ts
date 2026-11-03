import { describe, expect, it } from "vitest";
import { pickWhisperModel } from "./whisper-model";

describe("pickWhisperModel", () => {
  it("returns null when no model is installed", () => {
    expect(pickWhisperModel([{ id: "tiny-q5_1", installed: false }])).toBeNull();
  });
  it("prefers the turbo model over smaller ones", () => {
    expect(
      pickWhisperModel([
        { id: "tiny-q5_1", installed: true },
        { id: "large-v3-turbo-q5_0", installed: true },
      ]),
    ).toBe("large-v3-turbo-q5_0");
  });
  it("falls back to an unknown installed model", () => {
    expect(pickWhisperModel([{ id: "custom", installed: true }])).toBe("custom");
  });
});
