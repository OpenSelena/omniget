import test from "node:test";
import assert from "node:assert/strict";

import { getHlsGroupKey, getHlsVariantToken } from "../src/hls-grouping.js";

test("variant token collapses master-like names to 'master'", () => {
  assert.equal(getHlsVariantToken("master.m3u8"), "master");
  assert.equal(getHlsVariantToken("playlist.m3u8"), "master");
  assert.equal(getHlsVariantToken("index.m3u8"), "master");
  assert.equal(getHlsVariantToken("main.m3u8"), "master");
  assert.equal(getHlsVariantToken("manifest.m3u8"), "master");
});

test("variant token collapses pure resolution names to 'master'", () => {
  assert.equal(getHlsVariantToken("1080.m3u8"), "master");
  assert.equal(getHlsVariantToken("1080p.m3u8"), "master");
  assert.equal(getHlsVariantToken("720p.m3u8"), "master");
  assert.equal(getHlsVariantToken("480.m3u8"), "master");
});

test("variant token collapses quality labels to 'master'", () => {
  assert.equal(getHlsVariantToken("high.m3u8"), "master");
  assert.equal(getHlsVariantToken("hd.m3u8"), "master");
  assert.equal(getHlsVariantToken("auto.m3u8"), "master");
});

test("variant token preserves distinct video identifiers", () => {
  assert.equal(getHlsVariantToken("moviea_master.m3u8"), "moviea_master");
  assert.equal(getHlsVariantToken("movieb_master.m3u8"), "movieb_master");
  assert.equal(getHlsVariantToken("session-abc123.m3u8"), "session-abc123");
});

test("variant token is case-insensitive", () => {
  assert.equal(getHlsVariantToken("MASTER.M3U8"), "master");
  assert.equal(getHlsVariantToken("Playlist.M3U8"), "master");
});

test("variant token strips query string before matching", () => {
  assert.equal(getHlsVariantToken("master.m3u8?token=abc"), "master");
  assert.equal(getHlsVariantToken("moviea.m3u8?sig=xyz"), "moviea");
});

test("group key keeps distinct videos in the same directory separate", () => {
  const keyA = getHlsGroupKey("https://cdn.example.com/videos/moviea_master.m3u8");
  const keyB = getHlsGroupKey("https://cdn.example.com/videos/movieb_master.m3u8");
  assert.notEqual(keyA, keyB);
});

test("group key groups master with its numeric variants in the same directory", () => {
  const master = getHlsGroupKey("https://cdn.example.com/video1/master.m3u8");
  const v1080 = getHlsGroupKey("https://cdn.example.com/video1/1080p.m3u8");
  const v720 = getHlsGroupKey("https://cdn.example.com/video1/720.m3u8");
  assert.equal(master, v1080);
  assert.equal(master, v720);
});

test("group key differentiates different directories", () => {
  const keyA = getHlsGroupKey("https://cdn.example.com/videoA/master.m3u8");
  const keyB = getHlsGroupKey("https://cdn.example.com/videoB/master.m3u8");
  assert.notEqual(keyA, keyB);
});

test("group key differentiates different origins", () => {
  const keyA = getHlsGroupKey("https://cdn1.example.com/videos/master.m3u8");
  const keyB = getHlsGroupKey("https://cdn2.example.com/videos/master.m3u8");
  assert.notEqual(keyA, keyB);
});

test("group key falls back to raw url for invalid input", () => {
  assert.equal(getHlsGroupKey("not-a-url"), "not-a-url");
});
