/** Actions that put the URL in the omnibox instead of queuing it. */
export function isPrefillAction(action: string): boolean {
  return action === "prefill" || action === "confirm";
}

/**
 * Whether a received URL may start downloading without a click. Links opened
 * through `omniget://` ("confirm") always wait for the user, even when
 * "download on paste" is on: any web page can open such a link.
 */
export function mayAutoStart(action: string, autoDownloadOnPaste: boolean): boolean {
  return action === "prefill" && autoDownloadOnPaste;
}

/**
 * Whether putting a URL in the omnibox arms "download on paste". A URL the
 * user typed or pasted follows the setting; a received URL follows
 * `mayAutoStart`, so an `omniget://` link still waits for a click.
 */
export function armsAutoDownload(source: string, autoDownloadOnPaste: boolean): boolean {
  return source === "typed" ? autoDownloadOnPaste : mayAutoStart(source, autoDownloadOnPaste);
}
