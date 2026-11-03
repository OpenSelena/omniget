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
