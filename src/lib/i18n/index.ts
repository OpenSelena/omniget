import i18n from "sveltekit-i18n";
import en from "./en.json";

type Payload = [payload?: Record<string, unknown>];

const config = {
  // O inglês vai embutido no bundle principal e serve de fallback: se o
  // carregamento assíncrono do idioma falhar no webview, a UI nunca fica sem
  // texto (era o sintoma no app empacotado: todos os `$t` vazios).
  initLocale: "en",
  fallbackLocale: "en",
  translations: { en },
  loaders: [
    
    {
      locale: "en",
      key: "",
      loader: async () => (await import("./en.json")).default,
    },
	{
      locale: "ru",
      key: "",
      loader: async () => (await import("./ru.json")).default,
    },
    {
      locale: "el",
      key: "",
      loader: async () => (await import("./el.json")).default,
    },
    {
      locale: "pt",
      key: "",
      loader: async () => (await import("./pt.json")).default,
    },
    {
      locale: "zh",
      key: "",
      loader: async () => (await import("./zh.json")).default,
    },
    {
      locale: "zh-TW",
      key: "",
      loader: async () => (await import("./zh-TW.json")).default,
    },
    {
      locale: "ja",
      key: "",
      loader: async () => (await import("./ja.json")).default,
    },
    {
      locale: "it",
      key: "",
      loader: async () => (await import("./it.json")).default,
    },
    {
      locale: "fr",
      key: "",
      loader: async () => (await import("./fr.json")).default,
    },
    {
      locale: "es",
      key: "",
      loader: async () => (await import("./es.json")).default,
    },
    {
      locale: "fa",
      key: "",
      loader: async () => (await import("./fa.json")).default,
    },
    {
      locale: "lo",
      key: "",
      loader: async () => (await import("./lo.json")).default,
    },
  ],
};

export const defaultLocale = "en";

export const SUPPORTED_LOCALES = [
  "en",
  "ru",
  "el",
  "pt",
  "zh",
  "zh-TW",
  "ja",
  "it",
  "fr",
  "es",
  "fa",
  "lo",
] as const;

export function resolveSupportedLocale(lang: string | null | undefined): string | null {
  if (!lang) return null;
  const clean = lang.trim().toLowerCase();
  if (
    clean.startsWith("zh-tw") ||
    clean.startsWith("zh-hk") ||
    clean.startsWith("zh-mo") ||
    clean.startsWith("zh-hant")
  ) {
    return "zh-TW";
  }
  if (clean.startsWith("zh")) {
    return "zh";
  }
  const prefix2 = clean.slice(0, 2);
  const found = SUPPORTED_LOCALES.find((l) => l.toLowerCase() === prefix2);
  if (found) return found;
  return null;
}

export function detectSystemLocale(): string {
  if (typeof navigator !== "undefined") {
    if (Array.isArray(navigator.languages)) {
      for (const candidate of navigator.languages) {
        const resolved = resolveSupportedLocale(candidate);
        if (resolved) return resolved;
      }
    }
    if (typeof navigator.language === "string") {
      const resolved = resolveSupportedLocale(navigator.language);
      if (resolved) return resolved;
    }
  }
  return defaultLocale;
}

export const RTL_LOCALES = ["fa", "ar", "he"];

export function isRtlLocale(l: string | null | undefined): boolean {
  return !!l && RTL_LOCALES.includes(l);
}

export const { t, locale, locales, loading, loadTranslations } = new i18n<Payload>(config);
