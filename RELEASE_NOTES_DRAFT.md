# OmniGet v0.15.0 – draft release notes (branch `fixes`, based on v0.14.0)

Local draft only. Nothing has been tagged, pushed or released. The same notes ship inside the app (About → Changelog), translated into all 12 languages, from `src/lib/changelog/release-notes.ts`.

## Security
- yt-dlp extra options are checked against an allowlist; abbreviated long options are resolved first, so they cannot stand in for a dangerous option such as `--exec`.
- Browser-extension pairing only accepts an exact extension origin.
- Plugins, FFmpeg, aria2 and PDFium downloads are checked against their published SHA-256 digest. An archive without a digest is refused.
- URLs passed to aria2 and gallery-dl are validated.
- `omniget://` links open a confirmation inside the app.
- Direct links that point to local or private network addresses are blocked. This can be turned off in settings.
- Plugin archives are unpacked with `enclosed_name`, so they cannot write outside the plugin folder.
- Send-to-phone only serves video, audio and image files.
- CI: third-party actions in workflows that hold secrets are pinned to commit SHAs. `wingetcreate` is pinned to a fixed release and its SHA-256 is checked.
- Security audit fixes:
  - M1: `omniget://` links never start a download on their own.
  - M2: every download path, not only direct links, refuses local network addresses.
  - M3: a yt-dlp command edited before a retry is validated again.
  - M4: the local bridge token cannot be read through DNS rebinding.
  - M5: the MCP server has its own token and only writes inside the download folder.
  - F1: links in custom yt-dlp arguments are validated, and yt-dlp options end with `--`.
  - F2: Deno, gallery-dl and the upscaler are checked against a SHA-256 digest.
  - F3: HEAD requests and link previews no longer use up a send-to-phone link.
  - F4: IPv4 addresses hidden inside NAT64, 6to4, Teredo and compatible IPv6 are decoded before the local network check.
  - F5: the SignPath and GitHub tokens are kept out of the build steps.
  - F6: the plugin registry is pinned, plugins must come from a listed repository, only OpenSelena plugins install automatically, and plugin downloads are capped at 200 MB.
  - Info: send-to-phone serves the real file behind a link; video tools use absolute inputs and never overwrite silently; KaTeX is lifted to 0.18.10; the FFmpeg checksum no longer fails when BtbN replaces its latest build.

## Fixes
- The download list shows newest first (#96).
- Torrent size counts only the selected files.
- Download errors in toasts and history are translated.
- README release claims now match v0.14.0.
- Building from source on Windows works again (#143): `tauri-plugin-notification` matches the npm package (2.8.0), and `gpu-allocator` uses the same `windows` crate as `wgpu-hal` (0.62.2). `pnpm check:tauri-versions` guards both in CI, and the README explains the MSVC `MAX_PATH` workaround.
- Settings and the home screen are translated into all 12 languages, and French accents are fixed (Téléchargements, Réseau, Chaînes, Avancé…).

## New features
- 9:16 reframe presets: blurred background, or center crop.
- Automatic subtitles (Whisper), which can also be burned into the video.
- Send a downloaded file to a phone by scanning a QR code. The link only works on the local network, works once, and expires after 10 minutes.
- Snapchat: Spotlight videos, highlights and public profile stories.
- File actions for audio and image downloads.
- TikTok photo posts with music also get a slideshow video. This can be turned off in settings.
- H.265 video is avoided by default, for compatibility.
- yt-dlp installs itself automatically, retries when the connection drops and waits while offline. A banner appears only if the install fails, with a button that opens Settings → Dependencies.
- DRM-protected media (Widevine, PlayReady, FairPlay in HLS, DASH or Smooth Streaming manifests, and yt-dlp "DRM protected" errors) is detected and explained with a clear, translated message. OmniGet never tries to bypass DRM.
- Windows builds show Corpse as the publisher (installer, executable details and the winget draft). The README, About page, LICENSE and git history keep crediting OpenSelena and the original contributors.

## Dependencies
- Rust: semver updates, plus zip 9 and rust_xlsxwriter 0.102. `cargo audit` went from 37 to 15 findings.
- npm: semver updates, plus marked 18, mermaid 12 and graphviz-anywhere-web 0.2. The deprecated `@types/cytoscape` and `@types/marked` were removed. `pnpm audit` went from 36 to 5 low.
- Frontend toolchain: SvelteKit 3 (configuration moved into `vite.config.js`, `$lib` / `$components` replaced by `#lib` / `#components` subpath imports), Vite 8 (the top-level-await plugin is no longer needed), Vitest 5, and type checking with TypeScript 7 (`svelte-check --tsgo`). TypeScript 6 stays installed because SvelteKit 3 and svelte-check still need it.
- Plugins (Media Converter 1.0.3, Telegram Downloader 1.2.1, Course Downloader 1.4.1) are already on their latest releases.

## Known limitations
- The remaining Rust advisories need major upgrades: livekit/webrtc-sys (which needs clang 21), openmls, librqbit 9 and p256.
- `flatpak/cargo-sources.json` was already out of date in v0.14.0 and needs regenerating before a Flatpak build.
- Snapchat support depends on the structure of Snapchat's public pages.
- The winget package identifier and URLs still say OpenSelena, and the plugin registry pin must be moved on each plugin release.

---

## Résumé (français)
Cette branche corrige plusieurs failles de sécurité : options yt-dlp, appairage de l'extension, vérification SHA-256 des téléchargements, liens omniget://, adresses du réseau local, et actions CI épinglées. Elle corrige aussi des bugs : ordre de la liste, taille des torrents, erreurs traduites. Elle ajoute des fonctions : format vertical 9:16, sous-titres automatiques, envoi vers le téléphone par QR code, Snapchat, actions sur les fichiers audio et image, diaporama TikTok, et H.265 évité. Elle installe yt-dlp automatiquement, détecte les contenus protégés par DRM (sans jamais les contourner), affiche Corpse comme éditeur sous Windows, répare la compilation Windows (#143) et traduit les paramètres dans les 12 langues. Enfin, elle met à jour les dépendances, dont SvelteKit 3, Vite 8, Vitest 5 et TypeScript 7, et passe la version à 0.15.0. Rien n'a été publié.

## TODO before the first fork release: updater

The in-app updater still points at the upstream project
(`OpenSelena/omniget` latest.json, upstream minisign public key in
`src-tauri/tauri.conf.json`). The fork's repository and signing key do not
exist yet, so nothing was changed by default and no key was generated.

When they exist:

1. Generate the key pair once, offline: `pnpm tauri signer generate`.
   Store the private key and password as the `TAURI_SIGNING_PRIVATE_KEY`
   and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets.
2. Set the repository variables `OMNIGET_UPDATER_ENDPOINT` (https URL of
   the fork's `latest.json`) and `OMNIGET_UPDATER_PUBKEY` (the public key).
   The release build applies them through `scripts/updater-config.mjs`.
3. Update the AppImage zsync update string (`LDAI_UPDATE_INFORMATION` in
   `.github/workflows/release.yml`), which also names `OpenSelena`.
4. Users on an upstream build will not receive fork updates: their app
   trusts only the upstream key.
