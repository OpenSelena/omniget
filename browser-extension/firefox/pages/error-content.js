const ERROR_PAGE_MESSAGES = Object.freeze({
  error_page_document_title: "LibreGet Extension",
  error_page_eyebrow: "LibreGet Extension",
  error_page_install_cta: "Install LibreGet",
  error_page_open_extensions: "Open extension settings",
  error_host_missing_title: "Open LibreGet once to finish setup",
  error_host_missing_body: "Your browser could not find the LibreGet bridge on this computer yet.",
  error_host_missing_detail:
    "Install LibreGet if needed, then launch the desktop app once and click the extension again.",
  error_invalid_url_title: "This page URL cannot be sent to LibreGet",
  error_invalid_url_body:
    "The current page is not a supported media page for the LibreGet extension.",
  error_invalid_url_detail:
    "Try again from a direct video, reel, post, playlist, or course page.",
  error_launch_failed_title: "LibreGet could not be launched from your browser",
  error_launch_failed_body:
    "The extension talked to the native host, but the desktop app did not start correctly.",
  error_launch_failed_detail:
    "Check that LibreGet is installed and not blocked by your system, then try again.",
});

const ERROR_CODES = Object.freeze({
  HOST_MISSING: Object.freeze({
    title: "error_host_missing_title",
    body: "error_host_missing_body",
    detail: "error_host_missing_detail",
  }),
  INVALID_URL: Object.freeze({
    title: "error_invalid_url_title",
    body: "error_invalid_url_body",
    detail: "error_invalid_url_detail",
  }),
  LAUNCH_FAILED: Object.freeze({
    title: "error_launch_failed_title",
    body: "error_launch_failed_body",
    detail: "error_launch_failed_detail",
  }),
});

function defaultGetMessage(name) {
  if (typeof chrome === "undefined" || !chrome.i18n?.getMessage) {
    return "";
  }

  return chrome.i18n.getMessage(name);
}

function resolveMessage(name, getMessage) {
  return getMessage(name) || ERROR_PAGE_MESSAGES[name];
}

export function resolveErrorPageContent({
  code = "LAUNCH_FAILED",
  message = "",
  getMessage = defaultGetMessage,
} = {}) {
  const resolvedCode = Object.prototype.hasOwnProperty.call(ERROR_CODES, code)
    ? code
    : "LAUNCH_FAILED";
  const content = ERROR_CODES[resolvedCode];

  return {
    documentTitle: resolveMessage("error_page_document_title", getMessage),
    eyebrow: resolveMessage("error_page_eyebrow", getMessage),
    installLabel: resolveMessage("error_page_install_cta", getMessage),
    openExtensionsLabel: resolveMessage("error_page_open_extensions", getMessage),
    title: resolveMessage(content.title, getMessage),
    body: resolveMessage(content.body, getMessage),
    detail: message || resolveMessage(content.detail, getMessage),
    code: resolvedCode,
  };
}

export const DEFAULT_INSTALL_URL = "https://github.com/OpenSelena/libreget/releases/latest";

export function sanitizeInstallUrl(candidate) {
  if (!candidate || typeof candidate !== "string") {
    return DEFAULT_INSTALL_URL;
  }
  try {
    const parsed = new URL(candidate.trim());
    if (parsed.protocol === "https:" && (parsed.hostname === "github.com" || parsed.hostname === "libreget.org")) {
      return parsed.href;
    }
  } catch {
    // Malformed URL.
  }
  return DEFAULT_INSTALL_URL;
}
