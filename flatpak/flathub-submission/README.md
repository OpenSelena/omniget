# Flathub Submission Package for OmniGet

This folder contains the self-contained package required to register and publish **OmniGet** on [Flathub](https://github.com/flathub/flathub).

## Package Contents

- `com.openselena.omniget.yml`: The Flathub application manifest targeting GNOME 48 Platform runtime, with offline build sources for `v0.14.0`.
- `cargo-sources.json`: Pinned Rust dependency sources for offline compilation of `src-tauri/Cargo.lock`.
- `node-sources.json`: Pinned npm dependency sources for offline Vite build.
- `package-lock.json`: Pinned lockfile for the npm-cache offline installation.
- `0001-Fix-HAVE_VALGRIND-AM_CONDITIONAL.patch`: Patch for libdbusmenu build.
- `0001-Make-introspection-configurable.patch`: Patch for ayatana-ido build.

## How to Submit to Flathub

1. **Fork the Flathub Repository**:
   Fork `https://github.com/flathub/flathub` to your GitHub account or use `gh`:
   ```bash
   gh repo fork flathub/flathub --clone
   cd flathub
   ```

2. **Create the New App Branch**:
   ```bash
   git checkout -b new-pr/com.openselena.omniget
   ```

3. **Copy the Files into the Root of the Fork**:
   Copy all files from this `flatpak/flathub-submission/` directory into your new branch:
   ```bash
   cp /path/to/omniget/flatpak/flathub-submission/* .
   ```

4. **Commit and Push**:
   ```bash
   git add .
   git commit -m "Add com.openselena.omniget"
   git push origin new-pr/com.openselena.omniget
   ```

5. **Open the Pull Request**:
   ```bash
   gh pr create --title "Add com.openselena.omniget" --body "Submission of OmniGet (desktop multimedia downloader and study assistant)."
   ```

6. **Automated Verification**:
   The Flathub buildbot (`flathubbot`) will automatically trigger test builds for `x86_64` and `aarch64`. Once green, a Flathub maintainer will review and merge the PR, creating the `flathub/com.openselena.omniget` repository.
