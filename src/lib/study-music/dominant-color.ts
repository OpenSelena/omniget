import { convertFileSrc } from "@tauri-apps/api/core";

export type RGB = { r: number; g: number; b: number };

const cache = new Map<string, RGB>();
const paletteCache = new Map<string, RGB[]>();
const inFlight = new Map<string, Promise<RGB | null>>();
const paletteInFlight = new Map<string, Promise<RGB[] | null>>();

const FALLBACK: RGB = { r: 35, g: 35, b: 38 };

function clampByte(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)));
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h, s, l];
}

function hslToRgb(h: number, s: number, l: number): RGB {
  if (s === 0) {
    const v = clampByte(l * 255);
    return { r: v, g: v, b: v };
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const hue = (t: number) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  return {
    r: clampByte(hue(h + 1 / 3) * 255),
    g: clampByte(hue(h) * 255),
    b: clampByte(hue(h - 1 / 3) * 255),
  };
}

function vibrantize(rgb: RGB): RGB {
  const [h, s, l] = rgbToHsl(rgb.r, rgb.g, rgb.b);
  const boostedS = Math.min(1, s * 1.4);
  const adjustedL = Math.max(0.18, Math.min(0.42, l));
  return hslToRgb(h, boostedS, adjustedL);
}

function harmonize(rgb: RGB, targetLightness: number, satBoost: number): RGB {
  const [h, s, _l] = rgbToHsl(rgb.r, rgb.g, rgb.b);
  const sat = Math.min(1, Math.max(0.15, s * satBoost));
  return hslToRgb(h, sat, targetLightness);
}

async function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = (e) => reject(e);
    img.src = src;
  });
}

function sampleAverage(img: HTMLImageElement): RGB {
  const W = 32;
  const H = 32;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  if (!ctx) return FALLBACK;
  ctx.drawImage(img, 0, 0, W, H);
  let data: ImageData;
  try {
    data = ctx.getImageData(0, 0, W, H);
  } catch {
    return FALLBACK;
  }
  const buf = data.data;
  let r = 0;
  let g = 0;
  let b = 0;
  let count = 0;
  for (let i = 0; i < buf.length; i += 4) {
    const alpha = buf[i + 3];
    if (alpha < 200) continue;
    const rr = buf[i];
    const gg = buf[i + 1];
    const bb = buf[i + 2];
    const luma = (rr * 0.299 + gg * 0.587 + bb * 0.114) | 0;
    if (luma < 18 || luma > 245) continue;
    r += rr;
    g += gg;
    b += bb;
    count++;
  }
  if (count === 0) return FALLBACK;
  return {
    r: (r / count) | 0,
    g: (g / count) | 0,
    b: (b / count) | 0,
  };
}

type Bucket = {
  rSum: number;
  gSum: number;
  bSum: number;
  count: number;
  sat: number;
};

function samplePalette(img: HTMLImageElement, k: number): RGB[] {
  const W = 48;
  const H = 48;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  if (!ctx) return [];
  ctx.drawImage(img, 0, 0, W, H);
  let data: ImageData;
  try {
    data = ctx.getImageData(0, 0, W, H);
  } catch {
    return [];
  }
  const buf = data.data;
  const HUE_BINS = 12;
  const LIGHT_BINS = 3;
  const buckets = new Map<number, Bucket>();
  for (let i = 0; i < buf.length; i += 4) {
    const alpha = buf[i + 3];
    if (alpha < 200) continue;
    const rr = buf[i];
    const gg = buf[i + 1];
    const bb = buf[i + 2];
    const [h, s, l] = rgbToHsl(rr, gg, bb);
    if (l < 0.12 || l > 0.88) continue;
    const hueIdx = Math.min(HUE_BINS - 1, Math.floor(h * HUE_BINS));
    const lightIdx = Math.min(LIGHT_BINS - 1, Math.floor(l * LIGHT_BINS));
    const key = hueIdx * LIGHT_BINS + lightIdx;
    let b = buckets.get(key);
    if (!b) {
      b = { rSum: 0, gSum: 0, bSum: 0, count: 0, sat: 0 };
      buckets.set(key, b);
    }
    b.rSum += rr;
    b.gSum += gg;
    b.bSum += bb;
    b.count++;
    b.sat += s;
  }
  if (buckets.size === 0) return [];
  const scored = Array.from(buckets.values())
    .map((b) => ({
      rgb: {
        r: (b.rSum / b.count) | 0,
        g: (b.gSum / b.count) | 0,
        b: (b.bSum / b.count) | 0,
      },
      score: b.count * (0.4 + 0.6 * (b.sat / b.count)),
    }))
    .sort((a, b) => b.score - a.score);
  const out: RGB[] = [];
  for (const entry of scored) {
    if (out.length >= k) break;
    const [, , l] = rgbToHsl(entry.rgb.r, entry.rgb.g, entry.rgb.b);
    if (l < 0.15 || l > 0.85) continue;
    out.push(entry.rgb);
  }
  if (out.length === 0 && scored.length > 0) {
    out.push(scored[0].rgb);
  }
  return out;
}

function buildHarmonizedPalette(raw: RGB[]): RGB[] {
  if (raw.length === 0) return [];
  const dominant = vibrantize(raw[0]);
  const accentSource = raw[1] ?? raw[0];
  const accent = harmonize(accentSource, 0.42, 1.3);
  const highlightSource = raw[2] ?? raw[1] ?? raw[0];
  const highlight = harmonize(highlightSource, 0.6, 1.15);
  return [dominant, accent, highlight];
}

export async function dominantColorFromPath(path: string | null | undefined): Promise<RGB | null> {
  if (!path) return null;
  if (cache.has(path)) return cache.get(path)!;
  const existing = inFlight.get(path);
  if (existing) return existing;
  const promise = (async () => {
    try {
      const url = (() => {
        try {
          return convertFileSrc(path);
        } catch {
          return path;
        }
      })();
      const img = await loadImage(url);
      const avg = sampleAverage(img);
      const tinted = vibrantize(avg);
      cache.set(path, tinted);
      return tinted;
    } catch {
      return null;
    } finally {
      inFlight.delete(path);
    }
  })();
  inFlight.set(path, promise);
  return promise;
}

export async function palettePathOrUrl(input: string | null | undefined): Promise<RGB[] | null> {
  if (!input) return null;
  if (paletteCache.has(input)) return paletteCache.get(input)!;
  const existing = paletteInFlight.get(input);
  if (existing) return existing;
  const promise = (async () => {
    try {
      const url = (() => {
        if (/^https?:\/\//i.test(input)) return input;
        try {
          return convertFileSrc(input);
        } catch {
          return input;
        }
      })();
      const img = await loadImage(url);
      const raw = samplePalette(img, 4);
      const harmonized = buildHarmonizedPalette(raw);
      if (harmonized.length === 0) return null;
      paletteCache.set(input, harmonized);
      return harmonized;
    } catch {
      return null;
    } finally {
      paletteInFlight.delete(input);
    }
  })();
  paletteInFlight.set(input, promise);
  return promise;
}

export function rgbToCss(rgb: RGB | null): string {
  if (!rgb) return "rgb(35,35,38)";
  return `rgb(${rgb.r},${rgb.g},${rgb.b})`;
}

export function rgbToCssAlpha(rgb: RGB | null, alpha: number): string {
  if (!rgb) return `rgba(35,35,38,${alpha})`;
  return `rgba(${rgb.r},${rgb.g},${rgb.b},${alpha})`;
}
