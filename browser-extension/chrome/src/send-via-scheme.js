export const OMNIGET_SCHEME = "omniget://";

export function buildOmnigetSchemeUrl(url) {
  if (typeof url !== "string") {
    return null;
  }
  const trimmed = url.trim();
  if (!trimmed) {
    return null;
  }
  if (trimmed.startsWith(OMNIGET_SCHEME)) {
    return trimmed;
  }
  if (trimmed.startsWith("magnet:") || trimmed.startsWith("p2p:")) {
    return `omniget:${trimmed}`;
  }
  const withoutProtocol = trimmed.replace(/^https?:\/\//i, "");
  if (!withoutProtocol) {
    return null;
  }
  return `${OMNIGET_SCHEME}${withoutProtocol}`;
}

export async function openOmnigetScheme(url, { tabs } = { tabs: chrome?.tabs }) {
  const schemeUrl = buildOmnigetSchemeUrl(url);
  if (!schemeUrl) {
    return { ok: false, reason: "invalid-url" };
  }
  if (!tabs?.update) {
    return { ok: false, reason: "tabs-api-unavailable" };
  }
  try {
    await tabs.update({ url: schemeUrl });
    return { ok: true, schemeUrl };
  } catch (error) {
    return {
      ok: false,
      reason: "tabs-update-failed",
      message: error instanceof Error ? error.message : String(error),
    };
  }
}
