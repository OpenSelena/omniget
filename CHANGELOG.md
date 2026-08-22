# Changelog

All notable changes to the **OmniGet** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.8.12] - 2026-08-23

### Added
- **Open Omni Social Media Downloader**: Full multi-platform downloader interface (`/open-omni`) integrating `gallery-dl` and `yt-dlp` backend engines with active process cancellation and live progress streaming.
- **Open Omni Profile & Settings Management**: Persistent JSON presets for user configurations, domain-specific extractors, authentication cookie injection, and concurrency controls.
- **Loop Mascot Animation Controller**: Reactive mascot state controller responding in real-time to active download phases (Idle, Amazed, Downloading, Queue, Paused, and Error).
- **Built-in Offline Release Notes**: Local bundled release notes fallback ensuring the Changelog page always displays rich update details even when offline or rate-limited.

### Upstream Synchronization
- **macOS Authenticated Downloads**: Integrated direct WKWebView cookie store extraction (`objc2-web-kit`) to resolve HttpOnly auth sessions.
- **Updater & Signing Upgrades**: Synchronized minisign public keys and enabled building from source without mandatory updater signatures.
- **Performance & Reliability**: Resolved dependency-row process storms during CLI binary version queries, improved command palette action search, and added UI skeletons for League client tools.
- **Platform Parser Improvements**: Enhanced Instagram Reels video extraction and Bilibili collection parsing.

### Packaging & Infrastructure
- **Identity Standard**: Standardized bundle identifier to `com.igect.omniget` across Tauri config, Flatpak manifests, and CI/CD pipelines.
- **Automatic Storage Migration**: Non-destructive automatic migration copying existing configurations, credentials, and plugins from legacy data directories (`wtf.tonho.omniget`, `omniget`) to `com.igect.omniget`.
- **Rust Standard Library Modernization**: Upgraded synchronization primitives to standard library `std::sync::OnceLock` and `std::sync::LazyLock`, removing the external `once_cell` crate dependency.

---

## [0.8.11] - 2026-08-17

### Refactored & Optimized
- **Platform Architecture Cleanup**: Removed orphaned and unreferenced legacy platform extractor files from `src-tauri/src/platforms/`, consolidating all platform downloader logic solely within `omniget-core::platforms`.
- **Rust Standard Library Modernization**: Migrated all `once_cell::sync::OnceCell` and `Lazy` occurrences to Rust standard library's `std::sync::OnceLock` and `std::sync::LazyLock`, dropping the external `once_cell` crate dependency across the workspace.
- **Dependency Trimming**: Removed unused `open` crate dependency from Cargo configurations in favor of Tauri's native opener plugin.
- **Fallback Title Generation**: Streamlined generic video title generation in the download queue, removing redundant name tables in favor of lightweight timestamp-based fallbacks.
- **Universal Relative Time Formatting**: Refactored `timeAgo` to leverage native browser `Intl.RelativeTimeFormat`, ensuring accurate locale-aware time formatting without hardcoded dictionaries.
- **Bundle Hygiene**: Removed dev-only audit routes from the production application bundle.

---

## [0.8.10] - 2026-08-06

### Fixed & Refactored
- **CI / Release Workflows**: Updated release workflow repository owner references to `igect` (`#6534f2ee`).
- **App Storage Identifier**: Migrated local app data directory identifier to `com.igect.omniget` (`#d0fa2cd5`).
- **Landing Page & Web Preview**: Redesigned Open Omni web preview interface to match exact Svelte 5 desktop app UI styling (`#b013cde5`).
- **Repository Alignment**: Standardized all project links, references, and footer credits to `igect/omniget` (`#f91f6825`, `#65f13903`).
- **SEO & Dynamic Assets**: Added dynamic release asset fetching, updated default binary asset URLs, and refined SEO metadata (`#a56f2093`, `#3c7e2af8`, `#71b45cd7`).

---

## [0.8.9] - 2026-08-05

### Added
- **Open Omni UI & Mascot**: Updated Open Omni UI components and added interactive loop mascot animations (`#9bcff8f9`).

---

## [0.8.8] - 2026-08-04

### Fixed
- **Open Omni Engine**: Comprehensive bug fixes for download cancellation, stderr tail processing, and UI status sync (`#b658f87c`).
- **Code Style**: Standardized Rust code formatting for `open_omni.rs` (`#0ab08ae8`).

---

## [0.8.7] - 2026-08-03

### Changed
- Release version bump and stability improvements (`#6bae45e0`).

---

## [0.8.0] - 2026-07-28

### Added
- **Tauri 2.0 & SvelteKit 2 Migration**: Complete overhaul of the desktop application using Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`).
- **Platform Plugin Architecture**: Added extensible `PlatformDownloader` backend trait.
- **HLS & Media Engine**: Custom HLS downloader and multi-threaded media processing queue.
- **Multi-language Support**: i18n support across 9 locales.
