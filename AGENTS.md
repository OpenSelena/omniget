# OmniGet Agent Instructions & Release Protocol

Welcome to OmniGet. This file serves as the definitive reference and operating manual for all AI agents working in this repository.

---

## 1. Project Architecture

- **Desktop Framework**: Tauri v2 (`src-tauri/`) with Rust 1.97.0.
- **Frontend**: SvelteKit / Svelte 5 (`src/`) powered by Vite.
- **Embedded Plugins**: Statically linked core plugins (`courses`, `telegram`, `convert`) in `src-tauri/plugins/`.
- **Public Website & Docs**: GitHub Pages static suite served from repo root:
  - Root landing page: `index.html` and mirror `preview.html`.
  - Legacy landing page: `index.legacy.html`.
  - Documentation suite: `docs/` (`docs/index.html`, `docs/changelog/`, `docs/installation/`, `docs/courses/`, `docs/opennami/`, `docs/mcp/`, `docs/troubleshooting/`).
  - Companion Browser Extension: `browser-extension/chrome/`.

---

## 2. Release Protocol & Mandatory Post-Release SOP

> [!IMPORTANT]
> **NEVER skip writing the release notes, updating the landing page, or updating the documentation when cutting or preparing a release.**
> The in-app changelog modal (`src/lib/stores/changelog-store.svelte.ts`) fetches the GitHub release body directly from `https://api.github.com/repos/OpenSelena/omniget/releases/tags/v<VERSION>`. Leaving release notes blank breaks the in-app user experience.

### Standard Operating Procedure for Every Release:

1. **Version Bump**:
   - Run: `pnpm version:set <major|minor|patch|X.Y.Z>`
   - This automatically updates:
     - `package.json` & `package-lock.json`
     - `src-tauri/Cargo.toml` & `src-tauri/Cargo.lock`
     - `src-tauri/tauri.conf.json`
     - `src/lib/stores/changelog-store.svelte.ts`
     - `index.html` & `preview.html` (download links, version meta tags, CTA text)
     - `index.legacy.html` (download links and release badges)
     - `docs/*/index.html` (sidebar version tags)

2. **Documentation Updates**:
   - **`docs/changelog/index.html`**:
     - Prepend the new release block `<article class="release-box" id="vX-Y-Z">` above the previous release.
     - Demote the previous release tag pill to `<span class="sidebar-tag">`.
     - Update the Table of Contents `<ul class="toc-list" id="tocList">` to list the new version as `vX.Y.Z (Latest Stable)`.
     - Update the search modal description.
   - **`docs/index.html`**:
     - Update the Section 10 Release card to feature the new version.
     - Update the Quickstart release callout and User-Agent examples.

3. **Git Commit & Tag**:
   - Format: `chore(release): bump version to vX.Y.Z, ...`
   - Create annotated/signed tag: `git tag vX.Y.Z`
   - Push: `git push origin main --tags`

4. **GitHub Release Notes (MANDATORY)**:
   - As soon as the GitHub release is published by the workflow (or manually):
   - Compile comprehensive release notes detailing:
     - `## What's New` (categorized by feature area)
     - `## Fixes`
   - Update the release via GitHub CLI:
     ```bash
     gh release edit vX.Y.Z --notes-file <notes.md>
     ```
   - Verify the notes with:
     ```bash
     gh release view vX.Y.Z
     ```

5. **Asset Name & Casing Verification**:
   - Check the exact asset filenames uploaded by Tauri (`gh release view vX.Y.Z --json assets`).
   - Notice casing: e.g. `OmniGet_0.11.0_x64-setup.exe` (capitalized `OmniGet`) vs `omniget_0.11.0_x64-portable.exe` (lowercase `omniget`).
   - Ensure `index.html` and `preview.html` download buttons use the exact asset filenames.

---

## 3. Verification & Quality Gates

Before finishing any task or committing changes:
- `pnpm test` : Runs full Vitest suite (must pass 100%).
- `node scripts/e2e_verify.mjs` : Validates documentation suite routes, links, and accessibility.
