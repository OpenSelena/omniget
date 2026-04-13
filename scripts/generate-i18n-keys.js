#!/usr/bin/env node

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const i18nDir = path.join(__dirname, "..", "src", "lib", "i18n");
const enPath = path.join(i18nDir, "en.json");
const outputFile = path.join(i18nDir, "keys.ts");

function extractKeys(obj, prefix = "") {
  const keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "object" && value !== null && !Array.isArray(value)) {
      keys.push(...extractKeys(value, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

function generateKeysFile() {
  if (!fs.existsSync(enPath)) {
    console.error("Error: en.json not found at", enPath);
    process.exit(1);
  }

  const enContent = JSON.parse(fs.readFileSync(enPath, "utf-8"));
  const keys = extractKeys(enContent).sort();

  const grouped = {};
  for (const key of keys) {
    const category = key.split(".")[0];
    if (!grouped[category]) grouped[category] = [];
    grouped[category].push(key);
  }

  let output = `/**
 * i18n Translation Keys Reference
 *
 * AUTO-GENERATED FILE
 * Run: pnpm generate:i18n-keys
 *
 * This file provides autocomplete hints for translation keys.
 * Import the \`t\` function from '$lib/i18n' to use translations.
 *
 * Usage:
 * \`\`\`svelte
 * <script>
 *   import { t } from '$lib/i18n';
 * </script>
 *
 * <h1>{\$t('app.name')}</h1>
 * <p>{\$t('queue.items', { count: 5 })}</p>
 * \`\`\`
 */

export type TranslationKeys =\n`;

  const categories = Object.keys(grouped).sort();
  for (let i = 0; i < categories.length; i++) {
    const category = categories[i];
    const categoryKeys = grouped[category];

    for (let j = 0; j < categoryKeys.length; j++) {
      const key = categoryKeys[j];
      output += `  | '${key}'\n`;
    }

    if (i < categories.length - 1) {
      output += "\n";
    }
  }

  output += "  // Allows dynamic/computed keys while preserving autocomplete for known keys\n";
  output += "  | (string & {});\n";

  fs.writeFileSync(outputFile, output, "utf-8");
  console.log(`Generated ${keys.length} translation keys in keys.ts`);

  const otherLocales = fs
    .readdirSync(i18nDir)
    .filter((f) => f.endsWith(".json") && f !== "en.json");

  let hadMismatch = false;
  const enKeySet = new Set(keys);
  for (const file of otherLocales) {
    const p = path.join(i18nDir, file);
    try {
      const content = JSON.parse(fs.readFileSync(p, "utf-8"));
      const localeKeys = new Set(extractKeys(content));
      const missing = [...enKeySet].filter((k) => !localeKeys.has(k));
      const extra = [...localeKeys].filter((k) => !enKeySet.has(k));
      if (missing.length > 0 || extra.length > 0) {
        hadMismatch = true;
        console.warn(`[${file}] missing: ${missing.length}, extra: ${extra.length}`);
      }
    } catch (err) {
      hadMismatch = true;
      console.warn(`[${file}] failed to parse: ${err.message}`);
    }
  }
  if (!hadMismatch) {
    console.log(`All ${otherLocales.length} other locales in sync with en.json`);
  }
}

generateKeysFile();
