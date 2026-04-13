<p align="center">
  <img src="static/loop.png" alt="Loop, the OmniGet mascot" width="120" />
</p>

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/tonhowtf/omniget/ci.yml?branch=main&style=for-the-badge&label=CI" alt="CI" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/github/v/release/tonhowtf/omniget?style=for-the-badge&label=release" alt="Latest Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green?style=for-the-badge" alt="License GPL-3.0" /></a>
  <a href="https://github.com/tonhowtf/omniget/stargazers"><img src="https://img.shields.io/github/stars/tonhowtf/omniget?style=for-the-badge" alt="Stars" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases"><img src="https://img.shields.io/github/downloads/tonhowtf/omniget/total?style=for-the-badge&label=downloads" alt="Downloads" /></a>
</p>

<h1 align="center">OmniGet</h1>

<h3 align="center">Paste a link. Get your file.</h3>

OmniGet downloads videos, courses, and files from the internet. Paste a link from YouTube, Instagram, TikTok, or any of [1000+ supported sites](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md). It figures out what you want and downloads it. Free and open source.

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

**Torrents.** Drag a `.torrent` file or paste a magnet link. Built-in client, no extra software needed.

**Files between devices.** Send a file to another computer using a 4-word share code. Works across different networks.

**Anything else.** If a site is [supported by yt-dlp](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md), OmniGet can download from it. That covers over 1000 sites.

No setup required beyond the app itself. OmniGet handles [yt-dlp](https://github.com/yt-dlp/yt-dlp) (the engine that supports 1000+ sites) and FFmpeg (used to merge video and audio) automatically, stays up to date on its own, and comes with 11 color themes and 8 languages.

The app also includes Loop, a mascot that reacts to your downloads in real time. The themes include Catppuccin, Dracula, and NyxVamp variants.

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
| Udemy | Email + Browser Login | Global |
| Kiwify | Email + Password / Access Token | BR |
| Gumroad | Email + Password / Access Token | Global |
| Teachable | Access Token | Global |
| Kajabi | Access Token | Global |
| Skool | Email + Password / Access Token | Global |
| Wondrium / Great Courses | Email + Password / Access Token | US |
| Thinkific | Browser Login | Global |
| Rocketseat | Access Token | BR |

</details>

## How it works

1. **Paste a link** into the omnibox. Or drag a file, or search YouTube right there.
2. OmniGet figures out the platform and shows you a preview with quality options.
3. Hit download. Progress, speed, and ETA update as it goes.

For courses: log in to the platform, browse your library, pick what you want, and download it all at once.

## Copy. Press. Done.

Copy a video link from anywhere. Discord, Twitter, a group chat. Press **Ctrl+Shift+D** (or **Cmd+Shift+D** on macOS). That's it.

OmniGet grabs the URL from your clipboard and downloads it in the background. You don't even need to open the app. Change the hotkey in **Settings > Downloads > Hotkey**.

## Browser Extension

Install the [Chrome extension](browser-extension/chrome/README.md) to skip the copy-paste step. When you're on a page with a video, click the OmniGet icon. It sends the link and login info the app needs to start downloading.

The extension also detects video streams on any website, even ones OmniGet doesn't officially support. If your browser can play it, OmniGet can probably download it.

## Plugins

OmniGet starts simple. Extra features are available as plugins you can install from the built-in marketplace:

- **Courses.** Download full courses from 10 education platforms.
- **Telegram.** Browse chats and download media.
- **Convert.** Convert between video and audio formats.

Want to build one? Check out the [Plugin SDK](src-tauri/omniget-plugin-sdk/).

## Building from Source (for developers)

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

**Windows:** SmartScreen may warn you on first run. Click **More info**, then **Run anyway**. This is normal for open-source apps without a paid code signing certificate.

**macOS:** If Gatekeeper blocks the app, run in Terminal:

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

## Contributing

Found a bug or want a feature? [Open an issue](https://github.com/tonhowtf/omniget/issues). Pull requests are welcome.

## Notice to Platform Owners

If you represent a listed platform and have concerns, reach out at **tonhowtf@gmail.com** from a company email. We'll remove the platform right away.

## Legal

OmniGet is meant for personal use. Please respect copyright and each platform's terms of service. You're responsible for what you download.

## License

[GPL-3.0](LICENSE). The OmniGet name, logo, and Loop mascot are project trademarks not covered by the code license.
