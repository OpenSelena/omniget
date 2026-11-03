import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

// Workflows that hold release or signing secrets.
const PRIVILEGED = ["release.yml", "signpath-test.yml"];

function workflow(name: string): string {
  return readFileSync(resolve(__dirname, "../../../.github/workflows", name), "utf8");
}

function usesLines(text: string): string[] {
  return [...text.matchAll(/^\s*(?:-\s*)?uses:\s*(\S+)/gm)].map((m) => m[1]);
}

describe("privileged workflows", () => {
  for (const name of PRIVILEGED) {
    it(`${name} pins third-party actions to a full commit SHA`, () => {
      const unpinned = usesLines(workflow(name)).filter(
        (ref) => !ref.startsWith("actions/") && !/@[0-9a-f]{40}$/.test(ref),
      );
      expect(unpinned).toEqual([]);
    });

    it(`${name} does not expand the tag name inside scripts`, () => {
      expect(workflow(name)).not.toContain("${{ github.ref_name }}");
    });
  }
});

describe("winget workflow", () => {
  it("downloads a pinned wingetcreate and checks its SHA-256", () => {
    const text = workflow("winget.yml");
    expect(text).not.toContain("aka.ms/wingetcreate/latest");
    expect(text).toMatch(/Get-FileHash wingetcreate\.exe -Algorithm SHA256/);
  });
});
