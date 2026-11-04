#!/usr/bin/env node
// Installs the Tauri CLI used to sign updater artifacts, outside the
// project's node_modules, at the exact version and integrity recorded in
// pnpm-lock.yaml. The signing job runs on a fresh runner that never ran the
// project build, so nothing a build script or a dependency did can reach the
// signing key.
//
//   node scripts/pinned-tauri-cli.mjs <dest-dir>   prints the path of tauri.js
//
// Only `npm pack` is used (no install scripts run) and every tarball is
// checked against the lockfile's sha512 before it is unpacked.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * Exact version and integrity of a package in pnpm-lock.yaml.
 * @param {string} lock lockfile text
 * @param {string} name package name
 * @returns {{ version: string, integrity: string }}
 */
export function lockedPackage(lock, name) {
  const esc = name.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&");
  const re = new RegExp(`^  '${esc}@([0-9][^']*)':\\n    resolution: \\{integrity: (sha512-[A-Za-z0-9+/=]+)\\}`, "m");
  const m = lock.match(re);
  if (!m) throw new Error(`${name} is not pinned in pnpm-lock.yaml`);
  return { version: m[1], integrity: m[2] };
}

/**
 * @param {Buffer} data
 * @param {string} integrity sha512-<base64>
 */
export function checkIntegrity(data, integrity) {
  const got = `sha512-${createHash("sha512").update(data).digest("base64")}`;
  if (got !== integrity) throw new Error(`integrity mismatch: expected ${integrity}, got ${got}`);
}

/** Native CLI package for the runner this runs on. */
export function nativePackage(platform = process.platform, arch = process.arch) {
  /** @type {Record<string, string>} */
  const map = {
    "linux-x64": "@tauri-apps/cli-linux-x64-gnu",
    "linux-arm64": "@tauri-apps/cli-linux-arm64-gnu",
    "darwin-arm64": "@tauri-apps/cli-darwin-arm64",
    "darwin-x64": "@tauri-apps/cli-darwin-x64",
    "win32-x64": "@tauri-apps/cli-win32-x64-msvc",
  };
  const pkg = map[`${platform}-${arch}`];
  if (!pkg) throw new Error(`no Tauri CLI build for ${platform}-${arch}`);
  return pkg;
}

/**
 * @param {string} lock
 * @param {string} name
 * @param {string} dest
 * @returns {string}
 */
function install(lock, name, dest) {
  const { version, integrity } = lockedPackage(lock, name);
  const tmp = join(dest, ".pack");
  mkdirSync(tmp, { recursive: true });
  const file = execFileSync("npm", ["pack", `${name}@${version}`, "--silent", "--pack-destination", tmp], {
    encoding: "utf8",
  })
    .trim()
    .split("\n")
    .pop() ?? "";
  const tgz = join(tmp, file);
  checkIntegrity(readFileSync(tgz), integrity);
  const target = join(dest, "node_modules", ...name.split("/"));
  mkdirSync(target, { recursive: true });
  execFileSync("tar", ["-xzf", tgz, "-C", target, "--strip-components=1"]);
  return target;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const dest = process.argv[2];
  if (!dest) {
    console.error("usage: pinned-tauri-cli.mjs <dest-dir>");
    process.exit(2);
  }
  const lock = readFileSync(join(here, "..", "pnpm-lock.yaml"), "utf8");
  const cli = install(lock, "@tauri-apps/cli", dest);
  install(lock, nativePackage(), dest);
  const js = readdirSync(cli).includes("tauri.js") ? "tauri.js" : "main.js";
  console.log(join(cli, js));
}
