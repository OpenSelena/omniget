/**
 * Sanitizes cookie strings by removing CRLF and control characters to prevent header injection.
 */
export function sanitizeCookieString(raw: string): string {
  // Strip control characters (0x00-0x1F, 0x7F) including \r and \n
  return raw.replace(/[\x00-\x1F\x7F]/g, "");
}

/**
 * Determines whether course fetching should fallback to webview scraping.
 * Only errors (network, 401, parsing failures) trigger fallback.
 * Legitimate empty lists (0 courses) do NOT trigger webview scraping.
 */
export function shouldFallbackToWebview(apiResult: { success: boolean; courses?: unknown[]; error?: string }): boolean {
  return !apiResult.success;
}

/**
 * Formats a user-friendly token identity without losing information.
 */
export function formatTokenIdentity(token: string): string {
  const trimmed = token.trim();
  if (trimmed.includes("@")) {
    return trimmed;
  }
  if (trimmed.length >= 8) {
    return `Token User (${trimmed.slice(0, 6)})`;
  }
  return "Token User";
}

/**
 * Resolves API client credentials with environment variable fallback.
 */
export function resolveClientCredentials(): { clientId: string; clientSecret: string } {
  const clientId = process.env.UDEMY_CLIENT_ID || "TH96Ov3Ebo3OtgoSH5mOYzYolcowM3ycedWQDDce";
  const clientSecret = process.env.UDEMY_CLIENT_SECRET || "f2lgDUDxjFiOlVHUpwQNFUfCQPyMO0tJQMaud53PF01UKueW8enYjeEYoyVeP0bb2XVEDkJ5GLJaVTfM5QgMVz6yyXyydZdA5QhzgvG9UmCPUYaCrIVf7VpmiilfbLJc";
  return { clientId, clientSecret };
}

/**
 * Sanitizes portal name from cookie subdomains, ignoring non-portal subdomains like 'trk'.
 */
export function sanitizePortalName(sub: string): string {
  const clean = sub.trim().replace(/^\.+/, "").toLowerCase();
  switch (clean) {
    case "":
    case "www":
    case "trk":
    case "support":
    case "about":
    case "blog":
    case "img-c":
    case "s":
      return "www";
    default:
      return clean;
  }
}

/**
 * Validates Udemy API response to guarantee an error is thrown instead of swallowing
 * authorization/challenge failures into an empty list.
 */
export function parseUdemyResults(data: Record<string, any>): unknown[] {
  if (Array.isArray(data.results)) {
    return data.results;
  }
  if (typeof data.detail === "string") {
    throw new Error(`Udemy authorization error: ${data.detail}`);
  }
  throw new Error("Invalid API response: missing 'results' array");
}
