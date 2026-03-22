# OmniGet Chrome Extension

MV3 extension for sending supported media pages to OmniGet with one click.

## Supported platforms

| Platform | Content types |
|----------|--------------|
| YouTube | video, short, playlist, profile |
| Instagram | post, reel, story (image), profile |
| TikTok | video (direct, short link, embed), profile |
| Twitter / X | post, profile |
| Reddit | post, video, profile |
| Twitch | video, clip, live stream |
| Hotmart | course |
| Pinterest | pin (image) |
| Bluesky | post, profile |
| Telegram | post, profile |
| Vimeo | video |
| Udemy | course |
| Bilibili | video |

Mirror domains are also recognized: `youtu.be`, `youtube-nocookie.com`, `ddinstagram.com`, `vxtwitter.com`, `fixvx.com`, `v.redd.it`, `redd.it`, `clips.twitch.tv`, `pin.it`, `b23.tv`, `t.me`, `telegram.me`.

## How it works

1. You navigate to a supported media page.
2. The toolbar icon switches from gray (inactive) to colored (active).
3. You click the icon.
4. The extension sends the URL (and cookies for authenticated platforms) to OmniGet via Chrome Native Messaging.
5. OmniGet opens and starts the download or fills the omnibox.

If OmniGet is not installed or cannot be launched, the extension shows a localized error page.

## Load it locally

1. Install OmniGet and launch it once so it registers the Chrome native host.
2. Open `chrome://extensions`.
3. Enable **Developer mode**.
4. Click **Load unpacked**.
5. Select this folder: `browser-extension/chrome`

The unpacked extension keeps a stable ID through the committed manifest key:

`dkjelkhaaakffpghdfalobccaaipajip`

On macOS and Linux, that first launch writes Chrome's native messaging manifest into your user profile automatically. On Windows, a registry key is created under `HKCU\Software\Google\Chrome\NativeMessagingHosts`.

## Chrome Web Store packaging

The committed `manifest.json` keeps its `key` so unpacked installs have a stable development ID. The packaging script (`scripts/package.mjs`) strips it automatically before creating the ZIP:

```bash
node browser-extension/chrome/scripts/package.mjs --version 0.1.0 --output omniget-chrome-extension.zip
```

Once the CWS assigns a store ID, add it to `CHROME_EXTENSION_IDS` in [`src-tauri/src/native_host.rs`](../../src-tauri/src/native_host.rs).

## Internationalization

8 languages: English, French, Portuguese, Greek, Italian, Japanese, Simplified Chinese, Traditional Chinese.

Locale files are in `_locales/{lang}/messages.json`. Error pages and tooltip titles fall back to English when a translation is missing.

## Tests

```bash
node --test browser-extension/chrome/tests/*.test.mjs
```

94 tests across 7 files covering platform detection, click handling, badge feedback, tooltip titles, error content, cookie extraction, and manifest validation.

## Architecture

```
src/
  detect.js          URL-based platform detection (no content scripts)
  background.js      Service worker: icon switching, native messaging
  action-click.js    Click handler with DI for testability
  action-feedback.js Badge controller (green checkmark for 1.5s)
  action-title.js    Tooltip resolution with i18n fallback
  cookies.js         Cookie extraction for authenticated platforms
pages/
  error.html/css/js  Standalone error page (HOST_MISSING, INVALID_URL, LAUNCH_FAILED)
  error-content.js   Error message resolution with i18n fallback
scripts/
  package.mjs        ZIP packaging for CWS (native Node.js implementation)
tests/
  *.test.mjs         94 tests using node:test
```
