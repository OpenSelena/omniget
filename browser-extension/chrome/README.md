# OmniGet Chrome Extension

Windows-first MV3 extension for sending supported media pages to OmniGet.

## Load it locally

1. Install OmniGet and launch it once on Windows so it can register the Chrome native host.
2. Open `chrome://extensions`.
3. Enable `Developer mode`.
4. Click `Load unpacked`.
5. Select this folder: [`browser-extension/chrome`](/E:/Workspace/omniget/browser-extension/chrome)

The unpacked extension keeps a stable ID through the committed manifest key:

`dkjelkhaaakffpghdfalobccaaipajip`

## What it does

- Colors the toolbar icon on supported media pages only.
- Keeps the icon gray and disabled on unsupported pages.
- Sends the current page URL to OmniGet through Chrome Native Messaging.
- Opens a local error page when OmniGet is missing or could not be launched.

## Quick test

```powershell
node --test browser-extension/chrome/tests/detect.test.mjs
```
