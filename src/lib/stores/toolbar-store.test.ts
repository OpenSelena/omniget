import { readFileSync } from "fs";
import path from "path";
import { createRequire } from "module";
import { pathToFileURL } from "url";
import { compileModule } from "svelte/compiler";
import * as ts from "typescript";
import { beforeAll, describe, expect, it } from "vitest";

type ToolbarStoreModule = typeof import("./toolbar-store.svelte");

let store: ToolbarStoreModule;

beforeAll(async () => {
  const require = createRequire(import.meta.url);
  const svelteClientUrl = pathToFileURL(require.resolve("svelte/internal/client")).href;
  const sourcePath = path.resolve(__dirname, "toolbar-store.svelte.ts");
  const tsSource = readFileSync(sourcePath, "utf8");
  const jsSource = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext },
  }).outputText;

  const compiled = compileModule(jsSource, {
    filename: "toolbar-store.svelte.js",
    generate: "client",
  });

  const rewrittenCode = compiled.js.code.replace(
    /from\s+['"]svelte\/internal\/client['"]/g,
    `from ${JSON.stringify(svelteClientUrl)}`,
  );
  const dataUrl = `data:text/javascript;base64,${Buffer.from(rewrittenCode, "utf8").toString("base64")}`;
  store = (await import(/* @vite-ignore */ dataUrl)) as ToolbarStoreModule;
});

describe("toolbar-store (Svelte 5 compiled runtime)", () => {
  it("clears segments and actions when the registration cleanup function runs", () => {
    const cleanup = store.setToolbar({
      segments: [
        { id: "active", label: "Downloads", count: 1 },
        { id: "history", label: "History" },
        { id: "tools", label: "Tools" },
      ],
      activeSegment: "active",
      onSegment: () => {},
      actions: [
        {
          id: "pause-all",
          label: "Pause All",
          icon: "M6 4h4v16H6zM14 4h4v16h-4z",
          onClick: () => {},
        },
      ],
    });

    expect(store.getToolbar().segments).toHaveLength(3);
    expect(store.getToolbar().actions).toHaveLength(1);

    cleanup();

    expect(store.getToolbar().segments).toBeUndefined();
    expect(store.getToolbar().actions).toBeUndefined();
  });

  it("ignores stale cleanup calls after a newer route sets the toolbar", () => {
    const cleanupDownloads = store.setToolbar({
      segments: [{ id: "active", label: "Downloads" }],
      activeSegment: "active",
      onSegment: () => {},
    });

    const cleanupMarketplace = store.setToolbar({
      segments: [
        { id: "installed", label: "Installed", count: 3 },
        { id: "browse", label: "Browse" },
      ],
      activeSegment: "installed",
      onSegment: () => {},
    });

    cleanupDownloads();

    expect(store.getToolbar().activeSegment).toBe("installed");
    expect(store.getToolbar().segments).toHaveLength(2);

    cleanupMarketplace();

    expect(store.getToolbar().segments).toBeUndefined();
  });
});
