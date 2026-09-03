# Updater Signing Key Rotation for v0.9.1

## Context
In OmniGet v0.9.0, releases were signed in GitHub Actions using the legacy initial signing key (`062A79BB087AAE9E5`), while the repository's `tauri.conf.json` had diverged to an ephemeral public key (`6299D3141BC2DD19`) whose private key was unavailable. This caused in-app updater validation to fail with "The signature was created with a different key than the one provided".

## Decision
Generated a fresh canonical Tauri updater keypair (`9B56A5C3E660B07C`), synchronized GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), updated `src-tauri/tauri.conf.json`'s `pubkey`, and released v0.9.1.

## Consequences
- All releases from v0.9.1 onward are signed consistently in CI and verified cleanly by the desktop updater.
- Clients on intermediate builds prior to v0.9.1 must perform a one-time manual install of the v0.9.1 installer to pick up the new public key.
