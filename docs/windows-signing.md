# Windows code signing and winget

Status at v0.14.0: the Windows installer is **not signed** (`osslsigncode verify`
on `OmniGet_0.14.0_x64-setup.exe` reports "No signature found"), and OmniGet is
**not on winget** (`OpenSelena.OmniGet` does not exist in `microsoft/winget-pkgs`;
`OpenSelena.OpenOmni` is a different app, the terminal downloader).

## Signing through SignPath (already wired in `release.yml`)

The release workflow uploads the unsigned NSIS installer and the portable
`omniget.exe` as artifacts, submits them to SignPath and uploads the signed
files to the draft release. Every signing step is skipped when the secret is
missing, and signing failures do not fail the build (`continue-on-error`), so a
release can silently stay unsigned. Check the release log for
"Found SignPath-signed setup installer".

What is still missing (none of it can be done from the code):

1. Apply to the SignPath Foundation open source program and get the
   `omniget` project accepted. The workflow already uses the organization id,
   the project slug `omniget` and the signing policy `release-signing`; they
   must match the SignPath project settings.
2. In SignPath, configure the artifact configuration for an NSIS installer and
   a PE file, and link the GitHub repository as a trusted build system.
3. Add the repository secret `SIGNPATH_API_TOKEN` (a SignPath CI user token).
   Never commit it.
4. Optional: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   are needed to re-sign the updater `.sig` of the signed installer.
5. Release from a tag; verify the published installer with
   `osslsigncode verify -in OmniGet_x.y.z_x64-setup.exe` or the file
   properties dialog on Windows.

Only once a signed release is published should the README say that Windows
releases are signed.

## winget

`packaging/winget/manifests/o/OpenSelena/OmniGet/0.14.0/` holds a manifest for
the published v0.14.0 installer (SHA-256 computed from the release asset).

1. First submission (manual, once): validate with
   `winget validate --manifest packaging/winget/manifests/o/OpenSelena/OmniGet/0.14.0`,
   test with `winget install --manifest ...`, then open a pull request on
   `microsoft/winget-pkgs` with that folder (for example with
   `wingetcreate submit <folder>`).
2. Later releases: `.github/workflows/winget.yml` runs `wingetcreate update
   OpenSelena.OmniGet`, which only works after the first submission is merged.
   It needs the `WINGET_TOKEN` secret.
3. After the package is live, the README can show
   `winget install -e --id OpenSelena.OmniGet` again.
