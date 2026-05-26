<p align="center">
  <img src="static/loop.png" alt="Loop, the OmniGet mascot" width="120" />
</p>

<h1 align="center">OmniGet</h1>

<h3 align="center">Download Udemy courses, YouTube, and 1,800+ sites in one app. No terminal.</h3>

<p align="center">
<b>English</b>
| <a href="README_zh_CN.md">中文</a>
| <a href="README.ru.md">Русский</a>
</p>

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/github/v/release/tonhowtf/omniget?style=for-the-badge&label=release" alt="Latest Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green?style=for-the-badge" alt="License GPL-3.0" /></a>
  <a href="https://github.com/tonhowtf/omniget/stargazers"><img src="https://img.shields.io/github/stars/tonhowtf/omniget?style=for-the-badge" alt="GitHub stars" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases"><img src="https://img.shields.io/github/downloads/tonhowtf/omniget/total?style=for-the-badge&label=downloads" alt="Total downloads" /></a>
  <a href="https://hosted.weblate.org/engage/omniget/"><img src="https://hosted.weblate.org/widget/omniget/frontend-json/svg-badge.svg" alt="Translation status" /></a>
</p>

<p align="center">
  <b>OmniGet</b> is a free, open source desktop app for Windows, macOS, and Linux that downloads online courses (Udemy, Hotmart, Kiwify, Skool, Teachable, and more), videos and audio from YouTube, TikTok, Instagram, Twitter/X, Reddit, and over 1,800 other sites, plus music and books. Everything plays inside the app. No command line, no setup, your files stay on your computer.
</p>

<p align="center">
  <a href="#get-omniget"><b>Download for Windows, macOS, or Linux</b></a>
</p>

<p align="center">
  <img src="assets/screenshot.png" alt="OmniGet desktop app home screen, a free downloader for courses, videos, music and books" width="820" />
</p>

---

## The problem this solves

You already have yt-dlp open in a terminal. You found a Udemy downloader script that keeps breaking on every site update. You have a separate thing for music, and none of them talk to each other. Every download is three tools and a copy paste.

OmniGet does all three in one window. Paste a course link, a YouTube link, a TikTok, a magnet, a podcast, and it figures out the rest. No terminal, no Python, no setup. The file lands in your folder, and it plays right there in the app.

It is the only open source app that downloads a full Udemy or Hotmart course, video and audio from 1,800+ sites, and your music library, in one place, without the command line. Thousands of GitHub stars in its first months, and it grew because that combination did not exist anywhere else.

---

## What OmniGet downloads

You paste a link. OmniGet detects the site, shows a preview with quality options, and downloads. If [yt-dlp](https://github.com/yt-dlp/yt-dlp) supports a site, OmniGet downloads from it, which is roughly a thousand more than the table below.

| Category | Platforms |
|----------|-----------|
| Online courses | Hotmart, Udemy, Kiwify, Gumroad, Teachable, Kajabi, Skool, Wondrium, Thinkific, Rocketseat |
| Video and audio | YouTube, Instagram, TikTok, Twitter/X, Reddit, Twitch, Pinterest, Vimeo, Bluesky, **Bilibili** ✨ |
| Bilibili (deep) | Sign in for 4K / HDR / Dolby Vision / Hi-Res lossless / Dolby Atmos · danmaku (XML/ASS/JSON) · NFO for Kodi/Jellyfin · 11 URL types (UGC / 番剧 / 课程 / 收藏夹 / UP主 / 每周必看 / 稍后再看 / 历史记录 / b23.tv) |
| Asian platforms | Douyin (抖音), Xiaohongshu (小红书), Kuaishou (快手), Youku (优酷), iQiyi (爱奇艺), Tencent Video, Mango TV |
| Image galleries | DeviantArt, Pixiv, ArtStation, Flickr, Tumblr, Imgur albums, Kemono, Newgrounds, image boards |
| Files and transfer | `.torrent` and magnet links, plus direct P2P transfer between two computers with a short code |

Things people search for and OmniGet does:

- **Download a full online course**, every lesson and attached PDF, then watch it inside the app and resume where you stopped.
- **Download a YouTube video or whole playlist**, pick the quality, or grab audio only as MP3, M4A, Opus, FLAC, or WAV.
- **Download TikTok, Instagram, Twitter/X, Reddit** posts, reels, stories, carousels, and galleries.
- **Batch download** a list of links from a text file, or an entire creator profile.
- **Download only part of a video** by setting a start and end time.
- **Download subtitles** in any language, embed them, or generate them with Whisper when none exist.
- **Skip sponsors** with SponsorBlock, and auto embed metadata and thumbnails.
- **Follow a channel** and auto download new uploads, with a tray notification.
- **Download Bilibili at maximum quality**: sign in once and unlock 4K, HDR, Hi-Res lossless audio and Dolby Atmos. Anime and courses are organized like Plex expects (Season / Episode folders with `tvshow.nfo`).

Downloads are reliable, not a guessing game. Speed and ETA come straight from the downloader instead of being faked from a percentage, so they stay correct even when the file size is unknown or the stream is live. A stall is shown as a stall, not a frozen "3 seconds left". The queue resumes interrupted downloads, retries with backoff, and does not argue with you.

---

## Get OmniGet

<table>
  <tr>
    <th>Platform</th>
    <th>Download</th>
  </tr>
  <tr>
    <td><strong>Windows</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="Download OmniGet for Windows" src="https://img.shields.io/badge/Windows-Portable_EXE-0078D6?style=for-the-badge&logo=windows&logoColor=white" height="40"></a>
      <br/>
      <sub>Download the <code>.exe</code> and double-click. No installer, no admin needed.</sub>
    </td>
  </tr>
  <tr>
    <td><strong>macOS</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="Download OmniGet for macOS" src="https://img.shields.io/badge/macOS-DMG-000000?style=for-the-badge&logo=apple&logoColor=white" height="40"></a>
      <br/>
      <sub>Open the <code>.dmg</code> and drag OmniGet to Applications.</sub>
    </td>
  </tr>
  <tr>
    <td><strong>Linux</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="Download OmniGet for Linux" src="https://img.shields.io/badge/Linux-Flatpak-FFAA33?style=for-the-badge&logo=linux&logoColor=white" height="40"></a>
      <br/>
      <sub><code>flatpak install wtf.tonho.omniget</code>, or grab the bundle from Releases.</sub>
    </td>
  </tr>
</table>

Free and open source under GPL-3.0. Updates run quietly in the background. Bundled tools (yt-dlp, FFmpeg) install themselves and yt-dlp is verified by SHA256 before it runs. Your files never leave your computer.

---

## It also plays everything inside

This is the part people do not expect. OmniGet is not just where you download. It is where you watch, read, and listen.

### Open a course and actually watch it

Download the whole course (Hotmart, Udemy, Kiwify, Skool, Teachable, Kajabi, Wondrium, Thinkific) and watch it without leaving the app. Resume at the second you stopped. Take notes that jump to that moment when you click them. Read the attached PDFs side by side.

<p align="center">
  <img src="assets/screenshot-courses.png" alt="OmniGet course player with timestamped notes and PDF attachments" width="720" />
  <br/>
  <em>Course player, notes pinned to timestamps, attachments in the same window.</em>
</p>

### Read books, real ones

Drop a folder of PDFs and EPUBs. OmniGet pulls covers from them, fetches titles and authors, and opens each one in a built-in reader with highlights, bookmarks, a focus mode and a paper-feel theme for the eyes. CBZ comics and TXT/HTML too.

<p align="center">
  <img src="assets/screenshot-reader.png" alt="OmniGet built-in EPUB and PDF reader with highlights and focus mode" width="720" />
  <br/>
  <em>Reader with highlights, notes panel and focus mode.</em>
</p>

### Music, the way you remember it

Point OmniGet to your music folder and it shows your tracks the way iTunes used to: albums with covers, artists with discographies, a queue that doesn't argue with you.

- Plays MP3, FLAC, M4A, OGG, Opus, anything you already have.
- Pulls **synced lyrics** so they scroll along with the song.
- Connects to **Spotify, SoundCloud, YouTube Music, Qobuz and Last.fm**, so your playlists and likes show up next to your local files.
- **Equalizer** with presets, dark theme variants per album cover, an activity dashboard with your top tracks, and a Discord presence that shows what you're playing.

<p align="center">
  <img src="assets/screenshot-music.png" alt="OmniGet music player with album view, synced lyrics and streaming sources" width="820" />
  <br/>
  <em>Local library, synced lyrics, streaming sources, one player.</em>
</p>

---

## The small things that add up

Quietly there when you need them.

- **Subtitle Workshop** that opens SRT, VTT and ASS, with timing tools, two-point sync, find and replace, a one-click auto fix, AI translate and AI grammar fix, and a waveform with shot-change markers.
- **Pomodoro focus timer** that pauses your video when the session ends.
- **Notes app** with bidirectional links, daily journal and a knowledge graph.
- **Progress dashboard** with a streak counter, daily goals and a year-style heatmap.
- **FFmpeg converter** for local files, no internet required.
- **Telegram chat browser** that lets you save photos, videos and files from any chat.
- **Browser extension** (Chrome and Firefox) that hands the current page to OmniGet with one click.
- **Global hotkey** (`Ctrl+Shift+D`) that downloads whatever URL is in your clipboard.
- **9 languages**, **14 themes** including Catppuccin, Dracula, One Dark Pro and three e-ink variants.

---

## How it feels day-to-day

<p align="center">
  <img src="assets/screenshot-flow.png" alt="Typical OmniGet workflow, paste a link and it downloads in the background" width="720" />
</p>

Copy a link anywhere, a tweet, a Discord message, an open tab. Press `Ctrl+Shift+D`. OmniGet downloads in the background. You don't even open the window.

Or paste in the omnibox, glance at the preview, click download.

For a course: log in once on the platform, browse your library, pick what you want, walk away. Every lesson and attachment lands in the folder you chose.

For books: drop the files in a folder you already use, scan once, and they appear with covers.

For music: point at a folder, and the library is yours.

---

## Frequently asked questions

**Is OmniGet free?**
Yes. Free and open source under GPL-3.0, with no account, no ads, and no paid tier.

**Do I need the terminal or Python?**
No. OmniGet is a normal desktop app. Download it, double-click, paste a link. yt-dlp and FFmpeg are bundled and update themselves.

**Is this a yt-dlp GUI?**
It uses yt-dlp under the hood for the 1,800+ generic sites, with native extractors for the big platforms and a real interface, a queue, a library, and built-in players on top. So yes, and a lot more than a GUI.

**Can it download a full Udemy or Hotmart course?**
Yes. You log in once on the platform, pick the course, and OmniGet downloads every lesson and attachment, then plays them back with timestamped notes.

**Which sites are supported?**
Online courses, YouTube, TikTok, Instagram, Twitter/X, Reddit, Twitch, Vimeo, Bilibili, Pinterest, Bluesky, major Asian platforms, image galleries, torrents and magnets, plus around 1,800 more through yt-dlp.

**Does it work on Windows, macOS, and Linux?**
Yes, all three. Windows is a portable `.exe`, macOS is a `.dmg`, Linux is a Flatpak or bundle.

**Can it download audio only, or just a clip?**
Yes. Extract audio as MP3, M4A, Opus, FLAC, or WAV, or set a start and end time to download only the part you need.

**Are my downloads private?**
Yes. Everything runs locally and your files never leave your computer. There is no telemetry on what you download.

**Can it download Bilibili in 4K, HDR or Hi-Res lossless?**
Yes, with a Bilibili account signed in. OmniGet talks to the official Bilibili API and respects exactly what your 大会员 (premium) subscription unlocks. Without signing in, downloads still work via yt-dlp at standard quality. You can also save danmaku (弹幕) as XML, ASS or JSON, and generate Kodi/Jellyfin NFO metadata for your anime collection.

**Can it follow a channel and grab new videos automatically?**
Yes. Follow a channel and OmniGet polls for new uploads and can auto download them, with a system tray notification.

---

## Build from source

For developers. If you just want to use OmniGet, [grab a release](#get-omniget).

```bash
git clone https://github.com/tonhowtf/omniget.git
cd omniget
pnpm install
pnpm tauri dev
```

Requires [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) 18+, [pnpm](https://pnpm.io/).

<details>
<summary>Linux build dependencies</summary>

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

</details>

<details>
<summary>Windows SmartScreen and macOS Gatekeeper warnings</summary>

**Windows:** SmartScreen may warn you on first run. Click **More info**, then **Run anyway**. This is standard for open source apps without a paid code signing certificate.

**macOS:** If Gatekeeper blocks the app, run in Terminal:

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

Production build: `pnpm tauri build`.

---

## Contribute

Bug or feature idea? [Open an issue](https://github.com/tonhowtf/omniget/issues). Pull requests welcome, see [CONTRIBUTING.md](CONTRIBUTING.md).

OmniGet is translated on [Weblate](https://hosted.weblate.org/engage/omniget/). Pick a language, translate in your browser, and Weblate opens a pull request automatically.

## Notice to platform owners

If you represent a listed platform and have concerns, email **tonhowtf@gmail.com** from a company address. The platform comes off the list right away.

## Legal

OmniGet is meant for personal use. Respect copyright and each platform's terms of service. You are responsible for what you download.

## License

[GPL-3.0](LICENSE). The OmniGet name, logo and Loop mascot are project trademarks not covered by the code license.
