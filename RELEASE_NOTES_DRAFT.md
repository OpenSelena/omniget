# OmniGet – draft release notes (branch `fixes`, based on v0.14.0)

Local draft only. Nothing has been tagged, pushed or released.

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

## Fixes
- The download list shows newest first (#96).
- Torrent size counts only the selected files.
- Download errors in toasts and history are translated.
- README release claims now match v0.14.0.

## New features
- 9:16 reframe presets: blurred background, or center crop.
- Automatic subtitles (Whisper), which can also be burned into the video.
- Send a downloaded file to a phone by scanning a QR code. The link only works on the local network, works once, and expires after 10 minutes.
- Snapchat: Spotlight videos, highlights and public profile stories.
- File actions for audio and image downloads.
- TikTok photo posts with music also get a slideshow video. This can be turned off in settings.
- H.265 video is avoided by default, for compatibility.

## Dependencies
- Rust: semver updates, plus zip 9 and rust_xlsxwriter 0.102. `cargo audit` went from 37 to 15 findings.
- npm: semver updates, plus marked 18, mermaid 12 and graphviz-anywhere-web 0.2. The deprecated `@types/cytoscape` and `@types/marked` were removed. `pnpm audit` went from 36 to 5 low.
- Plugins (Media Converter 1.0.3, Telegram Downloader 1.2.1, Course Downloader 1.4.1) are already on their latest releases.

## Known limitations
- The remaining Rust advisories need major upgrades: livekit/webrtc-sys (which needs clang 21), openmls, librqbit 9 and p256.
- `flatpak/cargo-sources.json` was already out of date in v0.14.0 and needs regenerating before a Flatpak build.
- Snapchat support depends on the structure of Snapchat's public pages.

---

## Résumé (français)
Cette branche corrige plusieurs failles de sécurité : options yt-dlp, appairage de l'extension, vérification SHA-256 des téléchargements, liens omniget://, adresses du réseau local, et actions CI épinglées. Elle corrige aussi des bugs : ordre de la liste, taille des torrents, erreurs traduites. Elle ajoute des fonctions : format vertical 9:16, sous-titres automatiques, envoi vers le téléphone par QR code, Snapchat, actions sur les fichiers audio et image, diaporama TikTok, et H.265 évité. Enfin, elle met à jour les dépendances. Rien n'a été publié.
