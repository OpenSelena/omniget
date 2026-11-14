import { describe, expect, it } from "vitest";
import { extractUrls, isUrl } from "./librebox-controller";

describe("librebox URL detection and extraction", () => {
  it("detects clean URLs directly", () => {
    expect(isUrl("https://example.com/video")).toBe(true);
    expect(extractUrls("https://example.com/video")).toEqual(["https://example.com/video"]);
  });

  it("extracts embedded URL from Chinese mobile share text", () => {
    const shareText =
      "8.20 复制打开抖音极速版，看看【xxx的作品】https://v.douyin.com/XXXXXX/ 复制此链接，打开Dou音搜索";
    expect(extractUrls(shareText)).toEqual(["https://v.douyin.com/XXXXXX/"]);
  });

  it("extracts URL when followed immediately by Chinese punctuation without space", () => {
    const text = "来看看这个好玩的作品https://v.douyin.com/XXXXXX/，复制本段信息打开app";
    expect(extractUrls(text)).toEqual(["https://v.douyin.com/XXXXXX/"]);
  });

  it("strips trailing sentence-ending punctuation from URL", () => {
    const text = "Check out this video: https://www.bilibili.com/video/BV1GJ411x7h7.";
    expect(extractUrls(text)).toEqual(["https://www.bilibili.com/video/BV1GJ411x7h7"]);
  });

  it("extracts multiple URLs for batch processing", () => {
    const text = "Here are two links:\nhttps://example.com/1\nand https://example.com/2";
    expect(extractUrls(text)).toEqual(["https://example.com/1", "https://example.com/2"]);
  });

  it("returns empty array for pure search query", () => {
    expect(extractUrls("funny cat videos 2026")).toEqual([]);
    expect(extractUrls("tutorial for beginners")).toEqual([]);
  });

  it("handles magnet links and p2p links", () => {
    expect(extractUrls("Magnet link: magnet:?xt=urn:btih:123456789abcdef enjoy")).toEqual([
      "magnet:?xt=urn:btih:123456789abcdef",
    ]);
  });
});
