import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const lock = readFileSync(resolve(__dirname, "../../../pnpm-lock.yaml"), "utf8");

/** Versions of `name` resolved in pnpm-lock.yaml. */
function locked(name: string): string[] {
  const re = new RegExp(`^  '?${name.replace("/", "\\/")}@(\\d+\\.\\d+\\.\\d+)[^:]*'?:`, "gm");
  return [...new Set([...lock.matchAll(re)].map((m) => m[1]))];
}

function older(version: string, min: string): boolean {
  const a = version.split(".").map(Number);
  const b = min.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

describe("lockfile", () => {
  // GHSA-238p-pmpm-9mq7: katex before 0.18.2 (pulled by markmap-lib).
  it("has no katex older than 0.18.2", () => {
    const versions = locked("katex");
    expect(versions.length).toBeGreaterThan(0);
    expect(versions.filter((v) => older(v, "0.18.2"))).toEqual([]);
  });
});
