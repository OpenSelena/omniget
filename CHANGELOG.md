# Changelog

All notable changes to the **OmniGet** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
