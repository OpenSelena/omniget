export const LIBREGET_SCHEME = "libreget://";
const SCHEME_TAB_LIFETIME_MS = 1500;

export function buildLibregetSchemeUrl(url) {
  if (typeof url !== "string") {
    return null;
  }
  const trimmed = url.trim();
  if (!trimmed) {
    return null;
  }
  if (trimmed.startsWith(LIBREGET_SCHEME)) {
    return trimmed;
  }
  if (trimmed.startsWith("magnet:") || trimmed.startsWith("p2p:")) {
    return `libreget:${trimmed}`;
  }
  const withoutProtocol = trimmed.replace(/^https?:\/\//i, "");
  if (!withoutProtocol) {
    return null;
  }
  return `${LIBREGET_SCHEME}${withoutProtocol}`;
}

// Opens the libreget:// scheme in a hidden background tab so the OS can route
// it to the desktop app, without disturbing the user's currently focused tab.
// We close the throwaway tab shortly after creation since the page itself has
// no meaningful content (the protocol handler runs out-of-process).
export async function openLibregetScheme(
  url,
  {
    tabs = chrome?.tabs,
    schedule = (fn, ms) => setTimeout(fn, ms),
    tabLifetimeMs = SCHEME_TAB_LIFETIME_MS,
  } = {}
) {
  const schemeUrl = buildLibregetSchemeUrl(url);
  if (!schemeUrl) {
    return { ok: false, reason: "invalid-url" };
  }
  if (!tabs?.create) {
    return { ok: false, reason: "tabs-api-unavailable" };
  }
  try {
    const newTab = await tabs.create({ url: schemeUrl, active: false });
    const tabId = newTab?.id;
    if (typeof tabId === "number" && typeof tabs.remove === "function") {
      schedule(() => {
        try {
          const removeResult = tabs.remove(tabId);
          if (removeResult && typeof removeResult.catch === "function") {
            removeResult.catch(() => {});
          }
        } catch {
          // Tab may already be gone; ignore.
        }
      }, tabLifetimeMs);
    }
    return { ok: true, schemeUrl };
  } catch (error) {
    return {
      ok: false,
      reason: "tabs-create-failed",
      message: error instanceof Error ? error.message : String(error),
    };
  }
}
