const MEDIA_CONTENT_TYPES = [
  "video/mp4",
  "video/webm",
  "video/x-flv",
  "video/ogg",
  "audio/mpeg",
  "audio/ogg",
  "audio/mp4",
  "application/vnd.apple.mpegurl",
  "application/x-mpegurl",
  "application/dash+xml",
];

const MEDIA_EXTENSIONS = [
  ".mp4", ".webm", ".m3u8", ".mpd",
  ".flv", ".ogg", ".mp3", ".m4a", ".m4v",
  ".mkv", ".avi", ".mov",
];

const BLOCKED_HOSTS = [
  "google-analytics.com",
  "googletagmanager.com",
  "facebook.com",
  "doubleclick.net",
  "analytics",
];

const MIN_CONTENT_LENGTH = 100 * 1024;

const detectedMedia = new Map();
const pendingRequests = new Map();

export function getDetectedMedia(tabId) {
  return detectedMedia.get(tabId) || new Map();
}

export function clearTabMedia(tabId) {
  detectedMedia.delete(tabId);
}

export function getMediaCount(tabId) {
  const media = detectedMedia.get(tabId);
  return media ? media.size : 0;
}

function isBlockedHost(url) {
  try {
    const host = new URL(url).hostname;
    return BLOCKED_HOSTS.some(b => host.includes(b));
  } catch { return false; }
}

function isMediaByExtension(url) {
  try {
    const path = new URL(url).pathname.toLowerCase();
    return MEDIA_EXTENSIONS.some(ext => path.includes(ext));
  } catch { return false; }
}

function isMediaByContentType(contentType) {
  if (!contentType) return false;
  const lower = contentType.toLowerCase();
  return MEDIA_CONTENT_TYPES.some(mt => lower.includes(mt));
}

function getContentLength(headers) {
  const header = headers?.find(h => h.name.toLowerCase() === "content-length");
  return header ? parseInt(header.value, 10) : 0;
}

function getContentType(headers) {
  const header = headers?.find(h => h.name.toLowerCase() === "content-type");
  return header?.value || "";
}

function getMediaType(contentType, url) {
  const ct = contentType.toLowerCase();
  if (ct.includes("mpegurl") || url.includes(".m3u8")) return "hls";
  if (ct.includes("dash") || url.includes(".mpd")) return "dash";
  if (ct.includes("video/")) return "video";
  if (ct.includes("audio/")) return "audio";
  return "media";
}

function formatSize(bytes) {
  if (!bytes || bytes <= 0) return "";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function isHlsSegment(url, contentType) {
  const ct = contentType.toLowerCase();
  if (ct.includes("mp2t") || ct.includes("mpeg2-ts") || ct.includes("mpeg-ts")) {
    return true;
  }
  try {
    const path = new URL(url).pathname.toLowerCase();
    if (path.endsWith(".ts") || path.match(/\.ts\?/)) {
      return true;
    }
    if (/\/video\d+\.ts/.test(path)) {
      return true;
    }
  } catch {}
  return false;
}

export function registerSnifferListeners(onMediaDetected) {
  chrome.webRequest.onSendHeaders.addListener(
    (details) => {
      if (details.tabId < 0) return;
      if (details.method !== "GET") return;

      pendingRequests.set(details.requestId, {
        requestHeaders: details.requestHeaders || [],
        tabId: details.tabId,
      });
    },
    { urls: ["http://*/*", "https://*/*"] },
    ["requestHeaders", "extraHeaders"]
  );

  chrome.webRequest.onHeadersReceived.addListener(
    (details) => {
      if (details.tabId < 0) return;
      if (details.statusCode < 200 || details.statusCode >= 300) return;

      const url = details.url;
      if (isBlockedHost(url)) return;

      const contentType = getContentType(details.responseHeaders);
      const contentLength = getContentLength(details.responseHeaders);
      const isMedia = isMediaByContentType(contentType) || isMediaByExtension(url);

      if (!isMedia) {
        pendingRequests.delete(details.requestId);
        return;
      }

      if (isHlsSegment(url, contentType)) {
        pendingRequests.delete(details.requestId);
        return;
      }

      if (contentLength > 0 && contentLength < MIN_CONTENT_LENGTH) {
        if (!url.includes(".m3u8") && !contentType.includes("mpegurl")) {
          pendingRequests.delete(details.requestId);
          return;
        }
      }

      const reqData = pendingRequests.get(details.requestId);
      pendingRequests.delete(details.requestId);

      const mediaType = getMediaType(contentType, url);

      const entry = {
        url,
        contentType,
        contentLength,
        mediaType,
        sizeText: formatSize(contentLength),
        detectedAt: Date.now(),
        tabId: details.tabId,
        requestHeaders: reqData?.requestHeaders || [],
        responseHeaders: details.responseHeaders || [],
      };

      if (!detectedMedia.has(details.tabId)) {
        detectedMedia.set(details.tabId, new Map());
      }
      detectedMedia.get(details.tabId).set(url, entry);

      onMediaDetected(details.tabId, entry);
    },
    { urls: ["http://*/*", "https://*/*"] },
    ["responseHeaders"]
  );

  chrome.webRequest.onErrorOccurred.addListener(
    (details) => { pendingRequests.delete(details.requestId); },
    { urls: ["http://*/*", "https://*/*"] }
  );

  chrome.tabs.onRemoved.addListener((tabId) => {
    detectedMedia.delete(tabId);
  });

  chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
    if (changeInfo.url) {
      detectedMedia.delete(tabId);
    }
  });
}
