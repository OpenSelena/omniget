import test from "node:test";
import assert from "node:assert/strict";

import {
  DASH_MIN_CONTENT_LENGTH,
  MIN_CONTENT_LENGTH,
  isDashManifest,
  isHlsManifest,
  shouldDropBySize,
} from "../src/sniffer-filters.js";

test("isHlsManifest matches by URL extension and content-type", () => {
  assert.equal(isHlsManifest("https://cdn.example.com/master.m3u8", ""), true);
  assert.equal(isHlsManifest("https://cdn.example.com/MASTER.M3U8?token=x", ""), true);
  assert.equal(isHlsManifest("https://cdn.example.com/play", "application/x-mpegURL"), true);
  assert.equal(isHlsManifest("https://cdn.example.com/play", "application/vnd.apple.mpegurl"), true);
  assert.equal(isHlsManifest("https://cdn.example.com/play.mp4", "video/mp4"), false);
});

test("isDashManifest matches by URL extension and content-type", () => {
  assert.equal(isDashManifest("https://cdn.example.com/manifest.mpd", ""), true);
  assert.equal(isDashManifest("https://cdn.example.com/manifest.MPD?sig=y", ""), true);
  assert.equal(isDashManifest("https://cdn.example.com/play", "application/dash+xml"), true);
  assert.equal(isDashManifest("https://cdn.example.com/play.mp4", "video/mp4"), false);
});

test("shouldDropBySize keeps responses when content-length is unknown", () => {
  assert.equal(shouldDropBySize("https://cdn.example.com/file.mp4", "video/mp4", 0), false);
  assert.equal(shouldDropBySize("https://cdn.example.com/file.mp4", "video/mp4", null), false);
  assert.equal(shouldDropBySize("https://cdn.example.com/file.mp4", "video/mp4", undefined), false);
});

test("shouldDropBySize drops non-manifest media under 50 KB", () => {
  assert.equal(shouldDropBySize("https://cdn.example.com/tiny.mp4", "video/mp4", 1024), true);
  assert.equal(shouldDropBySize("https://cdn.example.com/big.mp4", "video/mp4", 200_000), false);
});

test("shouldDropBySize never drops HLS manifests by size", () => {
  assert.equal(shouldDropBySize("https://cdn.example.com/master.m3u8", "", 512), false);
  assert.equal(shouldDropBySize("https://cdn.example.com/small.m3u8", "application/x-mpegURL", 100), false);
  assert.equal(shouldDropBySize("https://cdn.example.com/big.m3u8", "application/x-mpegURL", 5_000_000), false);
});

test("shouldDropBySize drops DASH manifests below the DASH floor", () => {
  assert.equal(shouldDropBySize("https://cdn.example.com/error.mpd", "application/dash+xml", 128), true);
  assert.equal(shouldDropBySize("https://cdn.example.com/error.mpd", "application/dash+xml", DASH_MIN_CONTENT_LENGTH - 1), true);
});

test("shouldDropBySize keeps DASH manifests at or above the DASH floor", () => {
  assert.equal(shouldDropBySize("https://cdn.example.com/live.mpd", "application/dash+xml", DASH_MIN_CONTENT_LENGTH), false);
  assert.equal(shouldDropBySize("https://cdn.example.com/vod.mpd", "application/dash+xml", 30_000), false);
});

test("DASH floor is below the generic floor to allow legitimate small MPDs", () => {
  assert.ok(DASH_MIN_CONTENT_LENGTH < MIN_CONTENT_LENGTH);
});
