# Changelog

All notable changes to the **OmniGet** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.8.15] - 2026-08-31

### Fixed & Improved
- **Open Omni Responsive Workspace**: Reworked the Open Omni layout so Download, Profiles, and Settings use the available desktop space rather than a cramped fixed-size card.
- **Theme & Accessibility Compatibility**: Aligned Open Omni controls, dialogs, focus indicators, and text colors with the app theme system, including light, dark, and reduced-motion support.
- **Small-Window Layout**: Added adaptive navigation and form layouts so profile controls, filters, and settings remain usable at narrow widths.

---

## [0.8.14] - 2026-08-30

### Added & UI
- **Open Omni System Settings Interface**: Redesigned `/open-omni` layout with macOS System Settings-style sidebar navigation (Download, Profiles, Settings), centered action buttons, and responsive drawer handling.
- **Theme-Adaptive Icon System**: Integrated high-contrast navigation icons with automatic brightness/inversion filtering across light and dark themes.

### Documentation
- **OmniDisc Subsystem & Repository Alignment**: Added multi-language documentation for OmniDisc MLS messaging and updated upstream repository links.

---

## [0.8.13] - 2026-08-29

### Added
- **OmniDisc Subsystem Integration**: Bundled end-to-end encrypted messaging, voice/screen streaming, and guild management (`omnidisc-proto`, `omnidisc-media`, `omnidisc-mls`).
- **Open Omni Social Downloader**: Integrated `/open-omni` unified downloader route with `gallery-dl` / `yt-dlp` backend runners, persistent profile presets, and live task cancellation.

### Refactored & Optimized
- **Platform Architecture Cleanup**: Removed 14,000+ lines of duplicate legacy platform extractors from `src-tauri/src/platforms/`, standardizing downloader implementations under `omniget_core::platforms`.
- **Standard Library Synchronization**: Converted all `once_cell::sync` occurrences across Tauri commands and queues to `std::sync::OnceLock` and `std::sync::LazyLock`.
- **Dependency Pruning**: Removed obsolete `once_cell` and `open` crates from workspace manifests.

### Packaging & Infrastructure
- **Storage Identifier & Migration**: Standardized app ID to `com.igect.omniget` with automatic migration from legacy directory locations.
- **CI / CD Pipeline Updates**: Configured GitHub Actions release workflow and AppImage zsync for the `igect/omniget` repository.

---

## [0.8.12] - 2026-08-23

### Added
- **Open Omni Social Media Downloader (`/open-omni`)**: Dedicated multi-platform downloader interface combining `gallery-dl` and `yt-dlp` backend runners with live progress reporting and instant process cancellation.
- **Open Omni Presets & Concurrency Control**: Persistent JSON presets for domain extractors, authentication cookie injection, and concurrency throttling.
- **Loop Mascot Reactive Controller**: Reactive mascot state controller responding in real-time to active download states (Idle, Amazed, Downloading, Queue, Paused, Error).
- **Offline Release Notes Fallback**: Bundled local changelog fallback in `changelog-store.svelte.ts` ensuring update details load when offline or GitHub API rate-limited.
- **Rust Standard Library Modernization**: Migrated global synchronization primitives to `std::sync::OnceLock` and `std::sync::LazyLock`, eliminating the external `once_cell` crate.
- **Dependency Hygiene**: Dropped redundant `open` crate dependency in favor of Tauri's native opener plugin.

### Packaging & Infrastructure
- **App Storage Migration**: Non-destructive automatic migration moving legacy configuration and plugin folders (`wtf.tonho.omniget`, `omniget`) to `com.igect.omniget`.
- **Identifier Standard**: Standardized bundle identifier to `com.igect.omniget` across Tauri configuration, Flatpak manifests, and CI/CD pipelines.
## [0.8.11] - 2026-08-17

### Refactored & Optimized
- **Platform Architecture Cleanup**: Removed orphaned and unreferenced legacy platform extractor files from `src-tauri/src/platforms/`, consolidating all platform downloader logic solely within `omniget-core::platforms`.
- **Rust Standard Library Modernization**: Migrated all `once_cell::sync::OnceCell` and `Lazy` occurrences to Rust standard library's `std::sync::OnceLock` and `std::sync::LazyLock`, dropping the external `once_cell` crate dependency across the workspace.

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
