import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";
// @ts-expect-error lintapple ships untyped ESM root export
import { validatePolicy } from "lintapple";

describe("UI Lint Policies & Target Runner", () => {
  const rootDir = path.resolve(__dirname, "../..");
  const appPolicyPath = path.join(rootDir, "ui-policy.app.json");
  const docsPolicyPath = path.join(rootDir, "ui-policy.docs.json");

  it("validates ui-policy.app.json against lintapple and OmniGet desktop tokens", () => {
    const raw = JSON.parse(fs.readFileSync(appPolicyPath, "utf-8"));
    const validated = validatePolicy(raw);

    expect(validated.minControlSize).toBe(28);
    expect(validated.fontSizes).toEqual([10, 11, 12, 13, 15, 17, 22, 26, 32, 40]);
    expect(validated.viewports).toEqual([
      { width: 1100, height: 720 },
      { width: 800, height: 560 },
    ]);
    expect(validated.colorSchemes).toEqual(["dark", "light"]);
  });

  it("validates ui-policy.docs.json against lintapple and includes anti-slop banned copy", () => {
    const raw = JSON.parse(fs.readFileSync(docsPolicyPath, "utf-8"));
    const validated = validatePolicy(raw);

    expect(validated.viewports).toEqual([
      { width: 1280, height: 800 },
      { width: 375, height: 812 },
    ]);
    expect(validated.colorSchemes).toEqual(["dark", "light"]);
    expect(validated.bannedCopy).toEqual(
      expect.arrayContaining([
        "seamlessly",
        "seamless",
        "comprehensive",
        "empowering",
        "half the battle",
        "revolutionize",
        "delve",
        "testament",
        "elevate",
        "comprehensive suite",
        "robust powerhouse",
        "powerhouse",
        "supercharge",
        "cutting-edge",
        "game-changer",
        "unleash",
        "tapestry",
        "multifaceted",
        "unparalleled",
      ])
    );
  });

  it("resolves single Svelte routes, doc paths, --shell, and --all-themes", async () => {
    const { resolveTargets, ALL_APP_THEMES, SHELL_ROUTES, buildTauriMockInitScript } = await import(
      "../../scripts/lint-ui.mjs"
    );

    const singleRoute = resolveTargets(["/settings"]);
    expect(singleRoute.surface).toBe("app");
    expect(singleRoute.targets).toEqual(["/settings"]);
    expect(singleRoute.themes).toEqual(["dark", "light"]);

    const shellPlan = resolveTargets(["--shell"]);
    expect(shellPlan.targets).toEqual(SHELL_ROUTES);
    expect(SHELL_ROUTES).toContain("/omnidisc");

    const docShort = resolveTargets(["docs/mcp"]);
    expect(docShort.surface).toBe("docs");
    expect(docShort.targets).toEqual(["docs/mcp/index.html"]);
    expect(docShort.themes).toEqual(["dark", "light"]);

    const allThemes = resolveTargets(["/settings", "--all-themes"]);
    expect(allThemes.surface).toBe("app");
    expect(allThemes.targets).toEqual(["/settings"]);
    expect(allThemes.themes).toEqual(ALL_APP_THEMES);
    expect(ALL_APP_THEMES.length).toBe(14);

    const mockScript = buildTauriMockInitScript("dark");
    expect(mockScript).toContain("omnidisc_connect");
    expect(mockScript).toContain("plugin:courses|courses_list_platforms");
  });

  it("enforces strict zero-findings on docs, baseline diffing on app routes, and per-theme max normalization", async () => {
    const { tallyFindings, compareTargetBaseline } = await import("../../scripts/lint-ui.mjs");

    const sampleFindings = [
      { rule: "control-size", selector: "button.a" },
      { rule: "control-size", selector: "button.b" },
      { rule: "icon-button-tooltip", selector: "button.icon" },
    ];

    const counts = tallyFindings(sampleFindings);
    expect(counts).toEqual({
      "control-size": 2,
      "icon-button-tooltip": 1,
    });

    // Multi-theme findings take the max per rule across themes so --all-themes doesn't falsely multiply layout counts
    const multiThemeFindings = [
      { rule: "control-size", theme: "dark" },
      { rule: "control-size", theme: "dark" },
      { rule: "control-size", theme: "light" },
      { rule: "control-size", theme: "light" },
      { rule: "text-contrast", theme: "eink-sepia" },
      { rule: "text-contrast", theme: "eink-sepia" },
      { rule: "text-contrast", theme: "eink-sepia" },
    ];
    expect(tallyFindings(multiThemeFindings)).toEqual({
      "control-size": 2,
      "text-contrast": 3,
    });

    // App surface: passes if counts <= baseline, fails if any rule count increased
    const appPass = compareTargetBaseline("app", counts, {
      "control-size": 2,
      "icon-button-tooltip": 1,
    });
    expect(appPass.passed).toBe(true);
    expect(appPass.worse).toEqual([]);

    const appRegress = compareTargetBaseline("app", counts, {
      "control-size": 1,
      "icon-button-tooltip": 1,
    });
    expect(appRegress.passed).toBe(false);
    expect(appRegress.worse).toEqual([{ rule: "control-size", was: 1, now: 2 }]);

    // Docs surface: strict zero-findings even if a baseline object was passed
    const docsFail = compareTargetBaseline("docs", counts, {
      "control-size": 5,
      "icon-button-tooltip": 5,
    });
    expect(docsFail.passed).toBe(false);
    expect(docsFail.worse.length).toBe(2);
  });

  it("registers lint:ui and lint:ui:update in package.json and ships scripts/ui-lint-baseline.json", () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(rootDir, "package.json"), "utf-8"));
    expect(pkg.scripts?.["lint:ui"]).toBe("node scripts/lint-ui.mjs");
    expect(pkg.scripts?.["lint:ui:update"]).toBe("node scripts/lint-ui.mjs --update");

    const baselinePath = path.join(rootDir, "scripts", "ui-lint-baseline.json");
    expect(fs.existsSync(baselinePath)).toBe(true);
    const baseline = JSON.parse(fs.readFileSync(baselinePath, "utf-8"));
    expect(typeof baseline.targets).toBe("object");
  });

  it("documents targeted pnpm lint:ui workflow in AGENTS.md and .agents/skills/ui-lint/SKILL.md", () => {
    const agentsMd = fs.readFileSync(path.join(rootDir, "AGENTS.md"), "utf-8");
    expect(agentsMd).toContain("pnpm lint:ui");
    expect(agentsMd).toContain(".agents/skills/ui-lint/SKILL.md");

    const skillPath = path.join(rootDir, ".agents", "skills", "ui-lint", "SKILL.md");
    expect(fs.existsSync(skillPath)).toBe(true);
    const skillMd = fs.readFileSync(skillPath, "utf-8");
    expect(skillMd).toContain("name: ui-lint");
    expect(skillMd).toContain("ui-policy.app.json");
    expect(skillMd).toContain("ui-policy.docs.json");
  });
});



