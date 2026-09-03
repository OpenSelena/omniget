#!/usr/bin/env node

import fs from "fs";
import os from "os";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const parent = path.resolve(root, "..");

const PLUGINS = [
  { id: "courses", repo: "omniget-plugin-courses", crate: "omniget_plugin_courses" },
  { id: "telegram", repo: "omniget-plugin-telegram", crate: "omniget_plugin_telegram" },
  { id: "convert", repo: "omniget-plugin-convert", crate: "omniget_plugin_convert" },
];

function appDataPluginsDir() {
  if (process.env.OMNIGET_DATA_DIR) {
    return path.join(process.env.OMNIGET_DATA_DIR, "plugins");
  }
  const platform = process.platform;
  if (platform === "win32") {
    const base = process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming");
    return path.join(base, "wtf.tonho.omniget", "plugins");
  }
  if (platform === "darwin") {
    return path.join(
      os.homedir(),
      "Library",
      "Application Support",
      "wtf.tonho.omniget",
      "plugins",
    );
  }
  const xdg = process.env.XDG_DATA_HOME || path.join(os.homedir(), ".local", "share");
  return path.join(xdg, "wtf.tonho.omniget", "plugins");
}

function dllFilename(crate) {
  if (process.platform === "win32") return `${crate}.dll`;
  if (process.platform === "darwin") return `lib${crate}.dylib`;
  return `lib${crate}.so`;
}

function copyIfChanged(from, to) {
  if (!fs.existsSync(from)) return false;
  if (fs.existsSync(to)) {
    const a = fs.readFileSync(from);
    const b = fs.readFileSync(to);
    if (a.equals(b)) return false;
  }
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.copyFileSync(from, to);
  return true;
}

function buildPlugin(repoDir) {
  const result = spawnSync("cargo", ["build"], {
    cwd: repoDir,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  return result.status === 0;
}

function main() {
  if (process.env.OMNIGET_SKIP_PLUGIN_DEPLOY === "1") {
    console.log("OMNIGET_SKIP_PLUGIN_DEPLOY=1 — skipping plugin deployment.");
    return;
  }

  const pluginsDir = appDataPluginsDir();
  fs.mkdirSync(pluginsDir, { recursive: true });

  const installed = [];
  let deployedAny = false;

  for (const p of PLUGINS) {
    const repoDir = path.join(parent, p.repo);
    if (!fs.existsSync(repoDir)) {
      console.warn(`[skip] ${p.repo} not found at ${repoDir}`);
      continue;
    }

    console.log(`Building ${p.id}...`);
    if (!buildPlugin(repoDir)) {
      console.warn(`[skip] ${p.id} build failed`);
      continue;
    }

    const dllName = dllFilename(p.crate);
    const dllPath = path.join(repoDir, "target", "debug", dllName);
    if (!fs.existsSync(dllPath)) {
      console.warn(`[skip] DLL not found: ${dllPath}`);
      continue;
    }

    const destDir = path.join(pluginsDir, p.id);
    fs.mkdirSync(destDir, { recursive: true });

    const changedDll = copyIfChanged(dllPath, path.join(destDir, dllName));
    const manifestSrc = path.join(repoDir, "plugin.json");
    const changedManifest = copyIfChanged(manifestSrc, path.join(destDir, "plugin.json"));

    console.log(
      `  Deployed to ${destDir}${
        changedDll || changedManifest ? "" : " (unchanged)"
      }`,
    );
    deployedAny = true;

    const now = new Date().toISOString();
    installed.push({
      id: p.id,
      version: "0.1.0",
      installed_at: now,
      updated_at: now,
      enabled: true,
      repo: `tonhowtf/${p.repo}`,
      source_release: "local",
    });
  }

  if (deployedAny) {
    const json = JSON.stringify({ plugins: installed }, null, 2);
    fs.writeFileSync(path.join(pluginsDir, "installed.json"), json);
    console.log("Done. Restart OmniGet to load plugins.");
  } else {
    console.log("No local plugin repos found — nothing to deploy.");
  }
}

main();
