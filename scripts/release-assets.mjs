#!/usr/bin/env node
// Lists the release files of one build so the workflow can sign and upload
// them in separate steps. The build step never sees the updater signing key
// or GITHUB_TOKEN; the signing step sees only the key; the upload step sees
// only the token.
//
//   node scripts/release-assets.mjs bundles <bundle-dir>   every file to upload
//   node scripts/release-assets.mjs sign <bundle-dir>      files that get a .sig
//   node scripts/release-assets.mjs mac-archive <bundle-dir> <version> <arch>
//       prints "<.app dir>\t<archive path>" for the updater archive to create

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Bundle folders and the file suffixes Tauri writes into each. */
const KINDS = [
  { dir: "dmg", suffixes: [".dmg"], signed: false },
  { dir: "macos", suffixes: [".app.tar.gz"], signed: true },
  { dir: "appimage", suffixes: [".AppImage"], signed: true },
  { dir: "deb", suffixes: [".deb"], signed: true },
  { dir: "rpm", suffixes: [".rpm"], signed: true },
  { dir: "nsis", suffixes: ["-setup.exe"], signed: true },
  { dir: "msi", suffixes: [".msi"], signed: true },
];

/**
 * @param {string} dir
 * @returns {string[]}
 */
function list(dir) {
  return existsSync(dir) ? readdirSync(dir).sort() : [];
}

/**
 * Files that receive an updater signature (same set createUpdaterArtifacts
 * signs: every bundle except the dmg).
 * @param {string} bundleDir
 * @returns {string[]}
 */
export function filesToSign(bundleDir) {
  return KINDS.filter((k) => k.signed).flatMap((k) =>
    list(join(bundleDir, k.dir))
      .filter((f) => k.suffixes.some((s) => f.endsWith(s)))
      .map((f) => join(bundleDir, k.dir, f)),
  );
}

/**
 * Every file to upload: the bundles plus the .sig written next to them.
 * @param {string} bundleDir
 * @returns {string[]}
 */
export function filesToUpload(bundleDir) {
  return KINDS.flatMap((k) =>
    list(join(bundleDir, k.dir))
      .filter((f) => k.suffixes.some((s) => f.endsWith(s) || f.endsWith(`${s}.sig`)))
      .map((f) => join(bundleDir, k.dir, f)),
  );
}

/**
 * The macOS updater archive, named the way the published releases and
 * latest.json expect: <Product>_<version>_<aarch64|x64>.app.tar.gz.
 * @param {string} bundleDir
 * @param {string} version
 * @param {string} arch Rust target arch (aarch64 or x86_64)
 * @returns {{ app: string, archive: string } | null}
 */
export function macArchive(bundleDir, version, arch) {
  const macos = join(bundleDir, "macos");
  const app = list(macos).find((f) => f.endsWith(".app"));
  if (!app) return null;
  const product = app.slice(0, -".app".length);
  const short = arch === "x86_64" ? "x64" : arch;
  return {
    app: join(macos, app),
    archive: join(macos, `${product}_${version}_${short}.app.tar.gz`),
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [cmd, dir, version, arch] = process.argv.slice(2);
  if (cmd === "bundles") console.log(filesToUpload(dir).join("\n"));
  else if (cmd === "sign") console.log(filesToSign(dir).join("\n"));
  else if (cmd === "mac-archive") {
    const m = macArchive(dir, version, arch);
    if (m) console.log(`${m.app}\t${m.archive}`);
  } else {
    console.error("usage: release-assets.mjs bundles|sign|mac-archive <bundle-dir> [version arch]");
    process.exit(2);
  }
}
