#!/usr/bin/env node

/**
 * Packages the Chrome extension into a .zip ready for Chrome Web Store upload.
 *
 * Usage:
 *   node browser-extension/chrome/scripts/package.mjs [--output path/to/output.zip]
 *
 * What it does:
 *   1. Copies browser-extension/chrome/ into a temp directory
 *   2. Strips the "key" field from manifest.json (CWS assigns its own)
 *   3. Removes dev-only files (tests/, scripts/, CHPR.md, README.md, package.json)
 *   4. Creates a .zip archive
 */

import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { execSync } from "node:child_process";

const EXTENSION_DIR = resolve(import.meta.dirname, "..");

const DEV_ONLY = ["tests", "scripts", "CHPR.md", "README.md", "package.json"];

function parseArgs() {
  const args = process.argv.slice(2);
  const outputIndex = args.indexOf("--output");
  if (outputIndex !== -1 && args[outputIndex + 1]) {
    return { output: resolve(args[outputIndex + 1]) };
  }

  const manifest = JSON.parse(readFileSync(join(EXTENSION_DIR, "manifest.json"), "utf8"));
  return { output: resolve(`omniget-chrome-extension-v${manifest.version}.zip`) };
}

function stripManifestKey(dir) {
  const manifestPath = join(dir, "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  delete manifest.key;
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
}

function removeDevFiles(dir) {
  for (const name of DEV_ONLY) {
    const target = join(dir, name);
    rmSync(target, { recursive: true, force: true });
  }
}

function createZip(sourceDir, outputPath) {
  // Use tar on Unix-like systems, PowerShell on Windows
  if (process.platform === "win32") {
    execSync(
      `powershell -Command "Compress-Archive -Path '${sourceDir}\\*' -DestinationPath '${outputPath}' -Force"`,
      { stdio: "inherit" }
    );
  } else {
    execSync(`cd "${sourceDir}" && zip -r "${outputPath}" .`, { stdio: "inherit" });
  }
}

const { output } = parseArgs();

const tempDir = mkdtempSync(join(tmpdir(), "omniget-chrome-ext-"));
const stageDir = join(tempDir, "chrome");

try {
  console.log("Copying extension files...");
  cpSync(EXTENSION_DIR, stageDir, { recursive: true });

  console.log("Stripping manifest key...");
  stripManifestKey(stageDir);

  console.log("Removing dev-only files...");
  removeDevFiles(stageDir);

  console.log(`Creating ${basename(output)}...`);
  createZip(stageDir, output);

  console.log(`Done: ${output}`);
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
