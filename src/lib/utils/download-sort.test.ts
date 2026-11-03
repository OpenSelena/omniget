import { describe, expect, it } from "vitest";
import { sortByStartNewestFirst, sortNewestFirst } from "./download-sort";

describe("sortNewestFirst", () => {
  it("puts the most recently finished download first (#96)", () => {
    const items = [
      { id: 1, startedAt: 100, lastUpdateAt: 200 },
      { id: 2, startedAt: 150, lastUpdateAt: 900 },
      { id: 3, startedAt: 300, lastUpdateAt: 400 },
    ];
    expect(sortNewestFirst(items).map((d) => d.id)).toEqual([2, 3, 1]);
  });

  it("keeps the newest of 25 finished downloads on the first page of 20", () => {
    const items = Array.from({ length: 25 }, (_, i) => ({ id: i + 1, startedAt: i, lastUpdateAt: i }));
    const firstPage = sortNewestFirst(items).slice(0, 20);
    expect(firstPage[0].id).toBe(25);
    expect(firstPage.some((d) => d.id === 1)).toBe(false);
  });

  it("does not mutate the input and breaks ties by id", () => {
    const items = [
      { id: 1, startedAt: 5, lastUpdateAt: 5 },
      { id: 2, startedAt: 5, lastUpdateAt: 5 },
    ];
    const sorted = sortNewestFirst(items);
    expect(sorted.map((d) => d.id)).toEqual([2, 1]);
    expect(items.map((d) => d.id)).toEqual([1, 2]);
  });
});

describe("sortByStartNewestFirst", () => {
  it("ignores progress ticks so active rows do not jump", () => {
    const items = [
      { id: 1, startedAt: 100, lastUpdateAt: 999 },
      { id: 2, startedAt: 200, lastUpdateAt: 201 },
    ];
    expect(sortByStartNewestFirst(items).map((d) => d.id)).toEqual([2, 1]);
  });
});
