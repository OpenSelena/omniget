import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("Desktop Security Configuration & Build Setup", () => {
  const rootDir = path.resolve(__dirname, "../..");
  const tauriConfPath = path.join(rootDir, "src-tauri/tauri.conf.json");
  const capabilitiesPath = path.join(rootDir, "src-tauri/capabilities/default.json");
  const packageJsonPath = path.join(rootDir, "package.json");

  it("should enforce a strict non-null Content Security Policy (CSP)", () => {
    const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, "utf-8"));
    const csp = tauriConf.app?.security?.csp;
    expect(csp).toBeTruthy();
    expect(typeof csp).toBe("string");
    expect(csp).toContain("default-src 'self'");
    expect(csp).not.toContain("'unsafe-eval'");
  });

  it("should not allow wildcard '**' filesystem access in assetProtocol scope", () => {
    const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, "utf-8"));
    const allowList: string[] = tauriConf.app?.security?.assetProtocol?.scope?.allow ?? [];
    expect(allowList).not.toContain("**");
    expect(allowList.length).toBeGreaterThan(0);
  });

  it("should not use unscoped 'shell:default' in default capability", () => {
    const caps = JSON.parse(fs.readFileSync(capabilitiesPath, "utf-8"));
    const permissions: (string | Record<string, unknown>)[] = caps.permissions ?? [];
    expect(permissions).not.toContain("shell:default");

    const shellOpenPermission = permissions.find(
      (p) => typeof p === "object" && p !== null && (p as any).identifier === "shell:allow-open"
    ) as any;
    expect(shellOpenPermission).toBeDefined();
    expect(shellOpenPermission.allow).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ url: "https://**" }),
        expect.objectContaining({ url: "http://**" }),
      ])
    );
  });

  it("should configure 4GB heap size in package.json build script to prevent OOM", () => {
    const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf-8"));
    const buildScript = pkg.scripts?.build ?? "";
    expect(buildScript).toContain("--max-old-space-size=4096");
  });
});
