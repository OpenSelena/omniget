## Downloads

**Windows**: `.exe` installer or portable.
**macOS**: `.dmg` (Apple Silicon `aarch64` or Intel `x86_64`).
**Linux**: `.AppImage` (universal) or `.deb` (Debian/Ubuntu).

---

### Debian/Ubuntu users: AppImage compatibility

**Debian 12+ ships without FUSE 2 by default**, which AppImage requires.
If `./omniget.AppImage` fails with a libfuse error, run:

```bash
sudo apt install libfuse2
```

...or use `./omniget.AppImage --appimage-extract-and-run`.

**Easier alternative**: download the `.deb` instead, which works out of the
box on Debian 12+ and Ubuntu 22.04+.

---

### macOS users: "OmniGet is damaged and can't be opened"

The app is not notarized with Apple yet, so Gatekeeper quarantines it.
After moving the app to `/Applications`, run:

```bash
xattr -cr /Applications/omniget.app
```

Then open it normally. If it still refuses, allow it under
**System Settings → Privacy & Security**.

---

### Code signing policy

Windows installers are signed through [SignPath.io](https://about.signpath.io) (certificate by [SignPath Foundation](https://signpath.org)) only when the `SIGNPATH_API_TOKEN` secret is configured; otherwise this release is unsigned. Code changes and releases are reviewed and approved by the OpenSelena maintainers. See our [Privacy Policy](https://omniget.nav.bd/docs/#privacy).
