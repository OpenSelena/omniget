import test from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_BLOCKED_HOSTS,
  isHostBlocked,
  isUrlHostBlocked,
  mergeBlocklists,
  normalizeBlocklist,
  USER_BLOCKED_HOSTS_KEY,
} from "../src/blocked-hosts.js";

test("USER_BLOCKED_HOSTS_KEY is the documented storage key", () => {
  assert.equal(USER_BLOCKED_HOSTS_KEY, "userBlockedHosts");
});

test("DEFAULT_BLOCKED_HOSTS is non-empty and frozen", () => {
  assert.ok(DEFAULT_BLOCKED_HOSTS.length > 0);
  assert.equal(Object.isFrozen(DEFAULT_BLOCKED_HOSTS), true);
});

test("normalizeBlocklist lowercases and strips wildcards and leading dots", () => {
  const out = normalizeBlocklist(["  Ads.EXAMPLE.com  ", "*.tracker.io", ".beacon.net"]);

  assert.deepEqual(out, ["ads.example.com", "tracker.io", "beacon.net"]);
});

test("normalizeBlocklist dedupes case-insensitively", () => {
  const out = normalizeBlocklist(["A.com", "a.com", " A.COM "]);

  assert.deepEqual(out, ["a.com"]);
});

test("normalizeBlocklist ignores non-strings and empty entries", () => {
  const out = normalizeBlocklist(["valid.com", null, 42, "", "   "]);

  assert.deepEqual(out, ["valid.com"]);
});

test("normalizeBlocklist returns an empty array for non-array input", () => {
  assert.deepEqual(normalizeBlocklist(null), []);
  assert.deepEqual(normalizeBlocklist(undefined), []);
  assert.deepEqual(normalizeBlocklist("not-an-array"), []);
});

test("mergeBlocklists concatenates defaults and user without duplicates", () => {
  const merged = mergeBlocklists(["a.com", "b.com"], ["b.com", "c.com"]);

  assert.deepEqual(merged.sort(), ["a.com", "b.com", "c.com"].sort());
});

test("mergeBlocklists preserves defaults when user input is empty or invalid", () => {
  assert.deepEqual(mergeBlocklists(["a.com"], null), ["a.com"]);
  assert.deepEqual(mergeBlocklists(["a.com"], []), ["a.com"]);
  assert.deepEqual(mergeBlocklists(["a.com"], "garbage"), ["a.com"]);
});

test("isHostBlocked returns true when any blocklist entry is a substring of the host", () => {
  const blocklist = ["analytics", "doubleclick.net"];

  assert.equal(isHostBlocked("www.google-analytics.com", blocklist), true);
  assert.equal(isHostBlocked("ad.doubleclick.net", blocklist), true);
});

test("isHostBlocked returns false for unrelated hosts", () => {
  const blocklist = ["doubleclick.net"];

  assert.equal(isHostBlocked("example.com", blocklist), false);
});

test("isHostBlocked returns false for empty or invalid host input", () => {
  assert.equal(isHostBlocked("", ["anything"]), false);
  assert.equal(isHostBlocked(null, ["anything"]), false);
  assert.equal(isHostBlocked(undefined, ["anything"]), false);
});

test("isUrlHostBlocked parses host and delegates to isHostBlocked", () => {
  const blocklist = ["sentry.io"];

  assert.equal(isUrlHostBlocked("https://o123.ingest.sentry.io/foo", blocklist), true);
  assert.equal(isUrlHostBlocked("https://example.com/foo", blocklist), false);
});

test("isUrlHostBlocked returns false for malformed URLs", () => {
  assert.equal(isUrlHostBlocked("not a url", DEFAULT_BLOCKED_HOSTS), false);
  assert.equal(isUrlHostBlocked("", DEFAULT_BLOCKED_HOSTS), false);
});

test("user-supplied entry can extend the default blocklist", () => {
  const merged = mergeBlocklists(DEFAULT_BLOCKED_HOSTS, ["my-tracker.example"]);

  assert.ok(merged.includes("my-tracker.example"));
  assert.equal(isHostBlocked("cdn.my-tracker.example", merged), true);
});
