<p align="center">
  <img src="static/loop.png" alt="Loop, the OmniGet mascot" width="120" />
</p>

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/github/v/release/tonhowtf/omniget?style=for-the-badge&label=release" alt="Latest Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green?style=for-the-badge" alt="License GPL-3.0" /></a>
  <a href="https://github.com/tonhowtf/omniget/stargazers"><img src="https://img.shields.io/github/stars/tonhowtf/omniget?style=for-the-badge" alt="Stars" /></a>
</p>

<h1 align="center">OmniGet</h1>

<h3 align="center">Paste a link. Get your file.</h3>

OmniGet downloads videos, courses, and media from the internet. YouTube, Instagram, TikTok — paste a link and it handles the rest, including [1000+ sites via yt-dlp](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md). Free, open source, built with Rust.

<p align="center">
  <img src="assets/screenshot.png" alt="OmniGet downloading a YouTube video" width="800" />
  <br>
  <sub>Paste a link. Pick a quality. Download.</sub>
</p>

## Download

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-Windows-blue.svg?style=for-the-badge&logo=windows" alt="Download for Windows" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-macOS-black.svg?style=for-the-badge&logo=apple" alt="Download for macOS" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-Linux-orange.svg?style=for-the-badge&logo=linux&logoColor=white" alt="Download for Linux" /></a>
</p>

Also available as a Flatpak on Linux and a portable `.exe` on Windows.

## What can it download?

**Videos** from YouTube, Instagram, TikTok, Twitter/X, Reddit, Twitch, Pinterest, Vimeo, Bluesky, and Bilibili.

**Courses** from Hotmart, Udemy, Kiwify, Teachable, and [6 more platforms](#course-platforms). Log in once, download all lessons, attachments, and descriptions.

**Torrents** — drag a `.torrent` file or paste a magnet link. Built-in client, no extra software needed.

**Files between devices** — send a file to another computer with a 4-word share code. Works across different networks.

**Anything else** — if [yt-dlp supports it](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md), OmniGet downloads it. That's over 1000 additional sites.

No setup required beyond the app itself. OmniGet downloads yt-dlp and FFmpeg when it needs them, keeps itself up to date, and runs in dark or light mode across 8 languages.

### Media Platforms

| Platform | Content |
|----------|---------|
| YouTube | Videos, Shorts, Playlists, Search |
| Instagram | Posts, Reels, Stories |
| TikTok | Videos, Photos |
| Twitter / X | Videos, GIFs |
| Reddit | Videos, Images |
| Twitch | Clips |
| Pinterest | Images, Videos |
| Vimeo | Videos |
| Bluesky | Images, Videos |
| Bilibili (哔哩哔哩) | Videos, Series |
| Telegram | Photos, Videos, Files (via plugin) |
| Torrent / Magnet | Any `.torrent` file or magnet link |

<details>
<summary><strong>Chinese platforms</strong> (supported via yt-dlp)</summary>

| Platform | Content |
|----------|---------|
| Douyin (抖音) | Videos |
| Xiaohongshu (小红书) | Videos, Images |
| Kuaishou (快手) | Videos |
| Youku (优酷) | Videos |
| Tencent Video (腾讯视频) | Videos |
| iQiyi (爱奇艺) | Videos |
| Mango TV (芒果TV) | Videos |

These platforms may require a Chinese IP address.

</details>

<details>
<summary><strong>Course platforms</strong></summary>

| Platform | Auth | Region |
|----------|------|--------|
| Hotmart | Email + Password | BR / Global |
| Udemy | Email + Cookies | Global |
| Kiwify | Email + Password / Token | BR |
| Gumroad | Email + Password / Token | Global |
| Teachable | Token | Global |
| Kajabi | Token | Global |
| Skool | Email + Password / Token | Global |
| Wondrium / Great Courses | Email + Password / Token | US |
| Thinkific | Browser Cookies | Global |
| Rocketseat | Token | BR |

</details>

## How it works

1. **Paste a link** into the omnibox — or drag a file, or search YouTube right there
2. OmniGet figures out the platform and shows you a preview with quality options
3. Hit download — progress, speed, and ETA update as it goes

For courses: log in to the platform, browse your library, pick what you want, and download it all at once.

## Copy. Press. Done.

Copy a video link from anywhere — Discord, Twitter, a group chat. Press **Ctrl+Shift+D** (or **Cmd+Shift+D** on macOS). That's it.

OmniGet grabs the URL from your clipboard and downloads it in the background. You don't even need to open the app. Change the hotkey in **Settings → Download → Hotkey**.

## Browser Extension

Install the [Chrome extension](browser-extension/chrome/README.md) and stop copying links. When you're on a page with a video, click the OmniGet icon — it sends the URL, cookies, and everything the app needs to start downloading.

The extension also detects video streams on any website, even ones OmniGet doesn't officially support. If your browser can play it, OmniGet can probably download it.

## Plugins

OmniGet ships lean. Extra features live in plugins you can install from the built-in marketplace:

- **Courses** — download full courses from 10 education platforms
- **Telegram** — browse chats and download media
- **Convert** — convert between video and audio formats

Want to build one? Check out the [Plugin SDK](src-tauri/omniget-plugin-sdk/).

## Building from Source

**Prerequisites:** [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) 18+, [pnpm](https://pnpm.io/)

```bash
git clone https://github.com/tonhowtf/omniget.git
cd omniget
pnpm install
pnpm tauri dev
```

<details>
<summary>Linux dependencies</summary>

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

</details>

Production build: `pnpm tauri build`

<details>
<summary><strong>Windows SmartScreen / macOS Gatekeeper</strong></summary>

**Windows:** SmartScreen may warn you on first run — click **More info** → **Run anyway**. This is normal for open-source apps without a paid code signing certificate.

**macOS:** If Gatekeeper blocks the app, run in Terminal:

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

## Contributing

Found a bug or want a feature? [Open an issue](https://github.com/tonhowtf/omniget/issues). Pull requests are welcome.

## Notice to Platform Owners

If you represent a listed platform and have concerns, email **tonhowtf@gmail.com** from an official domain — the platform will be removed promptly.

## Legal

Use it for personal stuff. Respect copyright and platform rules.

## License

[GPL-3.0](LICENSE). The OmniGet name, logo, and Loop mascot are project trademarks not covered by the code license.
