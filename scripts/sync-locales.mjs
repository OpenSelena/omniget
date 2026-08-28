#!/usr/bin/env node
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const dir = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "lib", "i18n");
const checkOnly = process.argv.includes("--check");
const source = JSON.parse(readFileSync(join(dir, "en.json"), "utf8"));

function sync(src, dst, path, stats) {
  const out = {};
  for (const key of Object.keys(src)) {
    const full = path ? `${path}.${key}` : key;
    const s = src[key];
    const d = dst?.[key];
    if (s && typeof s === "object" && !Array.isArray(s)) {
      out[key] = sync(s, d && typeof d === "object" ? d : {}, full, stats);
    } else if (d === undefined) {
      out[key] = s;
      stats.added.push(full);
    } else {
      out[key] = d;
    }
  }
  for (const key of Object.keys(dst ?? {})) {
    if (!(key in src)) stats.removed.push(path ? `${path}.${key}` : key);
  }
  return out;
}

let dirty = false;
for (const file of readdirSync(dir).filter((f) => f.endsWith(".json") && f !== "en.json").sort()) {
  const target = JSON.parse(readFileSync(join(dir, file), "utf8"));
  const stats = { added: [], removed: [] };
  const merged = sync(source, target, "", stats);
  const changed = stats.added.length > 0 || stats.removed.length > 0;
  if (changed) dirty = true;
  console.log(`${file}: +${stats.added.length} missing (filled with English) -${stats.removed.length} stale`);
  if (stats.removed.length) console.log(`  stale: ${stats.removed.slice(0, 10).join(", ")}${stats.removed.length > 10 ? ", …" : ""}`);
  if (!checkOnly && changed) writeFileSync(join(dir, file), JSON.stringify(merged, null, 2) + "\n");
}
if (checkOnly && dirty) process.exit(1);
