import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const manifestUrl = new URL("../manifest.json", import.meta.url);

async function readManifest() {
  return JSON.parse(await readFile(manifestUrl, "utf8"));
}

test("localizes the default action title through manifest i18n", async () => {
  const manifest = await readManifest();

  assert.equal(manifest.action.default_title, "__MSG_action_title_unsupported__");
});

test("declares the 48px toolbar icon for the inactive action state", async () => {
  const manifest = await readManifest();

  assert.equal(manifest.action.default_icon["48"], "icons/inactive-48.png");
});
