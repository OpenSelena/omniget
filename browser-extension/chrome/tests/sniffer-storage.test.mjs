import test from "node:test";
import assert from "node:assert/strict";

import {
  DEFAULT_PAGE_TTL_MS,
  PAGE_KEY_STORAGE_PREFIX,
  classifyStoredPages,
  isExpired,
  normalizePageKey,
  pageKeyFromStorageKey,
  storageKeyForPage,
} from "../src/sniffer-storage.js";

test("normalizePageKey strips query and hash but keeps origin+path", () => {
  assert.equal(
    normalizePageKey("https://www.example.com/watch?v=123&list=abc#top"),
    "https://www.example.com/watch",
  );
});

test("normalizePageKey lowercases the host", () => {
  assert.equal(
    normalizePageKey("https://WWW.Example.com/Video/ID-42"),
    "https://www.example.com/Video/ID-42",
  );
});

test("normalizePageKey preserves path case (case-sensitive paths)", () => {
  assert.equal(
    normalizePageKey("https://example.com/User/Profile"),
    "https://example.com/User/Profile",
  );
});

test("normalizePageKey returns null for non-http(s) URLs", () => {
  assert.equal(normalizePageKey("chrome://extensions"), null);
  assert.equal(normalizePageKey("file:///tmp/a.html"), null);
  assert.equal(normalizePageKey("javascript:void(0)"), null);
  assert.equal(normalizePageKey(""), null);
  assert.equal(normalizePageKey(null), null);
  assert.equal(normalizePageKey("not-a-url"), null);
});

test("normalizePageKey defaults empty path to '/'", () => {
  assert.equal(normalizePageKey("https://example.com"), "https://example.com/");
});

test("storageKeyForPage / pageKeyFromStorageKey round-trip", () => {
  const pageKey = "https://example.com/watch";
  const storageKey = storageKeyForPage(pageKey);
  assert.ok(storageKey.startsWith(PAGE_KEY_STORAGE_PREFIX));
  assert.equal(pageKeyFromStorageKey(storageKey), pageKey);
});

test("pageKeyFromStorageKey ignores foreign keys", () => {
  assert.equal(pageKeyFromStorageKey("sniffer_enabled"), null);
  assert.equal(pageKeyFromStorageKey("media_42"), null);
  assert.equal(pageKeyFromStorageKey(null), null);
  assert.equal(pageKeyFromStorageKey(42), null);
});

test("isExpired returns true for missing or malformed timestamps", () => {
  assert.equal(isExpired(undefined, 1000), true);
  assert.equal(isExpired(NaN, 1000), true);
  assert.equal(isExpired("abc", 1000), true);
});

test("isExpired respects TTL window", () => {
  const now = 1_000_000;
  assert.equal(isExpired(now - 1000, now, 10_000), false);
  assert.equal(isExpired(now - 20_000, now, 10_000), true);
  assert.equal(isExpired(now, now, 10_000), false);
});

test("isExpired treats future timestamps as not expired", () => {
  const now = 1_000_000;
  assert.equal(isExpired(now + 5000, now, 10_000), false);
});

test("classifyStoredPages separates valid from stale and ignores unrelated keys", () => {
  const now = 5_000_000;
  const storage = {
    "sniffer_enabled": true,
    [storageKeyForPage("https://a.example.com/x")]: {
      media: [["u", { title: "t" }]],
      savedAt: now - 1000,
    },
    [storageKeyForPage("https://b.example.com/y")]: {
      media: [],
      savedAt: now - DEFAULT_PAGE_TTL_MS - 1,
    },
    [storageKeyForPage("https://c.example.com/z")]: {
      savedAt: now - 1000,
    },
    [storageKeyForPage("https://d.example.com/w")]: null,
  };
  const { valid, stale } = classifyStoredPages(storage, now);
  assert.equal(valid.length, 1);
  assert.equal(valid[0].pageKey, "https://a.example.com/x");
  assert.equal(valid[0].media.length, 1);
  assert.ok(stale.includes(storageKeyForPage("https://b.example.com/y")));
  assert.ok(stale.includes(storageKeyForPage("https://c.example.com/z")));
  assert.ok(stale.includes(storageKeyForPage("https://d.example.com/w")));
  assert.equal(stale.length, 3);
});

test("classifyStoredPages handles null or non-object input gracefully", () => {
  const { valid, stale } = classifyStoredPages(null, 1000);
  assert.deepEqual(valid, []);
  assert.deepEqual(stale, []);
});
