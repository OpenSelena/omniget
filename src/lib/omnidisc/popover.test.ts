import { describe, expect, it } from "vitest";
import { placeMenu, type AnchorRect } from "./popover";

const VIEWPORT = { width: 1000, height: 800 };

function rect(left: number, top: number, width: number, height: number): AnchorRect {
  return { left, top, width, height, right: left + width, bottom: top + height };
}

describe("placeMenu", () => {
  it("hangs below the anchor, right edges aligned", () => {
    const place = placeMenu(rect(600, 200, 100, 30), { width: 180, height: 160 }, VIEWPORT);
    expect(place).toEqual({ left: 520, top: 234 });
  });

  it("aligns left edges when asked", () => {
    const place = placeMenu(rect(600, 200, 100, 30), { width: 180, height: 160 }, VIEWPORT, {
      align: "start",
    });
    expect(place.left).toBe(600);
  });

  it("flips above when there is no room below", () => {
    const place = placeMenu(rect(600, 700, 100, 30), { width: 180, height: 160 }, VIEWPORT);
    expect(place.top).toBe(536);
  });

  it("stays below and clamped when neither side fits", () => {
    const place = placeMenu(rect(600, 300, 100, 30), { width: 180, height: 900 }, VIEWPORT);
    expect(place.top).toBe(8);
  });

  it("keeps a wide menu inside the left edge", () => {
    const place = placeMenu(rect(20, 200, 30, 30), { width: 180, height: 160 }, VIEWPORT);
    expect(place.left).toBe(8);
  });

  it("keeps a menu inside the right edge", () => {
    const place = placeMenu(rect(960, 200, 30, 30), { width: 180, height: 160 }, VIEWPORT, {
      align: "start",
    });
    expect(place.left).toBe(812);
  });
});
