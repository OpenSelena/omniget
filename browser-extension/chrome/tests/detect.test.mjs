import test from "node:test";
import assert from "node:assert/strict";

import { detectSupportedMediaUrl } from "../src/detect.js";

test("flags direct YouTube watch pages as supported", () => {
  const detected = detectSupportedMediaUrl("https://www.youtube.com/watch?v=abc123");
  assert.equal(detected?.platform, "youtube");
  assert.equal(detected?.contentType, "video");
  assert.equal(detected?.supported, true);
});

test("flags YouTube channel pages as unsupported", () => {
  const detected = detectSupportedMediaUrl("https://www.youtube.com/@omniget");
  assert.equal(detected?.platform, "youtube");
  assert.equal(detected?.contentType, "profile");
  assert.equal(detected?.supported, false);
});

test("flags Instagram reels as supported", () => {
  const detected = detectSupportedMediaUrl("https://www.instagram.com/reel/abc123/");
  assert.equal(detected?.contentType, "reel");
  assert.equal(detected?.supported, true);
});

test("flags Reddit post pages as supported", () => {
  const detected = detectSupportedMediaUrl("https://www.reddit.com/r/videos/comments/abc123/example/");
  assert.equal(detected?.contentType, "post");
  assert.equal(detected?.supported, true);
});

test("keeps generic pages unsupported", () => {
  assert.equal(detectSupportedMediaUrl("https://www.google.com/search?q=omniget"), null);
});
