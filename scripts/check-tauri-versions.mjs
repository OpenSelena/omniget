#!/usr/bin/env node
// Guards against the Windows source build failures from upstream issue #143:
//  1. every @tauri-apps npm package must share major.minor with its Rust
//     crate in Cargo.lock (the Tauri CLI aborts otherwise);
//  2. Cargo.lock must be internally consistent, and the crates that hand
//     D3D12 objects to each other (wgpu-hal and gpu-allocator) must use the
//     same `windows` crate version.
// Usage: node scripts/check-tauri-versions.mjs   (exit 1 on any problem)
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

/** @typedef {{ name: string, version: string, deps: string[] }} LockPackage */

/**
 * Parses Cargo.lock; each dependency is "name" or "name version".
 * @param {string} text
 * @returns {LockPackage[]}
 */
export function parseCargoLock(text) {
  const packages = [];
  for (const block of text.split(/^\[\[package\]\]$/m).slice(1)) {
    const name = block.match(/^name = "([^"]+)"$/m)?.[1];
    const version = block.match(/^version = "([^"]+)"$/m)?.[1];
    const depsBlock = block.match(/^dependencies = \[\n([\s\S]*?)^\]/m)?.[1] ?? "";
    const deps = [...depsBlock.matchAll(/^ "([^"]+)",?$/gm)].map((m) => m[1].split(" (")[0]);
    if (name && version) packages.push({ name, version, deps });
  }
  return packages;
}

/**
 * Versions of the root importer's @tauri-apps packages in pnpm-lock.yaml.
 * @param {string} text
 * @returns {Record<string, string>}
 */
export function parsePnpmTauri(text) {
  /** @type {Record<string, string>} */
  const out = {};
  const importer = text.split(/^importers:\n/m)[1]?.split(/^packages:/m)[0] ?? "";
  const re = /^ {6}'(@tauri-apps\/[\w-]+)':\n {8}specifier: .*\n {8}version: ([\d.]+)/gm;
  for (const m of importer.matchAll(re)) out[m[1]] = m[2];
  return out;
}

/** @param {string} v */
const majorMinor = (v) => v.split(".").slice(0, 2).join(".");

/** @param {string} npmName */
function crateFor(npmName) {
  const short = npmName.replace("@tauri-apps/", "");
  if (short === "api") return "tauri";
  if (short.startsWith("plugin-")) return `tauri-plugin-${short.slice(7)}`;
  return null;
}

/**
 * @param {LockPackage[]} packages
 * @param {string} ref
 */
function resolveDep(packages, ref) {
  const [name, version] = ref.split(" ");
  const matches = packages.filter((p) => p.name === name && (!version || p.version === version));
  return matches.length === 1 ? matches[0] : null;
}

/**
 * Returns a list of human-readable problems; empty means the build inputs agree.
 * @param {string} cargoLockText
 * @param {string} pnpmLockText
 * @returns {string[]}
 */
export function checkTauriVersions(cargoLockText, pnpmLockText) {
  /** @type {string[]} */
  const problems = [];
  const packages = parseCargoLock(cargoLockText);
  const npm = parsePnpmTauri(pnpmLockText);

  for (const [npmName, npmVersion] of Object.entries(npm)) {
    const crate = crateFor(npmName);
    if (!crate) continue;
    const found = packages.filter((p) => p.name === crate);
    if (found.length === 0) continue;
    for (const p of found) {
      if (majorMinor(p.version) !== majorMinor(npmVersion)) {
        problems.push(`${crate} (v${p.version}) does not match ${npmName} (v${npmVersion})`);
      }
    }
  }

  for (const p of packages) {
    for (const ref of p.deps) {
      if (!resolveDep(packages, ref)) {
        problems.push(`${p.name} ${p.version} depends on "${ref}", which Cargo.lock does not define`);
      }
    }
  }

  /** @param {string} name */
  const windowsOf = (name) =>
    packages
      .filter((p) => p.name === name)
      .flatMap((p) => p.deps.filter((d) => d.split(" ")[0] === "windows"))
      .map((d) => resolveDep(packages, d)?.version ?? d);
  const hal = windowsOf("wgpu-hal");
  const allocator = windowsOf("gpu-allocator");
  for (const v of allocator) {
    if (hal.length > 0 && !hal.includes(v)) {
      problems.push(`gpu-allocator uses windows ${v} but wgpu-hal uses windows ${hal.join(", ")}`);
    }
  }
  return problems;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const problems = checkTauriVersions(
    readFileSync(join(root, "src-tauri/Cargo.lock"), "utf8"),
    readFileSync(join(root, "pnpm-lock.yaml"), "utf8"),
  );
  for (const p of problems) console.error(`✗ ${p}`);
  if (problems.length > 0) process.exit(1);
  console.log("Tauri npm packages and Cargo.lock agree");
}
