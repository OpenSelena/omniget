# Contributing to OmniGet

Thanks for taking the time to contribute.

## Running the dev build

**Prerequisites:** [Rust](https://rustup.rs/) stable, [Node.js](https://nodejs.org/) 18+, [pnpm](https://pnpm.io/) 10+.

On Linux, install the Tauri system dependencies first:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Then:

```bash
git clone https://github.com/tonhowtf/omniget.git
cd omniget
pnpm install
pnpm tauri dev
```

## Before opening a pull request

Run these locally — CI runs the same checks:

```bash
cd src-tauri
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace

cd ..
pnpm check
```

## Commit style

Follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`. Keep the subject under 72 characters.

## Security issues

Do not file public issues for security problems. See [SECURITY.md](SECURITY.md).

## License

By contributing you agree that your changes are licensed under [GPL-3.0](LICENSE).
