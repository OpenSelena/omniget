import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { currentOs } from "$lib/platform";

const cache = new Map<string, { url: Promise<string>; mintedAt: number }>();

// Grants live 6 hours server-side; re-mint before that so a cached URL can
// never 404 mid-playback (the player reuses the URL on every seek).
const CACHE_TTL_MS = 5 * 60 * 60 * 1000;

/**
 * URL the webview can actually stream a local media file from.
 *
 * On Linux this cannot be `convertFileSrc`: WebKitGTK's media loader (the
 * GStreamer source element) performs a real HTTP request and never consults
 * Tauri's registered asset scheme, so an asset URL fails before the first
 * byte and the player spins forever — video, audio and subtitle tracks, on
 * every local file. The local bridge is a real HTTP server with Range
 * support, so Linux media plays from a short-lived grant the backend mints
 * for that one file (`media_stream_url`). Windows and macOS webviews honour
 * the asset scheme, so they keep the old behaviour.
 */
export function mediaSrc(path: string): Promise<string> {
  if (!path) return Promise.resolve("");
  if (currentOs() !== "linux") return Promise.resolve(convertFileSrc(path));
  const hit = cache.get(path);
  if (hit && Date.now() - hit.mintedAt < CACHE_TTL_MS) return hit.url;
  const url = invoke<string>("media_stream_url", { path }).catch((error) => {
    console.warn("[media] stream URL unavailable, falling back to asset protocol:", error);
    return convertFileSrc(path);
  });
  cache.set(path, { url, mintedAt: Date.now() });
  return url;
}
