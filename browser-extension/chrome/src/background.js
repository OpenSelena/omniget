import { detectSupportedMediaUrl } from "./detect.js";
import { resolveActionTitle } from "./action-title.js";

const HOST_NAME = "wtf.tonho.omniget";
const INSTALL_URL = "https://github.com/tonhowtf/omniget/releases/latest";

function getIconPath(iconSet) {
  return {
    16: chrome.runtime.getURL(iconSet[0]),
    24: chrome.runtime.getURL(iconSet[1]),
    32: chrome.runtime.getURL(iconSet[2]),
  };
}

const ACTIVE_ICON_PATHS = ["icons/active-16.png", "icons/active-24.png", "icons/active-32.png"];
const INACTIVE_ICON_PATHS = ["icons/inactive-16.png", "icons/inactive-24.png", "icons/inactive-32.png"];

chrome.runtime.onInstalled.addListener(() => {
  refreshActiveTab().catch(() => {});
});

chrome.runtime.onStartup.addListener(() => {
  refreshActiveTab().catch(() => {});
});

chrome.tabs.onActivated.addListener(() => {
  refreshActiveTab().catch(() => {});
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (!changeInfo.url && !changeInfo.status) {
    return;
  }
  if (!tab?.url) {
    return;
  }
  refreshTabAction(tabId, tab).catch((error) => {
    console.error("[OmniGet] Failed to refresh tab action:", error);
  });
});

chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId !== chrome.windows.WINDOW_ID_NONE) {
    refreshActiveTab().catch(() => {});
  }
});

chrome.action.onClicked.addListener(async (tab) => {
  const detected = detectSupportedMediaUrl(tab?.url);
  if (!detected?.supported) {
    return;
  }

  try {
    const response = await sendNativeMessage({
      type: "enqueue",
      url: detected.url,
    });

    if (!response?.ok) {
      openErrorPage({
        code: response?.code ?? "LAUNCH_FAILED",
        message: response?.message ?? "OmniGet could not be launched from Chrome.",
        url: detected.url,
      });
    }
  } catch (error) {
    openErrorPage({
      code: mapChromeErrorCode(error),
      message: error instanceof Error ? error.message : String(error),
      url: detected.url,
    });
  }
});

async function refreshActiveTab() {
  const [tab] = await chrome.tabs.query({
    active: true,
    lastFocusedWindow: true,
  });

  if (tab?.id !== undefined) {
    await refreshTabAction(tab.id, tab);
  }
}

async function refreshTabAction(tabId, tab) {
  if (!tab?.url) {
    return;
  }

  const detected = detectSupportedMediaUrl(tab.url);
  const supported = Boolean(detected?.supported);

  try {
    await chrome.action.setIcon({
      tabId,
      path: supported ? getIconPath(ACTIVE_ICON_PATHS) : getIconPath(INACTIVE_ICON_PATHS),
    });
  } catch (error) {
    console.error("[OmniGet] Failed to set icon:", error);
  }

  try {
    await chrome.action.setTitle({
      tabId,
      title: resolveActionTitle(supported),
    });
  } catch (error) {
    console.error("[OmniGet] Failed to set title:", error);
  }

  try {
    if (supported) {
      await chrome.action.enable(tabId);
    } else {
      await chrome.action.disable(tabId);
    }
  } catch (error) {
    console.error("[OmniGet] Failed to set enabled state:", error);
  }
}

function sendNativeMessage(message) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(HOST_NAME, message, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      resolve(response);
    });
  });
}

function mapChromeErrorCode(error) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("Specified native messaging host not found")) {
    return "HOST_MISSING";
  }
  if (message.includes("Access to the specified native messaging host is forbidden")) {
    return "HOST_MISSING";
  }
  return "LAUNCH_FAILED";
}

function openErrorPage({ code, message, url }) {
  const params = new URLSearchParams({
    code,
    installUrl: INSTALL_URL,
    message,
    url,
  });

  chrome.tabs.create({
    url: `${chrome.runtime.getURL("pages/error.html")}?${params.toString()}`,
  });
}
