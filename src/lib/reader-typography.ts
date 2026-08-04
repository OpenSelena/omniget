import { pluginInvoke } from "$lib/plugin-invoke";

export type FontKey = "system" | "serif" | "sans" | "mono" | "dm-sans" | "ibm-plex-mono";
export type Justify = "left" | "justify";

export type Typography = {
  font_family: FontKey;
  font_size: number;
  line_height: number;
  justify: Justify;
};

export const DEFAULT_TYPOGRAPHY: Typography = {
  font_family: "system",
  font_size: 16,
  line_height: 1.6,
  justify: "left",
};

export const FONT_LIMITS = {
  size: { min: 12, max: 32, step: 1 },
  line: { min: 1.2, max: 2.0, step: 0.05 },
};

const GLOBAL_KEY = "study.read.typography";

export function fontStack(key: FontKey): string {
  switch (key) {
    case "serif":
      return "'Charter', 'Georgia', 'Iowan Old Style', Cambria, serif";
    case "sans":
      return "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif";
    case "mono":
      return "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, monospace";
    case "dm-sans":
      return "'DM Sans', system-ui, sans-serif";
    case "ibm-plex-mono":
      return "'IBM Plex Mono', monospace";
    case "system":
    default:
      return "'DM Sans', system-ui, sans-serif";
  }
}

export function loadGlobal(): Typography {
  if (typeof localStorage === "undefined") return { ...DEFAULT_TYPOGRAPHY };
  const raw = localStorage.getItem(GLOBAL_KEY);
  if (!raw) return { ...DEFAULT_TYPOGRAPHY };
  try {
    const parsed = JSON.parse(raw);
    return mergeTypography(DEFAULT_TYPOGRAPHY, parsed);
  } catch {
    return { ...DEFAULT_TYPOGRAPHY };
  }
}

export function saveGlobal(t: Typography) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(GLOBAL_KEY, JSON.stringify(t));
}

export function mergeTypography(base: Typography, patch: unknown): Typography {
  const out = { ...base };
  if (!patch || typeof patch !== "object") return out;
  const p = patch as Record<string, unknown>;
  if (typeof p.font_family === "string") out.font_family = p.font_family as FontKey;
  if (typeof p.font_size === "number") {
    out.font_size = clamp(p.font_size, FONT_LIMITS.size.min, FONT_LIMITS.size.max);
  }
  if (typeof p.line_height === "number") {
    out.line_height = clamp(p.line_height, FONT_LIMITS.line.min, FONT_LIMITS.line.max);
  }
  if (p.justify === "justify" || p.justify === "left") out.justify = p.justify;
  return out;
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

export async function loadBookSettings(bookId: number): Promise<Typography | null> {
  try {
    const v = await pluginInvoke<unknown>("study", "study:read:books:get_settings", { bookId });
    if (!v || typeof v !== "object") return null;
    const tp = (v as Record<string, unknown>).typography;
    if (!tp) return null;
    return mergeTypography(DEFAULT_TYPOGRAPHY, tp);
  } catch {
    return null;
  }
}

export async function saveBookSettings(bookId: number, t: Typography): Promise<void> {
  try {
    const cur = await pluginInvoke<unknown>("study", "study:read:books:get_settings", { bookId }).catch(
      () => null,
    );
    const merged: Record<string, unknown> =
      cur && typeof cur === "object" ? { ...(cur as Record<string, unknown>) } : {};
    merged.typography = t;
    await pluginInvoke("study", "study:read:books:set_settings", {
      bookId,
      settings: merged,
    });
  } catch (e) {
    console.error("save book settings failed", e);
  }
}

export function buildIframeCss(t: Typography): string {
  return `
    html, body {
      font-family: ${fontStack(t.font_family)} !important;
      font-size: ${t.font_size}px !important;
      line-height: ${t.line_height} !important;
      text-align: ${t.justify === "justify" ? "justify" : "left"} !important;
      hyphens: ${t.justify === "justify" ? "auto" : "manual"};
      max-width: 70ch;
      margin: 0 auto;
      padding: 24px;
      box-sizing: border-box;
    }
    p, li, blockquote {
      font-size: inherit !important;
      line-height: inherit !important;
    }
    img { max-width: 100%; height: auto; }
  `;
}

export function applyIframeTypography(iframe: HTMLIFrameElement | null, t: Typography) {
  if (!iframe) return;
  try {
    const doc = iframe.contentDocument;
    if (!doc) return;
    let style = doc.getElementById("__omniget_typography__") as HTMLStyleElement | null;
    if (!style) {
      style = doc.createElement("style");
      style.id = "__omniget_typography__";
      doc.head?.appendChild(style);
    }
    style.textContent = buildIframeCss(t);
  } catch (e) {
    console.error("apply typography to iframe failed", e);
  }
}
