import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { resolveSupportedLocale, detectSystemLocale, defaultLocale } from "./i18n";

describe("resolveSupportedLocale", () => {
  it("resolves Simplified Chinese locales to zh", () => {
    expect(resolveSupportedLocale("zh-CN")).toBe("zh");
    expect(resolveSupportedLocale("zh-Hans")).toBe("zh");
    expect(resolveSupportedLocale("zh-SG")).toBe("zh");
    expect(resolveSupportedLocale("zh")).toBe("zh");
  });

  it("resolves Traditional Chinese locales to zh-TW", () => {
    expect(resolveSupportedLocale("zh-TW")).toBe("zh-TW");
    expect(resolveSupportedLocale("zh-HK")).toBe("zh-TW");
    expect(resolveSupportedLocale("zh-MO")).toBe("zh-TW");
    expect(resolveSupportedLocale("zh-Hant")).toBe("zh-TW");
  });

  it("resolves other supported languages from regional tags", () => {
    expect(resolveSupportedLocale("pt-BR")).toBe("pt");
    expect(resolveSupportedLocale("pt-PT")).toBe("pt");
    expect(resolveSupportedLocale("ru-RU")).toBe("ru");
    expect(resolveSupportedLocale("ja-JP")).toBe("ja");
    expect(resolveSupportedLocale("es-ES")).toBe("es");
    expect(resolveSupportedLocale("fr-FR")).toBe("fr");
    expect(resolveSupportedLocale("it-IT")).toBe("it");
    expect(resolveSupportedLocale("el-GR")).toBe("el");
    expect(resolveSupportedLocale("fa-IR")).toBe("fa");
    expect(resolveSupportedLocale("lo-LA")).toBe("lo");
    expect(resolveSupportedLocale("en-US")).toBe("en");
    expect(resolveSupportedLocale("en-GB")).toBe("en");
  });

  it("returns null for unsupported or empty locales", () => {
    expect(resolveSupportedLocale(null)).toBeNull();
    expect(resolveSupportedLocale(undefined)).toBeNull();
    expect(resolveSupportedLocale("")).toBeNull();
    expect(resolveSupportedLocale("de-DE")).toBeNull();
    expect(resolveSupportedLocale("ko-KR")).toBeNull();
  });
});

describe("detectSystemLocale", () => {
  const originalNavigator = globalThis.navigator;

  afterEach(() => {
    Object.defineProperty(globalThis, "navigator", {
      value: originalNavigator,
      configurable: true,
      writable: true,
    });
  });

  it("detects Chinese from navigator.languages", () => {
    Object.defineProperty(globalThis, "navigator", {
      value: { languages: ["zh-CN", "en-US"], language: "zh-CN" },
      configurable: true,
      writable: true,
    });
    expect(detectSystemLocale()).toBe("zh");
  });

  it("falls back to defaultLocale when system language is unsupported", () => {
    Object.defineProperty(globalThis, "navigator", {
      value: { languages: ["de-DE", "ko-KR"], language: "de-DE" },
      configurable: true,
      writable: true,
    });
    expect(detectSystemLocale()).toBe(defaultLocale);
  });
});
