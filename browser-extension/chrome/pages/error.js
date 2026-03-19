const params = new URLSearchParams(window.location.search);

const code = params.get("code") ?? "LAUNCH_FAILED";
const message = params.get("message") ?? "";
const url = params.get("url") ?? "";
const installUrl = params.get("installUrl") ?? "https://github.com/tonhowtf/omniget/releases/latest";

const title = document.getElementById("title");
const body = document.getElementById("body");
const detail = document.getElementById("detail");
const urlNode = document.getElementById("url");
const installLink = document.getElementById("install-link");

const openExtensionsBtn = document.getElementById("open-extensions");

const content = getContent(code);

title.textContent = content.title;
body.textContent = content.body;
detail.textContent = message || content.detail;
urlNode.textContent = url;
installLink.href = installUrl;

openExtensionsBtn.addEventListener("click", () => {
  chrome.tabs.create({ url: "chrome://extensions" });
});

function getContent(errorCode) {
  switch (errorCode) {
    case "HOST_MISSING":
      return {
        title: "Open OmniGet once to finish Chrome setup",
        body: "Chrome could not find the OmniGet bridge on this computer yet.",
        detail: "Install OmniGet if needed, then launch the desktop app once and click the extension again.",
      };
    case "INVALID_URL":
      return {
        title: "This page URL cannot be sent to OmniGet",
        body: "The current page is not a supported media page for the OmniGet extension.",
        detail: "Try again from a direct video, reel, post, playlist, or course page.",
      };
    default:
      return {
        title: "OmniGet could not be launched from Chrome",
        body: "The extension talked to the native host, but the desktop app did not start correctly.",
        detail: "Check that OmniGet is installed and not blocked by Windows, then try again.",
      };
  }
}
