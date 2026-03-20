const DEFAULT_LAUNCH_FAILED_CODE = "LAUNCH_FAILED";

export async function handleSupportedActionClick({
  tabId,
  url,
  sendNativeMessage,
  clearBadge = async () => {},
  showSuccessBadge = async () => {},
  openErrorPage,
  mapChromeErrorCode,
}) {
  if (tabId !== undefined && tabId !== null) {
    await clearBadge(tabId);
  }

  try {
    const response = await sendNativeMessage({
      type: "enqueue",
      url,
    });

    if (!response?.ok) {
      await openErrorPage({
        code: response?.code ?? DEFAULT_LAUNCH_FAILED_CODE,
        message: response?.message ?? "",
        url,
      });
      return false;
    }

    if (tabId !== undefined && tabId !== null) {
      await showSuccessBadge(tabId);
    }

    return true;
  } catch (error) {
    await openErrorPage({
      code: mapChromeErrorCode(error),
      message: error instanceof Error ? error.message : String(error),
      url,
    });
    return false;
  }
}
