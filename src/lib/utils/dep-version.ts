/**
 * Utility to parse and format dependency version strings cleanly for display.
 * Strips verbose build variables (e.g. from upstream PDFium VERSION files)
 * and extracts archive metadata for tooltips.
 */
export function formatDepVersion(version: string | null | undefined): { display: string; tooltip?: string } {
  if (!version || !version.trim()) {
    return { display: "—" };
  }

  let str = version.trim();
  let tooltip: string | undefined = undefined;

  // Extract archive or package filename suffix, e.g. "(pdfium-win-x64.tgz)"
  const archiveMatch = str.match(/\(([^)]+\.(?:tgz|tar\.gz|zip|exe|dll|so|dylib))\)/i);
  if (archiveMatch) {
    tooltip = archiveMatch[1];
    str = str.replace(/\s*\([^)]+\)/g, "").trim();
  }

  // Extract BUILD=... if present
  const buildMatch = str.match(/BUILD=(\d+)/i);
  if (buildMatch) {
    str = buildMatch[1];
  } else if (str.startsWith("MAJOR=")) {
    const parts = str.split(/\s+/);
    const bPart = parts.find((p) => p.toUpperCase().startsWith("BUILD="));
    if (bPart) {
      str = bPart.split("=")[1];
    }
  }

  // Prepend 'v' only if not already starting with 'v' or 'V'
  const display = /^v/i.test(str) ? str : `v${str}`;

  return { display, tooltip };
}
