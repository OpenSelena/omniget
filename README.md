<p align="center">
  <img src="static/loop.png" alt="Loop, the OmniGet mascot" width="120" />
</p>

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/github/v/release/tonhowtf/omniget?style=for-the-badge&label=release" alt="Latest Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green?style=for-the-badge" alt="License GPL-3.0" /></a>
  <a href="https://github.com/tonhowtf/omniget/stargazers"><img src="https://img.shields.io/github/stars/tonhowtf/omniget?style=for-the-badge" alt="Stars" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases"><img src="https://img.shields.io/github/downloads/tonhowtf/omniget/total?style=for-the-badge&label=downloads" alt="Downloads" /></a>
  <a href="https://hosted.weblate.org/engage/omniget/"><img src="https://hosted.weblate.org/widget/omniget/frontend-json/svg-badge.svg" alt="Translation status" /></a>
</p>

<h1 align="center">OmniGet</h1>

<h3 align="center">Download courses and books, then actually study them.</h3>

<p align="center">
The free, open source desktop app for people who learn from <strong>online courses</strong> (Hotmart, Udemy, Kiwify, Skool, Teachable) and from <strong>books</strong> (PDF, EPUB, CBZ). You download once, you own the file, and you study without leaving the app.
</p>

<p align="center">
  <img src="assets/screenshot.png" alt="OmniGet home screen" width="800" />
</p>

## Get it

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-Windows-blue.svg?style=for-the-badge&logo=windows" alt="Download for Windows" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-macOS-black.svg?style=for-the-badge&logo=apple" alt="Download for macOS" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/badge/-Linux-orange.svg?style=for-the-badge&logo=linux&logoColor=white" alt="Download for Linux" /></a>
</p>

Portable `.exe` on Windows, Flatpak on Linux, and a regular `.dmg` on macOS. No setup. The app keeps its own dependencies up to date in the background.

## What is it for

OmniGet is a downloader that grew up. The downloader half pulls media from the sites you already use. The study half is what makes it different: a real video player and a real book reader, with notes, flashcards and progress tracking, all working on the files you already have on your disk.

Three things you can do today:

1. **Download an entire online course** from platforms like Hotmart or Udemy and watch every lesson in OmniGet itself.
2. **Open a PDF or EPUB** and read it with bookmarks, highlights and a focus mode that hides everything except the page.
3. **Paste any video link** (YouTube, Instagram, TikTok, and friends) and get an MP4 in a few clicks.

The rest is small things you discover later. Torrents, P2P file transfer, FFmpeg conversion, a Telegram chat browser, a global hotkey, a browser extension, themes, languages.

---

## Watch courses inside OmniGet

Most course downloaders give you a folder full of files and leave you alone. OmniGet does the download (Hotmart, Udemy, Kiwify, Gumroad, Teachable, Kajabi, Skool, Wondrium, Thinkific, Rocketseat) and then opens them in a built in player that turns the folder into a real learning environment.

Open the **Library**, scan a folder once, and OmniGet builds a course list with cover art and progress for each course.

Inside a lesson you get:

- **A real video player** with playback speed (0.5× to 2×), keyboard shortcuts, fullscreen, picture in picture, theater mode, autoplay of the next lesson, and a 5 second countdown you can cancel.
- **Auto resume** at the exact second you stopped, per lesson.
- **Subtitles** when the course ships them (`.vtt` files), with a built in toggle.
- **Notes pinned to timestamps**: write a note at minute 12:34, click it later, the player jumps to that second. Markdown supported.
- **Attachments panel** that lists every PDF, image, text and code file the course came with. Click to preview right next to the video, no second app needed.
- **Lesson description** rendered cleanly when the course provides one (`description.html`).
- **Continue watching** widget on the home screen with the last courses you opened.

Around the courses, the **Library** itself has:

- Tags and collections (group your engineering courses, your design courses).
- Status filters (not started, in progress, completed) and a search across titles.
- Health check that finds zero byte videos, missing files and orphan attachments after you move things around on disk.

Everything lives in the OmniGet database. The original folder on disk is never touched, so you can move files, back them up, or open them in VLC anytime.

---

## Read PDFs and EPUBs inside OmniGet

The reader is the part most people don't expect. OmniGet ships a full document reader for **PDF**, **EPUB**, **CBZ**, **TXT** and **HTML**. You don't need Calibre, you don't need Adobe, you don't upload files to a website.

The library:

- **Auto extracts covers** from PDF, EPUB and CBZ files when you scan a folder.
- **Search Anna's Archive** from inside the app and download books legally available to you. Configure your own sources and domains.
- **ISBN metadata enrichment** that fills in title, author, publisher and a high quality cover with one click.
- **Filter** by reading status (all, reading, finished, favorites) or by format (PDF, EPUB, TXT, HTML, CBZ).
- **Sort** by title, author, recently added, or last opened.
- **Tags and collections** so your library doesn't drown in 500 books.
- **Multiple library roots**: keep technical PDFs in one folder and fiction EPUBs in another, OmniGet treats them as one library you can filter.

Inside a book:

- **Two reading modes**: paged (book feel) and scroll (web feel). Your choice is remembered per session.
- **Outline / table of contents** for PDFs and EPUBs, click any entry to jump.
- **Bookmarks** with one keystroke, with a panel listing all of them.
- **Highlights** in different colors, with notes attached.
- **Notes panel** that lives next to the page, written in Markdown.
- **Search inside the book** with hits highlighted in context.
- **Zoom** for PDFs (DPI based, sharper than scaling) and **font size + font family** for EPUBs.
- **Multiple reader themes**: clean white, sepia paper, dark, plus a paper filter that mimics e ink.
- **Cursor line guide** that follows your mouse, helpful for long lines and dyslexia.
- **Focus mode** that hides everything except the page, full keyboard navigation, no chrome.
- **Reading progress** tracked automatically by page or by location, with last opened time.
- **Reading sessions** logged so you can see how long you actually spent reading each day.
- **Manga mode** for CBZ files with right to left page flow.

Books and notes are kept in a local SQLite database. Your annotations move with the file when you back up your library.

---

## Where you can download from

You paste a link, OmniGet figures out the site, shows a preview with quality options, and downloads the file. That is the whole loop.

| What | Examples |
|------|----------|
| Online courses | Hotmart, Udemy, Kiwify, Gumroad, Teachable, Kajabi, Skool, Wondrium, Thinkific, Rocketseat |
| Video and audio | YouTube, Instagram, TikTok, Twitter / X, Reddit, Twitch, Pinterest, Vimeo, Bluesky, Bilibili |
| Asian platforms | Douyin (抖音), Xiaohongshu (小红书), Kuaishou (快手), Youku (优酷), iQiyi (爱奇艺), Tencent Video, Mango TV |
| Books | Anna's Archive (in app search), local files |
| Files | `.torrent` and magnet links, P2P direct transfer between two computers using a 4 word code |

If yt-dlp supports a site, OmniGet downloads from it. That covers roughly a thousand sites in total.

## What else is inside

Things that are there if you need them, easy to ignore if you don't:

- **Pomodoro focus timer** that pauses your video player when the session ends.
- **Spaced repetition flashcards** (SM2, the Anki algorithm), with `.apkg` import and AnkiWeb sync.
- **Notes app** with bidirectional links between pages, daily journal, knowledge graph and templates.
- **Progress dashboard** with a streak counter, daily goals, GitHub style heatmap, and weekly minutes.
- **FFmpeg converter** for local file conversion, no upload to anywhere.
- **Telegram chat browser** that reads your account and lets you save photos, videos and files from any chat.
- **Browser extension** (Chrome and Firefox) that hands the current page to OmniGet with one click.
- **Global hotkey** (`Ctrl+Shift+D`) that downloads whatever URL is in your clipboard.
- **9 languages**: English, Portuguese, Chinese, Traditional Chinese, Japanese, Italian, French, Greek.
- **14 themes** including Catppuccin (mocha, macchiato, frappé, latte), Dracula, One Dark Pro, NyxVamp and three e ink variants.

## How it actually feels

Copy a link anywhere (a tweet, a Discord message, a tab). Press `Ctrl+Shift+D`. The app downloads in the background. You don't even open the window.

Or paste the link in the omnibox, glance at the preview, click download.

For courses, log in once on the platform, browse your library, pick what you want, walk away. The app handles every lesson and attachment.

For books, drop them in a folder you already use, scan once, and they show up with covers in the library.

## Build from source

For developers. If you just want to use the app, [grab a release](#get-it).

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

**Windows:** SmartScreen may warn you on first run. Click **More info**, then **Run anyway**. Standard for open source apps without a paid code signing certificate.

**macOS:** If Gatekeeper blocks the app, run in Terminal:

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

Production build: `pnpm tauri build`.

## Contribute

Bug or feature idea? [Open an issue](https://github.com/tonhowtf/omniget/issues). Pull requests welcome, see [CONTRIBUTING.md](CONTRIBUTING.md).

OmniGet is translated on [Weblate](https://hosted.weblate.org/engage/omniget/). Pick a language, translate in your browser, Weblate opens a pull request automatically.

## Notice to platform owners

If you represent a listed platform and have concerns, email **tonhowtf@gmail.com** from a company address. The platform comes off the list right away.

## Legal

OmniGet is meant for personal use. Respect copyright and each platform's terms of service. You are responsible for what you download.

## License

[GPL-3.0](LICENSE). The OmniGet name, logo and Loop mascot are project trademarks not covered by the code license.
