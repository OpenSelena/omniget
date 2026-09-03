# Changelog

All notable changes to **OmniGet** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For full release notes, assets, and signatures of each version, see
[GitHub Releases](https://github.com/tonhowtf/omniget/releases).

## [Unreleased]

## [0.9.0] — 2026-08-29

### Added
- **Chat is enabled by default** and pre-connected to `chat.tonho.wtf`: the
  first screen is register/login, not a server address. Running your own
  instance is still one click away under "change server".
- **A community server everyone joins on sign-up**, with Welcome, Support and
  Community categories and one channel per officially supported language.
- **Screen sharing on Linux** (X11 and Wayland) through the
  xdg-desktop-portal ScreenCast interface and PipeWire, with system audio from
  the default sink monitor. Hardware encoding uses VAAPI or NVENC when
  `VideoEncoderBackend::list_available()` reports them, and the inspector
  reports the backend actually selected rather than the one hoped for.
  It is a build feature (`linux-capture`, on by default) because `libspa` will
  not compile against PipeWire as old as Ubuntu 22.04 ships; the x86_64 release
  stays on that base to keep its glibc floor, so that artifact is voice-only and
  `MediaCapabilities` reports the loss instead of offering a button that fails.
  The arm64 build has both.
- **Local preview of your own screen share**, emitted as a periodic JPEG from
  the active capture filter and rendered on the sharer's tile.
- **Temporary file storage**: attachments are deleted from the server
  `OMNIDISC_ATTACHMENT_TTL_SECONDS` (default 30 minutes) after upload. The row
  survives as a tombstone so a message reads "this file expired" instead of
  offering a dead link, and the media endpoint answers `410 Gone`.
- **Encryption at rest for uploads**: blobs are written as XChaCha20-Poly1305
  frames under a key derived per blob from the instance secret, so a stolen
  disk or a copied backup yields nothing. Range requests still stream.
- **Disk-pressure policy**: no per-user quota; the volume is the budget. Users
  are warned at 80 % and every stored file is deleted at 95 %, announced over
  the gateway as `STORAGE_STATUS` and shown as a banner in the client.
- **Upload rate limits** per account (requests and bytes) and a `/api/storage`
  endpoint reporting fill ratio and retention window.
- **Focus mode** in the chat rail, hiding the main OmniGet sidebar.
- Unread and mention badges on the server rail; collapsible channel categories.

### Fixed
- **The microphone on macOS.** The app never declared
  `NSMicrophoneUsageDescription` and never requested TCC authorisation, so
  CoreAudio started successfully and delivered digital silence forever: the
  user appeared connected and was inaudible, with no error anywhere. Permission
  is now requested through AVFoundation before the input stream opens, and a
  denial surfaces as `ERR_VOICE_MIC_PERMISSION`.
- **Screen shares were invisible to viewers.** Discovery depended entirely on
  the gateway relaying `self_stream`; the LiveKit `TrackPublished` event never
  reached the frontend, so with an older server nobody saw the stream and the
  Watch button never appeared. Screenshare tracks now raise an engine
  notification of their own.
- The stream pop-out could not go full screen or close itself — the window was
  not covered by any Tauri capability — and closing it natively leaked its
  viewer, permanently preventing re-watching that person.
- Leaving a voice channel while sharing left the stream published.
- A microphone that delivers only digital silence is now reported, instead of a
  level meter that never moves.
- Notifications are delivered for the channel you have open when the window is
  in the background.
- Direct messages no longer appear inside a server's channel list.
- The Portuguese translation of the screen-sharing interface, which was still
  English.

### Changed
- Animations pause and the screen-share preview stops while the app window is
  not focused, so a chat behind a game costs as little as possible.
- The janitor sweeps every minute instead of every fifteen, which is what makes
  a thirty-minute retention promise measurable.

## [0.6.2] — 2026-05-18

### Added
- **Cookie Manager** (`Settings → Cookies`): per-platform cookie buckets with
  visual cards (platform logo, inline-editable alias, clickable source URL,
  freshness status). Import via browser extension popup ("Save site cookies"
  button), file picker, or paste modal. Multi-account support per domain.
  See `COOKIES.md` for architecture details.
- **yt-dlp per-domain cookie resolution**: `set_per_domain_cookie_fn` wires
  the Cookie Manager into yt-dlp invocations app-wide — downloads from
  YouTube, SoundCloud, Hotmart and any other supported site automatically
  pick up the right cookies from the new multi-file layout.
- **Plugin SDK**: new `PluginHost::get_cookie_file(domain, account?)` and
  `cookie_status(domain)` methods with default impls (zero ABI break — plugins
  built against earlier SDKs continue to load). Exported `CookieStatus` enum.
- **Torrent file selection**: pasting a magnet/`.torrent` resolves its file
  list first; the home page shows a checkbox picker (sizes shown, all selected
  by default) so only chosen files download.
- **Torrent peer discovery**: a curated set of stable public trackers is
  injected into every torrent, and UPnP/NAT-PMP port mapping is enabled, so
  magnets with few/dead trackers still find peers. Both toggleable in
  `Settings → Advanced`.
- **Channel Follow** (`Settings → Channels`): follow a channel/playlist URL;
  a background poller checks for new uploads on a per-channel interval, can
  auto-download them, and notifies (in-app toast + system notification). A
  localized tray submenu lists followed channels with "check now".
- **AI features** (`Settings → AI`): configure OpenAI, Anthropic, or any
  local/OpenAI-compatible endpoint (Ollama, LM Studio…). Keys are stored
  locally and never leave the device, sent to logs, or telemetry. Powers:
  video summary (from subtitles, with style + output-language options),
  Whisper transcription fallback for videos without subtitles, and an
  AI/quick-action **video tools** overlay (deterministic presets +
  natural-language → reviewed ffmpeg command, gated by a security allowlist).
- **Subtitle workshop** (`Downloads → Tools`): open SRT/VTT, edit timing/text,
  shift, find/replace, split/merge, QC hints, waveform + shot-change strip,
  AI translate, save as SRT/VTT.
- **Download scheduling**: optionally start a download at a chosen time (with
  presets) and an optional auto-stop time, from the home advanced options.
- **Tray download speed**: the tray tooltip now shows aggregate speed while
  downloads are active (throttled, no icon flicker).
- **Keep system awake** while downloads are active (`Settings → Advanced`,
  on by default) so long downloads survive an idle machine.
- **Delete file on remove**: completed downloads gain an explicit, separate,
  confirmation-gated "delete file from disk" action (the normal remove is
  unchanged — no surprise data loss).

### Changed
- Browser extension popup now exposes a manual "Save site cookies" button that
  ships the active site's cookies to the Cookie Manager via the existing
  localhost bridge (`POST /v1/cookies`), with metadata (source URL, page title)
  feeding the Cookie Manager UI.
- The localhost bridge `CookiesRequest` schema accepts optional
  `sourceUrl`/`pageTitle`/`alias` fields. Existing extensions remain compatible
  (fields are `#[serde(default)]`).
- `chrome-extension-cookies.txt` (single-file legacy) is now repartitioned into
  `cookies/<domain>/_default.txt` on first startup, and **the legacy file is
  deleted afterwards**. The multi-file layout is the only source going forward.
- `Music → YouTube` hub "Connect" action and `Sources Order → YouTube Music
  Connect` now navigate to `Settings → Cookies` instead of the legacy YouTube
  cookies drawer.

### Removed
- Legacy `YoutubePanel.svelte` cookies drawer (functionality fully replaced by
  Settings → Cookies). Related store fields `musicUI.youtubeOpen` and methods
  `openYoutube()` / `closeYoutube()` removed.
- Legacy single-file cookie storage path (`chrome-extension-cookies.txt`):
  yt-dlp no longer reads from it, the localhost bridge no longer writes to it
  in parallel, and the plugin SDK no longer falls back to it. The
  `extension_storage::write_extension_cookies` function remains as dead code
  for the moment but has no callers.
- Dead duplicate yt-dlp flag catalog + helpers (`YTDLP_FLAG_CATALOG`,
  `toggleFlag`, `isFlagActive`, `getFlagValue`, `setFlagValue`) in
  `routes/settings/+page.svelte` (the live copy lives in `SettingsNetwork`).

### Fixed
- **Hardcoded strings**: Marketplace install-error fallback, the global Toast
  close button `aria-label`, and the Settings → yt-dlp flag chips/placeholders
  ("Embed subtitles", "Limit rate", "e.g. 1M"…) were English-only; all now go
  through i18n across the 9 locales.

## [0.6.0] — 2026-05-10

### Added
- **Music download hotkey** in Settings → Downloads. Independent global shortcut (default `CmdOrCtrl+Shift+M`) that grabs the URL from the clipboard and routes it to the study plugin's audio downloader. Audio format pickable in settings (m4a / mp3 / flac / opus / wav).
- **Browser extension settings panel** in Settings → Plugins. Lists Chrome / Edge / Brave / Firefox / Safari with the bundled extension version; "Update / Install" extracts the bundled extension to a stable path inside the app data folder and opens it in the file explorer with install instructions per browser.
- **Browser extension bundled as Tauri resource** so the install/update flow works on packaged builds.
- **`browser_extension_status`, `browser_extension_export`, `browser_extension_open_folder`** Tauri commands.

### Fixed
- **Cookie file overwrite bug** in `native_host.rs::write_extension_cookies`. Each captured platform was wiping all previously written cookies because the file was opened with `fs::write`. The new flow merges by root domain — TikTok captures no longer kill SoundCloud auth, etc. This unblocks SoundCloud login via the browser extension.

### Changed
- **Browser extension** bumped to 0.4.0 with cookie auto-capture for SoundCloud (`oauth_token`, `sc_anonymous_id`), `tabs.onUpdated` listener, and `scanOpenTabsForCookies` on extension load.

## [0.5.2] — 2026-05-06

### Added
- **Anki backup manager** in `/study/anki/settings`: list, verify (with integrity badge), restore (with confirmation), and "keep latest N" cleanup.
- **Anki tag manager** at `/study/anki/tags`: tree view with collapse, rename/reparent modals, "clear unused" action.
- **Anki media manager** at `/study/anki/media`: list with filters (all / unused / missing), bulk trash, in-session trash log with restore, empty trash with confirmation.
- **Anki export/import**: `.colpkg` import support and dedicated export section (`.apkg`, `.colpkg`, `.json`, `.csv` per notetype) in `/study/anki/import`.
- **Anki filtered decks** at `/study/anki/decks/filtered`: create/rebuild/empty/delete with query presets and order options.
- **Anki deck presets CRUD** at `/study/anki/decks/presets`: create/edit/delete with full FSRS/learn-step/leech config.
- **Anki revlog stats** at `/study/anki/stats/revlog`: filter by deck, notetype, tag, time range, or specific card; ease-rate summary.
- **Anki browse**: bulk add/remove tags, "unbury deck" action, in-drawer note edit/delete, sibling cards listing.
- **Read downloads manager** at `/study/read/downloads`: live progress, retry, cancel, clear finished, list torrent mirrors with magnet copy.
- **Read global annotation search** at `/study/read/search`: full-text across highlights/notes with rebuild-index button.
- **Alt Library** (formerly Anna's Archive): clear cache and headless browser toggle in advanced sources settings.
- **Pets settings**: pagination (24 per page) to avoid UI freeze on large collections.

### Changed
- **Library health banner** rewrite: replaced alarmist `⚠️ N problemas` and AI-slop `(s)` plurals with humane copy, neutral dot indicator, and concrete `Revisar` CTA.
- **Anna's Archive → Alt Library** across all UI strings (8 locales) and backend display strings. Internal identifiers (`annas_*` keys, `AnnasSource`, etc) kept for code stability.
- **Library page**: 24 hardcoded Portuguese strings migrated to i18n keys with proper translations across 8 locales.
- **MaintenancePanel** decorative emojis removed for cleaner button labels.
- **Course-page bulk action toasts** rewritten with proper plurals (`1 aula marcada` vs `N aulas marcadas`) instead of `(s)`.
- **Sidebar/EmbedView/InlineToolbar emojis** replaced with consistent inline SVG icons that respect theme colors.

### Fixed
- **`cancelDownload` for course downloads** (#102 follow-up): plugin command name was wrong (`cancel_download` instead of `cancel_course_download`) and arg was snake_case where backend expected camelCase. Hotmart, Udemy, and Rocketseat course cancellations now work.

### CI
- New `rust-debian` job validates that the Rust workspace compiles in a `debian:bookworm` container.
- Release notes now include explicit guidance for Debian/Ubuntu users about `libfuse2` for AppImage compatibility.

## [0.5.1] — 2026-05-05

### Fixed
- Plugin loader surfaces ABI-mismatch errors instead of looping on "restart required" forever (#102). Plugins built against an older SDK now show "Plugin incompatible with this version" with a one-click path to the Marketplace, and the Marketplace card displays the underlying load error.

### Changed
- `list_plugins` Tauri command now returns `load_error: { message, kind, plugin_abi, expected_abi }` on plugins that failed to load.

### i18n
- Replaced 17 hardcoded Portuguese strings in the study command palette (`+layout.svelte`) with `study.palette.*` keys, translated across all 8 locales.
- Added `marketplace.plugin_incompatible_*`, `marketplace.plugin_load_failed_*`, `marketplace.plugin_load_error_details`, and `marketplace.reinstall` keys.

## [0.5.0] — 2026-05-03

### Added
- Pets system: download/install gamification companions for the study plugin.
- `Mascot` celebrations: new `amazed` state triggered on first-of-session and on batches of 5+ successful downloads.
- `celebrate/` component for download-completion visuals.
- `pdfium` core module powering the study reader.
- `host_queue` command for plugin-driven download orchestration.
- Autostart on system boot.
- `scripts/bump-version.js` — single-command version bump across 8 files (package.json, Cargo.toml, tauri.conf.json, metainfo.xml, changelog-store, etc.) with `appstreamcli` validation.
- `scripts/generate-i18n-keys.js` — auto-generated `TranslationKeys` union type with drift audit across all 9 locales.
- `scripts/sync-locales.mjs` — fills missing translation keys from `en.json`.
- `scripts/check-i18n-usage.mjs` — scans `$t(...)` usages and reports keys missing from `en.json`.
- `scripts/contrast-audit.mjs` — color-token contrast audit for all 14 themes.
- Build-time git metadata (`__COMMIT_HASH__`, `__GIT_BRANCH__`, `__APP_VERSION__`, `__BUILD_DATE__`) surfaced on the About page.
- Multi-OS CI matrix (Ubuntu / Windows / macOS) for the Rust workspace and a dedicated frontend `svelte-check` job.
- `appstreamcli validate` step in CI to catch Flatpak metainfo regressions early.
- Retry logic (10× × 30s) in the release workflow when waiting for `.sig` asset propagation.
- `aria-live` regions on toasts (`alert`/`assertive` for errors, `status`/`polite` otherwise).

### Changed
- Theme grid restructured (System / Light / Dark visible up-front, the rest under "More themes").
- Settings reorganized: Downloads-first, plain-language labels, cookie file relocated, YouTube split out.
- Torrent session now stops gracefully on app exit (`RunEvent::ExitRequested`) with a 5s timeout.
- Pinterest platform regex patterns moved to `static LazyLock<Regex>` — init-time validation instead of runtime panic.
- `/about/debug` gated behind a debug toggle.

### Removed
- League and Misc/social plugins fully removed (frontend, Rust glue, i18n).
- Dead toggle components: `Toggle.svelte`, `SettingsToggle.svelte`, `Switcher.svelte` (none were imported anywhere; inline `<button class="toggle">` in settings is the de-facto implementation).

### Fixed
- `cancel_course_download` long-standing bug.
- `study/read/books/annas-settings`: null-guard on `settings.domains` revert handler.

## [0.4.0] — 2026-04-04

### Added
- Firefox browser extension (parity with Chrome extension).
- Extra-headers support in download options.
- Context menu on detected media for quick download.
- Download statistics tracking and completion events surfaced on the About page.
- OTP verification flow and browser-login method for course platforms.
- `open_auth_webview` command with cookie polling and initialization scripts for authentication flows.
- Quick action buttons replacing the "more actions" dropdown (Fitts' Law improvement).
- Theme grid with additional themes; visual cards instead of dropdown.
- Download cancellation while a download is running.
- Mascot personality: varied bubble texts per state across all locales.
- Pokémon-style random names for generic video titles without metadata.
- `Cancel` translations across all 9 locales.

### Changed
- `Mutex` replaced with `RwLock` in the plugin manager and plugin command routing to improve concurrency.
- Download label clarity and platform detection logic.
- Enhanced ffmpeg handling: file-size validation, quarantine removal on macOS, consistent detection via `find_tool()`.

### Fixed
- Media detection on profile content types with improved cookie deduplication.
- Confirmation messages for download removal across all locales.
- Error classification for missing downloaded files with longer recency window.
- Subtitle paths no longer overwrite the captured video path.

## [0.3.7] — 2026-03-19

### Added
- Global cookie file setting (`download.cookie_file`) with periodic check and toast notification on access errors.
- `check_cookie_error` command.
- Lesson descriptions saved as HTML/text files alongside downloads (Medway, Memberkit, Fluency, and more).
- Release workflow: signing pipeline and updater JSON generation.
- `socket2` with all features (for P2P improvements).

### Changed
- Release workflow: Apple certificate secrets removed; platform signature updates automated.
- Debug / release build profiles tuned (codegen units, debug options).
- Browser cookies supported in video-info retrieval phase.

### Removed
- Flatpak build workflow from release pipeline (Flatpak now tracked separately via Flathub).

## [0.3.6] — 2026-03-08

### Added
- **Bilibili** platform support (URL parser, downloader, UI services list).
- **Torrent** (magnet) support listed in services.
- **P2P file transfer** with UDP hole punching and STUN integration; 6-char pairing codes.
- Debug panel with log filtering and diagnostics export.
- Parallel download with DC switching and progress tracking for Telegram.
- Greek, French, Italian, Japanese, Portuguese, Chinese translations for proxy + torrent + P2P features.
- Toast logging in `showToast`.

### Fixed
- yt-dlp format selector: video-only fallback for DASH sites without ffmpeg.
- Telegram: DC migration (`FILE_MIGRATE`), DC switching in sequential download, timeout handling for chat/media fetching, unsupported peer IDs gracefully ignored.
- Redundant QR logins preserve the active session instead of invalidating it.
- Clipboard URL pattern extended to recognize `p2p://` protocol.

## [0.3.5] — 2026-03-07

### Added
- Greek language support in the settings language selector.
- Open-folder reveal action on download items with `revealFile` command; i18n in 7 locales.
- Rate limiting for HTTP 429 errors in video downloads.
- Retry logic for file replacement in `embed_metadata`.
- WebKitGTK DMABuf-renderer crash workaround for Wayland.
- Translated yt-dlp backend errors to the user's locale.

### Changed
- ffmpeg availability check: caching mechanism, empty path filtering, location-based instead of boolean flag.
- Light theme color variables tuned for contrast.
- Component styles migrated to CSS variables for box-shadow and button backgrounds.

### Fixed
- yt-dlp stderr read via `wait_with_output` instead of a separate task; meaningful error lines extracted.

## [0.3.4] — 2026-03-01

### Fixed
- ffmpeg detection inconsistency: 3 functions (`find_tool`, `is_ffmpeg_available`, `find_ffmpeg_location`) now delegate to `find_tool("ffmpeg")` which also validates macOS quarantine attributes.
- Subtitle paths no longer hijack `captured_path` when yt-dlp emits `[download] Destination: *.vtt`.
- Info-fetch phase no longer passes `-f <format>` so restricted videos can still produce metadata.

## [0.3.3] — 2026-03-01

### Added
- 

## [0.3.2] — 2026-02-27

### Added
- 

## [0.3.1] — 2026-02-26

### Added
- Rocketseat course platform support.

## [0.3.0] — 2026-02-24

### Added
- Plugin SDK and marketplace (`omniget-plugin-sdk`, `PluginManager`, libloading-based DLL loader).
- `courses` plugin (external) with generic `[platform]` frontend page.
- `telegram` plugin (external).
- `convert` plugin (external, ffmpeg-based media converter).

## Older versions

See [GitHub Releases](https://github.com/tonhowtf/omniget/releases) for 0.2.x and 0.1.x release notes. Tags: `v0.2.0`..`v0.2.16`, `v0.1.0`.
