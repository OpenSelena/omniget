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

/** Each `actions/checkout` step with the lines of its block. */
function checkoutBlocks(text: string): string[] {
  const lines = text.split(/\r?\n/);
  const blocks: string[] = [];
  lines.forEach((line, i) => {
    if (!/uses:\s*actions\/checkout@/.test(line)) return;
    const indent = line.search(/\S/);
    const block = [line];
    for (let j = i + 1; j < lines.length; j++) {
      const next = lines[j];
      if (next.trim() === "") continue;
      if (next.search(/\S/) < indent || /^\s*-\s/.test(next)) break;
      block.push(next);
    }
    blocks.push(block.join("\n"));
  });
  return blocks;
}

describe("release secrets", () => {
  for (const name of PRIVILEGED) {
    it(`${name} does not leave the GitHub token in .git/config`, () => {
      const blocks = checkoutBlocks(workflow(name));
      expect(blocks.length).toBeGreaterThan(0);
      for (const block of blocks) expect(block).toMatch(/persist-credentials:\s*false/);
    });
  }

  it("release.yml gives SIGNPATH_API_TOKEN only to the steps that sign", () => {
    const text = workflow("release.yml");
    // Never at job level (visible to pnpm install and cargo build scripts).
    expect(text).not.toMatch(/^ {4}env:\s*\r?\n {6}SIGNPATH_API_TOKEN:/m);
    expect(text).not.toContain("env.SIGNPATH_API_TOKEN");
    const envLines = text.match(/^\s+SIGNPATH_API_TOKEN: \$\{\{ secrets\.SIGNPATH_API_TOKEN \}\}/gm) ?? [];
    expect(envLines).toHaveLength(1);
    expect(text).toMatch(/id: signpath\s*\r?\n(?:.*\r?\n){0,3}\s+env:\s*\r?\n\s+SIGNPATH_API_TOKEN:/);
  });
});

describe("winget workflow", () => {
  it("downloads a pinned wingetcreate and checks its SHA-256", () => {
    const text = workflow("winget.yml");
    expect(text).not.toContain("aka.ms/wingetcreate/latest");
    expect(text).toMatch(/Get-FileHash wingetcreate\.exe -Algorithm SHA256/);
  });
});
