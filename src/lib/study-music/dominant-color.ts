import { convertFileSrc } from "@tauri-apps/api/core";

export type RGB = { r: number; g: number; b: number };

const cache = new Map<string, RGB>();
const inFlight = new Map<string, Promise<RGB | null>>();

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

export function rgbToCss(rgb: RGB | null): string {
  if (!rgb) return "rgb(35,35,38)";
  return `rgb(${rgb.r},${rgb.g},${rgb.b})`;
}

export function rgbToCssAlpha(rgb: RGB | null, alpha: number): string {
  if (!rgb) return `rgba(35,35,38,${alpha})`;
  return `rgba(${rgb.r},${rgb.g},${rgb.b},${alpha})`;
}
