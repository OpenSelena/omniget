#!/usr/bin/env node
// Prints the Tauri config override used by the release build.
//
// The updater endpoint and public key in tauri.conf.json still point at the
// upstream project. A fork sets its own with these build-time variables
// (GitHub repository variables in CI; the public key is not secret):
//
//   OMNIGET_UPDATER_ENDPOINT  URL of latest.json (https only)
//   OMNIGET_UPDATER_PUBKEY    base64 minisign public key from `tauri signer generate`
//
// Set both or neither: with neither, tauri.conf.json's values stay; with only
// one, the build fails.
//
//   node scripts/updater-config.mjs > src-tauri/tauri.release.conf.json
//   pnpm tauri build --config src-tauri/tauri.release.conf.json

import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * @param {Record<string, string | undefined>} env
 * @param {Record<string, any>} base the static CI override (tauri.ci-build.conf.json)
 * @returns {Record<string, any>}
 */
export function updaterConfig(env, base = {}) {
  const out = structuredClone(base);
  const endpoint = (env.OMNIGET_UPDATER_ENDPOINT ?? "").trim();
  const pubkey = (env.OMNIGET_UPDATER_PUBKEY ?? "").trim();
  if (!endpoint && !pubkey) return out;
  // One without the other gives an updater that silently never verifies.
  if (!endpoint || !pubkey) {
    throw new Error("Set both OMNIGET_UPDATER_ENDPOINT and OMNIGET_UPDATER_PUBKEY, or neither");
  }
  /** @type {Record<string, any>} */
  const updater = {};
  if (endpoint) {
    let url;
    try {
      url = new URL(endpoint);
    } catch {
      throw new Error("OMNIGET_UPDATER_ENDPOINT is not a valid URL");
    }
    if (url.protocol !== "https:") throw new Error("OMNIGET_UPDATER_ENDPOINT must use https");
    updater.endpoints = [endpoint];
  }
  if (pubkey) {
    if (!/^[A-Za-z0-9+/]+=*$/.test(pubkey)) throw new Error("OMNIGET_UPDATER_PUBKEY must be base64");
    updater.pubkey = pubkey;
  }
  out.plugins = { ...(out.plugins ?? {}), updater: { ...(out.plugins?.updater ?? {}), ...updater } };
  return out;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const base = JSON.parse(readFileSync(join(here, "..", "src-tauri", "tauri.ci-build.conf.json"), "utf8"));
  console.log(JSON.stringify(updaterConfig(process.env, base), null, 2));
}
