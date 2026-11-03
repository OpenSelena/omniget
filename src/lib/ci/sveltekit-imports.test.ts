import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

// SvelteKit 3 dropped the implicit `$lib` alias and the `$app/stores` module.
// The app uses the `#lib` / `#components` subpath imports from package.json,
// which need an explicit file extension to resolve.
const ROOT = resolve(__dirname, "../../..");
const SRC = join(ROOT, "src");

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) return sourceFiles(full);
    return /\.(ts|js|svelte)$/.test(name) ? [full] : [];
  });
}

const SPECIFIER = /["'`]((?:\$lib|\$components|#lib|#components|\$app\/stores)(?:\/[^"'`?]*)?)[?"'`]/g;

function specifiers(): { file: string; spec: string }[] {
  return sourceFiles(SRC)
    .filter((file) => file !== __filename)
    .flatMap((file) =>
      [...readFileSync(file, "utf8").matchAll(SPECIFIER)].map((m) => ({ file, spec: m[1] })),
    );
}

describe("SvelteKit 3 imports", () => {
  it("no source file uses the removed $lib, $components or $app/stores specifiers", () => {
    const legacy = specifiers().filter(({ spec }) => spec.startsWith("$"));
    expect(legacy.map(({ file, spec }) => `${file}: ${spec}`)).toEqual([]);
  });

  it("every #lib / #components import points at an existing file", () => {
    const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
    expect(pkg.imports).toMatchObject({
      "#lib/*": "./src/lib/*",
      "#components/*": "./src/components/*",
    });
    const missing = specifiers()
      .filter(({ spec }) => spec.startsWith("#"))
      .filter(({ spec }) => {
        const target = spec.replace(/^#lib/, "src/lib").replace(/^#components/, "src/components");
        return !existsSync(join(ROOT, target)) || statSync(join(ROOT, target)).isDirectory();
      });
    expect(missing.map(({ file, spec }) => `${file}: ${spec}`)).toEqual([]);
  });
});
