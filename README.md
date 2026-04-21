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

<h3 align="center">Paste a link. Get your file.</h3>

<p align="center">
OmniGet is a free, open-source desktop app that downloads videos, courses, music, and files from the sites you already use — YouTube, Instagram, TikTok, Hotmart, Udemy, SoundCloud, and <a href="https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md">1000+ more</a>. One link in, one file out.
</p>

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

Also available as a Flatpak on Linux and a portable `.exe` on Windows. No setup: OmniGet bundles [yt-dlp](https://github.com/yt-dlp/yt-dlp) and FFmpeg, keeps them up to date, and handles everything in the background.

## What OmniGet does

You paste a URL. OmniGet figures out the site, shows you a preview with quality options, and downloads the file. That's the whole loop.

Underneath, it covers four big jobs:

- **Videos and audio** from 1000+ sites via yt-dlp.
- **Entire online courses**, login included, every lesson and attachment on disk.
- **Torrents and magnet links** with a built-in client.
- **Direct file transfer** between two computers with a 4-word code.

Around that, the app ships with Loop (a mascot that reacts to your downloads), 11 color themes, 9 languages, a global hotkey, and an optional browser extension.

### Supported video and audio sites

| Platform | What you can grab |
|----------|-------------------|
| YouTube | Videos, Shorts, playlists, search results |
| Instagram | Posts, reels, stories |
| TikTok | Videos, photos |
| Twitter / X | Videos, GIFs |
| Reddit | Videos, images |
| Twitch | Clips |
| Pinterest | Images, videos |
| Vimeo | Videos |
| Bluesky | Images, videos |
| Bilibili (哔哩哔哩) | Videos, series |
| Telegram | Photos, videos, files (via plugin) |
| Torrent / Magnet | Any `.torrent` file or magnet link |

If a site is [supported by yt-dlp](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md), OmniGet can download from it. Roughly 1000 sites in total.

<details>
<summary><strong>Chinese platforms</strong> (via yt-dlp)</summary>

| Platform | What you can grab |
|----------|-------------------|
| Douyin (抖音) | Videos |
| Xiaohongshu (小红书) | Videos, images |
| Kuaishou (快手) | Videos |
| Youku (优酷) | Videos |
| Tencent Video (腾讯视频) | Videos |
| iQiyi (爱奇艺) | Videos |
| Mango TV (芒果TV) | Videos |

Some of these may need a Chinese IP address.

</details>

## Plugins

OmniGet starts minimal. Everything beyond plain downloading is an optional plugin you install from the built-in marketplace. Each one drops a new section into the sidebar.

### Courses — download entire online courses

Log in once, pick what you want, walk away. OmniGet downloads every lesson, every attachment, and the lesson descriptions, organized on disk as `Course / Module / Lesson.mp4`.

Ten platforms supported:

| Platform | Login | Region |
|----------|-------|--------|
| Hotmart | Email + password | BR / Global |
| Udemy | Email + browser login | Global |
| Kiwify | Email + password or token | BR |
| Gumroad | Email + password or token | Global |
| Teachable | Access token | Global |
| Kajabi | Access token | Global |
| Skool | Email + password or token | Global |
| Wondrium / Great Courses | Email + password or token | US |
| Thinkific | Browser login | Global |
| Rocketseat | Access token | BR |

Repository: [omniget-plugin-courses](https://github.com/tonhowtf/omniget-plugin-courses).

### Study — turn your downloaded courses into a study app

Study reads the courses already on your disk and wraps them in a real learning environment. It doesn't need to re-download anything: it scans the folders, builds a library, and adds the tools you'd expect from a paid study platform.

What it gives you:

- **A proper video player** with 0.5×–2× speed, picture-in-picture, theater mode, auto-resume from where you stopped, auto-play of the next lesson, and subtitles in 9 languages.
- **Notes tied to timestamps.** Click a note to jump back to the exact second. Markdown and LaTeX supported. Export to Markdown.
- **Bookmarks** inside any lesson with a single keystroke.
- **Flashcards with spaced repetition** (SM2 algorithm, the same one Anki uses). Rate each card *Again / Hard / Good / Easy* and Study schedules the next review for you.
- **Daily streak, goals and a GitHub-style activity heatmap.** A day counts when you watch lessons, review cards, or finish a focus session.
- **Pomodoro focus timer** with session history.
- **Unified search** across courses, notes, and flashcards.
- **Continue-watching** on the home screen.

Repository: [omniget-study](https://github.com/tonhowtf/omniget-study).

### League — a companion for League of Legends

League turns OmniGet into a desk-side helper while you play LoL. Nothing runs until you click *Activate* — the plugin loads inert with no connections, no background threads, no listeners.

Once activated, it can:

- **Auto-accept** queue pops and watch game flow events.
- **Read champ-select** and show cards for each player: rank, recent games, winrate, warning flags for saved trolls or smurfs.
- **Install and manage mods** (skins and custom huds) with a single toggle per profile.
- **Index your replays** (`.rofl` files), run the in-replay camera, and capture clips.
- **Privacy toggles** for appearing offline, online, or mobile independently of the client.
- **Inspect the League client traffic** if you're curious about what the LCU is doing.

Everything is optional. Sub-modules turn on and off one by one, and two of them (*Privacy* and *Devtools*) never auto-activate even if you enable auto-start.

Repository: [omniget-plugin-league](https://github.com/tonhowtf/omniget-plugin-league).

### Misc — a grab bag of useful things

Misc is the plugin where small, useful utilities live. Right now it bundles tools for four sites:

- **GitHub.** Find out who stopped following you, who starred (or unstarred) your repos, see a leaderboard of your biggest fans across all repos, and keep an eye on a specific user's public activity with optional toast notifications.
- **Instagram.** Download posts, reels, and stories from public profiles by username, hashtag, or location.
- **SoundCloud.** Download tracks and full playlists, embed cover art and metadata into the audio files, mirror a playlist locally (sync both ways), see your charts and trending tracks, and show what you're listening to in your Discord status.
- **Telegram account lookup.** Paste a `@username` and get public profile info back: photo, bio, account creation estimate, and a quick readability report of the bio (language, hints at age or city, emoji usage).

Think of it as the *miscellaneous* drawer. Features land here when they're useful but don't justify a plugin of their own.

Repository: [omniget-social](https://github.com/tonhowtf/omniget-social) (folder name kept for historical reasons — the plugin surface is called *Misc*).

### Telegram — browse chats and download media

Open your Telegram account inside OmniGet and pull photos, videos, and files out of any chat. 16 commands, covers the common cases.

Repository: [omniget-plugin-telegram](https://github.com/tonhowtf/omniget-plugin-telegram).

### Convert — local media converter

Thin wrapper around FFmpeg: drag a file in, pick the target format, get the output. Works offline, no upload to anything.

Repository: [omniget-plugin-convert](https://github.com/tonhowtf/omniget-plugin-convert).

### Build your own

Plugins are external Rust crates. If you want to add a site or a workflow, start from the [Plugin SDK](src-tauri/omniget-plugin-sdk/).

## How it works in practice

1. **Paste a link** into the omnibox. Or drag a `.torrent` file, or type a search straight into YouTube.
2. OmniGet recognizes the site and shows a preview with available qualities.
3. Hit download. Progress, speed, and ETA update live.

For courses: log in to the platform, browse your library, pick what you want, and download it in one pass.

### Copy. Press. Done.

Copy a link anywhere — Discord, a group chat, a tweet. Press **Ctrl+Shift+D** (or **Cmd+Shift+D** on macOS). OmniGet reads the clipboard and downloads in the background. You don't even need to open the window.

Change the hotkey in **Settings > Downloads > Hotkey**.

### Browser extension

Install the [Chrome extension](browser-extension/chrome/README.md) to skip the copy-paste step. When you land on a page with a video, click the OmniGet icon and the extension hands the URL (and any login cookies the app needs) over directly.

The extension also sniffs video streams on sites OmniGet doesn't officially support. If your browser can play it, OmniGet can usually download it.

## Building from source

For developers. If you just want to use OmniGet, [grab a release](#download).

**Prerequisites:** [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) 18+, [pnpm](https://pnpm.io/).

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

Production build: `pnpm tauri build`.

<details>
<summary><strong>Windows SmartScreen / macOS Gatekeeper</strong></summary>

**Windows:** SmartScreen may warn you on first run. Click **More info**, then **Run anyway**. This is normal for open-source apps without a paid code-signing certificate.

**macOS:** If Gatekeeper blocks the app, run in Terminal:

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

## Contributing

Found a bug or want a feature? [Open an issue](https://github.com/tonhowtf/omniget/issues). Pull requests are welcome — check [CONTRIBUTING.md](CONTRIBUTING.md) before you start.

## Translating

OmniGet ships in 9 languages (English, Portuguese, Chinese, Traditional Chinese, Japanese, Italian, French, Greek) and is hosted on [Weblate](https://hosted.weblate.org/engage/omniget/). Pick a language, translate in the browser, Weblate opens a pull request for you. See [docs/translations.md](docs/translations.md) for details.

## Notice to platform owners

If you represent a listed platform and have concerns, email **tonhowtf@gmail.com** from a company address. We'll remove the platform right away.

## Legal

OmniGet is meant for personal use. Please respect copyright and each platform's terms of service. You're responsible for what you download.

## License

[GPL-3.0](LICENSE). The OmniGet name, logo, and Loop mascot are project trademarks not covered by the code license.
