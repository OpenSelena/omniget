# Plugin Development Guide

Yes — plugin development is supported and contributions are welcome. OmniGet's own Courses, Telegram, and Convert features ship as plugins built on the same SDK described here. This guide documents how the system actually works today, including the parts that are still rough. Where something is unstable or not wired up yet, it says so.

- SDK crate: [`src-tauri/omniget-plugin-sdk`](../src-tauri/omniget-plugin-sdk) (v0.2.0, ABI v2)
- Template: <https://github.com/tonhowtf/omniget-plugin-template>
- Registry: <https://github.com/tonhowtf/omniget-plugins>
- Real-world examples: [omniget-plugin-courses](https://github.com/tonhowtf/omniget-plugin-courses), [omniget-plugin-telegram](https://github.com/tonhowtf/omniget-plugin-telegram), [omniget-plugin-convert](https://github.com/tonhowtf/omniget-plugin-convert)

## 1. Overview

An OmniGet plugin is a **Rust dynamic library** (`cdylib`: `.so` / `.dylib` / `.dll`) plus a `plugin.json` manifest, installed into the user's plugins directory. At startup (and on install/enable, without a restart) the core opens the library with `libloading`, checks the ABI version, calls the exported `omniget_plugin_init` constructor, and hands the plugin an `Arc<dyn PluginHost>` with host services.

What a plugin can provide:

- **Backend commands** — async JSON-in/JSON-out handlers. The frontend calls them through the `plugin_command` Tauri command (there is a `pluginInvoke(pluginId, command, args)` helper in `src/lib/plugin-invoke.ts`).
- **Sidebar navigation items** — declared in `plugin.json` (`nav`), rendered in the app sidebar and command palette with localized labels and a custom SVG icon.
- **Events** — plugins emit Tauri events to the frontend via `PluginHost::emit_event`, plus toasts and per-download log lines.
- **Settings & data** — per-plugin JSON settings and a per-plugin data directory managed by the host.
- **i18n** — per-plugin `i18n/{locale}.json` translation files served by the `get_plugin_i18n` command.
- **Frontend files** — a `frontend/` directory shipped with the plugin (see section 5 for the honest status of this).

The moving parts in the core:

| Piece | File |
| --- | --- |
| SDK: `OmnigetPlugin` trait + `export_plugin!` macro | `src-tauri/omniget-plugin-sdk/src/plugin.rs` |
| SDK: `PluginHost` trait (host services) | `src-tauri/omniget-plugin-sdk/src/host.rs` |
| SDK: manifest / registry types | `src-tauri/omniget-plugin-sdk/src/manifest.rs` |
| Loader (dlopen, ABI check, panic hardening) | `src-tauri/src/plugin_loader.rs` |
| Host implementation | `src-tauri/src/plugin_host.rs` |
| Install / marketplace / hot-load commands | `src-tauri/src/commands/plugins.rs` |

### The plugin contract

A plugin implements the `OmnigetPlugin` trait and exports itself with the `export_plugin!` macro, which generates two `extern "C"` symbols: `omniget_plugin_abi_version()` and `omniget_plugin_init()`.

```rust
use std::sync::Arc;
use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};

pub struct MyPlugin {
    host: Option<Arc<dyn PluginHost>>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl MyPlugin {
    pub fn new() -> Self {
        // Plugins own their async runtime; the host awaits the futures you
        // return but does not provide a tokio context you can rely on.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        Self { host: None, runtime: Arc::new(runtime) }
    }
}

impl OmnigetPlugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn name(&self) -> &str { "My Plugin" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>> {
        let handle = self.runtime.handle().clone();
        Box::pin(async move {
            handle.spawn(async move {
                match command.as_str() {
                    "hello" => Ok(serde_json::json!({ "msg": "hello" })),
                    other => Err(format!("Unknown command: {other}")),
                }
            }).await.map_err(|e| e.to_string())?
        })
    }

    fn commands(&self) -> Vec<String> {
        vec!["hello".into()]
    }
}

omniget_plugin_sdk::export_plugin!(MyPlugin::new());
```

The loader wraps `omniget_plugin_init` and `initialize` in `catch_unwind`, so a panicking plugin becomes a load error instead of crashing the app — but do not lean on that; treat panics in plugin code as bugs.

## 2. Quick start

> **Template status:** the [omniget-plugin-template](https://github.com/tonhowtf/omniget-plugin-template) repo has the right repo layout and a working 4-platform release CI, but its Rust code still shows an older Tauri-plugin style (`Builder::new(...)`, `#[tauri::command]`) and its `Cargo.toml` lacks `crate-type = ["cdylib"]` and the `omniget-plugin-sdk` dependency. Use the template for structure and CI, but wire the crate itself like the snippet above and like the real plugins (e.g. `omniget-plugin-convert`, the smallest one).

### 2.1 Set up the repo

```bash
git clone https://github.com/tonhowtf/omniget-plugin-template.git omniget-plugin-myplugin
cd omniget-plugin-myplugin
```

Rename the plugin everywhere `my-plugin` appears: `plugin.json` (`id`, `name`, `homepage`, `nav[].route`), `Cargo.toml` (`[package] name`), and the source files.

### 2.2 Cargo.toml

```toml
[package]
name = "omniget-plugin-myplugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
omniget-plugin-sdk = { git = "https://github.com/tonhowtf/omniget" }
# Optional: shared download/HTTP/ffmpeg machinery from the core.
# omniget-core = { git = "https://github.com/tonhowtf/omniget", features = ["desktop"] }
```

**The `[patch]` trick for local development.** All official plugins are developed against a sibling clone of the omniget repo, so SDK changes are picked up immediately without pushing to GitHub:

```
~/dev/omniget/                     <- clone of the core repo
~/dev/omniget-plugin-myplugin/     <- your plugin
```

```toml
[patch."https://github.com/tonhowtf/omniget"]
omniget-plugin-sdk = { path = "../omniget/src-tauri/omniget-plugin-sdk" }
omniget-core = { path = "../omniget/src-tauri/omniget-core" }
```

Note that a path `[patch]` is not optional: cargo fails if `../omniget` does not exist. The official plugin repos keep the patch permanently and their release CI checks out `tonhowtf/omniget` as a sibling directory so the paths resolve (see the "Checkout host (for omniget-plugin-sdk path patch)" step in `omniget-plugin-courses`'s `release.yml`). If you prefer building purely against the pinned git dependency, simply omit the `[patch]` section.

### 2.3 Build

```bash
cargo build --release
```

This produces `target/release/libomniget_plugin_myplugin.so` (Linux), `.dylib` (macOS), or `omniget_plugin_myplugin.dll` (Windows).

### 2.4 Install for local testing

Plugins live in `{app-data}/plugins/{plugin-id}/`, where `{app-data}` is (see `omniget-core/src/core/paths.rs`):

- Linux: `~/.local/share/wtf.tonho.omniget/`
- macOS: `~/Library/Application Support/wtf.tonho.omniget/`
- Windows: `%APPDATA%\wtf.tonho.omniget\`
- Override for testing: set the `OMNIGET_DATA_DIR` environment variable before launching OmniGet.

Copy your files there:

```
plugins/my-plugin/
├── plugin.json
├── libomniget_plugin_myplugin.so     <- or .dylib / .dll
├── i18n/                             <- optional
│   └── en.json
└── frontend/                         <- optional
```

The loader (`find_native_lib`) picks the library by the manifest's `rust_crate` field (`{rust_crate}.{ext}` or `lib{rust_crate}.{ext}`), falling back to the first `.so`/`.dylib`/`.dll` it finds in the directory — so a single library file needs no `rust_crate` field.

Finally, register the plugin in `{app-data}/plugins/installed.json` — only plugins listed there (with `"enabled": true`) are loaded at startup:

```json
{
  "plugins": [
    {
      "id": "my-plugin",
      "version": "0.1.0",
      "installed_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z",
      "enabled": true,
      "repo": null,
      "source_release": null
    }
  ]
}
```

Leave `repo` as `null` for local builds, otherwise the launcher's auto-updater will overwrite your build with the latest GitHub release. Restart OmniGet; check the log for `Loaded plugin: my-plugin v0.1.0` or a load error (load errors are also surfaced in the Marketplace UI). There is currently no "install from local zip/file" command — editing `installed.json` is the local-dev path.

### 2.5 Call it from the frontend

```ts
import { pluginInvoke } from "$lib/plugin-invoke";
const res = await pluginInvoke<{ msg: string }>("my-plugin", "hello");
```

## 3. The plugin.json manifest

Parsed into `PluginManifest` (`src-tauri/omniget-plugin-sdk/src/manifest.rs`). Unknown keys are ignored (serde default behavior).

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | yes | Unique plugin id. Must match the install directory name and the registry entry. |
| `name` | string | yes | Display name. |
| `version` | string | yes | Semver-ish version string, shown in the UI. |
| `description` | string | yes | Short description, shown in the plugins list. |
| `author` | string | yes | Author name / GitHub handle. |
| `min_omniget_version` | string | no | Declared minimum core version. **Currently informational — not enforced anywhere in the loader.** |
| `license` | string | no | SPDX id, e.g. `"GPL-3.0"`. |
| `homepage` | string | no | Project URL. |
| `icon` | string | no | Path (relative to the plugin dir) to an icon, e.g. `"assets/icon.png"`. |
| `nav` | array | no | Sidebar entries (below). |
| `events` | object | no | `{ "progress": [..], "complete": [..] }` — names of Tauri events the plugin emits, grouped by role. Informational metadata today. |
| `capabilities` | string[] | no | Capability strings like `"core:events"`, `"core:toast"`, `"core:settings"`, `"core:filesystem"`, `"core:proxy"`, `"core:tools"`, `"core:download-queue"`, `"core:external-data-cache"`. **Informational — shown in the marketplace for transparency, not enforced at runtime** (see the note on `external_data_cache` in `host.rs`). |
| `settings_schema` | JSON | no | Reserved. Not currently read by the host. |
| `rust_crate` | string | no | Base name of the native library file, used by the loader to pick the right file when the plugin dir contains several. |
| `frontend_dir` | string | no | Reserved. The host currently hardcodes `frontend/` regardless of this field. |

Each `nav` item:

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `route` | string | — | App route the sidebar item navigates to, e.g. `"/courses"`. |
| `label` | object | — | Locale → label map, e.g. `{ "en": "Courses", "pt": "Cursos" }`. Falls back to `en`, then to the plugin id. |
| `icon_svg` | string | `null` | SVG **path data only** (the `d` attribute contents), rendered by the sidebar. |
| `group` | `"primary"` \| `"secondary"` | `"secondary"` | Sidebar group. |
| `order` | number | `50` | Sort order among nav items (lower = higher). |

You will see `"routes"` and `"i18n"` keys in the shipped plugins' manifests; those keys are **not** part of `PluginManifest` and are currently ignored by the core.

## 4. The PluginHost API

Your plugin receives `Arc<dyn PluginHost>` in `initialize`. Implementation: `src-tauri/src/plugin_host.rs`.

| Method | What it does |
| --- | --- |
| `emit_event(name, payload)` | Emits a Tauri event to the frontend (any window can `listen(name, ...)`). |
| `show_toast(toast_type, message)` | Shows an app toast (emits the `plugin-toast` event; types match the core toast store, e.g. `"success"`, `"error"`). |
| `plugin_data_dir(plugin_id)` | Path to `plugins/{id}/data/` — backup-worthy user data (not auto-created). |
| `plugin_frontend_dir(plugin_id)` | Path to `plugins/{id}/frontend/`. |
| `get_settings(plugin_id)` | Reads `plugins/{id}/data/settings.json`; returns `{}` if absent. |
| `save_settings(plugin_id, value)` | Writes the settings JSON (creates `data/` as needed). |
| `proxy_config()` | The user's global proxy settings, or `None` if disabled — honor this for your network traffic. |
| `tool_path(tool)` | Absolute path to a managed tool (e.g. `"ffmpeg"`, `"yt-dlp"`): OmniGet's own `bin/` dir first, then `$PATH`. |
| `default_output_dir()` | The user's Downloads directory (falls back to home). |
| `external_data_cache(plugin_id, namespace)` | Creates and returns a per-plugin, per-namespace OS cache directory for large regenerable data (hash tables, CDN assets). Never auto-cleaned; you own retention. Declare `"core:external-data-cache"`. |
| `get_cookie_file(domain, account)` | Netscape-format cookie file for a domain from the host cookie store (for `yt-dlp --cookies` etc.), or `None`. Has a default impl returning `None`, so it works across host versions. |
| `cookie_status(domain)` | `CookieStatus::Missing` or `Available { path, last_modified_secs, cookie_count }` — for "your cookies expired" UX. Default impl returns `Missing`. |
| `emit_download_log(download_id, line)` | Appends a line to the user-visible log of a download in the queue UI. Default impl is a no-op. |

## 5. Frontend integration

Honest status: **there is no dynamic plugin-frontend loading yet.** The pages for the official plugins (`/courses`, `/telegram`, `/convert`) are compiled into the core app under `src/routes/`, and talk to the plugin backends via `pluginInvoke`. The plumbing for shipping UI with the plugin exists — plugins ship a `frontend/` directory (built with Vite in library mode, with core modules like `$lib/plugin-invoke` marked external), the host exposes `plugin_frontend_dir`, and there is a `get_plugin_frontend_path` command — but nothing in the core frontend consumes it today. If your plugin needs its own page, the current path is a PR to the core adding a route under `src/routes/` (this is how all three official plugins work).

What *is* wired up:

- **Sidebar nav**: `nav` entries from loaded, enabled plugins appear in the sidebar and command palette (`src/routes/+layout.svelte` calls `list_plugins`).
- **Hot load (since [#149](https://github.com/tonhowtf/omniget/pull/149))**: installing from the marketplace or enabling a disabled plugin loads the dylib at runtime — no restart. The core emits a `plugins-changed` event and the sidebar re-reads the plugin list. Caveats:
  - **Updating an already-loaded plugin still requires a restart**: the installer writes the new files (renaming in-use files to `*.old` on Windows-style locked files) but does not reload a library that is already loaded.
  - **Unloading is deliberately leaky**: on uninstall the core calls `shutdown()` and then intentionally *leaks* the library instead of `dlclose`-ing it, because plugin threads may still be executing its code. The plugin disappears from the UI immediately; its code stays mapped until restart.
- **Events**: anything you `emit_event` can be received in core pages or shipped frontend code with `listen(...)` from `@tauri-apps/api/event`.
- **Status/errors**: `list_plugins` returns `enabled`, `loaded`, and a structured `load_error` (`kind`: `manifest_read`, `manifest_parse`, `no_native_lib`, `library_load`, `missing_abi_symbol`, `abi_mismatch`, `missing_init_symbol`, `initialize`) that pages use to render "not installed / needs restart / incompatible / load failed" states.

## 6. i18n conventions

- Ship translations as `i18n/{locale}.json` in the plugin directory (`en.json`, `pt.json`, `zh.json`, ...), matching the core's locale codes.
- Structure them as a namespace object keyed by your plugin/feature id, exactly like the core's locale files:

  ```json
  { "myplugin": { "title": "My Plugin", "connected_as": "Connected as {{email}}" } }
  ```

- `{{placeholders}}` follow the core's `sveltekit-i18n` interpolation style; `_one` suffixed keys are used for singular forms (see `courses.course_count_one`).
- The backend command `get_plugin_i18n(plugin_id, locale)` returns the locale file, falling back to `en.json`, then `{}`. Honest status: the core UI does not call it yet — the official plugins' strings are currently duplicated into the core's `src/lib/i18n/*.json`. Ship the `i18n/` dir anyway (the official plugins do) so your plugin is ready when loading is wired up; if your plugin's page lives in the core (see section 5), add its strings to the core locale files in the same PR.
- Nav labels do not use these files — they come from the inline `label` map in `plugin.json`.

## 7. ABI stability — read this before shipping

Plainly: **the plugin API is young and not yet stable.** Here is exactly what is and is not guaranteed:

- The SDK defines `ABI_VERSION` (currently **2**, SDK crate v0.2.0). The loader calls the plugin's exported `omniget_plugin_abi_version()` and refuses to load on mismatch, with a clear error surfaced in the UI (`abi_mismatch`, showing both versions). A plugin missing the symbol entirely (built against a pre-v2 SDK) is also rejected.
- When `ABI_VERSION` bumps, **every plugin must be rebuilt** against the new SDK. There is no compatibility shim.
- Beyond the version handshake, the boundary passes a Rust trait object (`*mut dyn OmnigetPlugin`) and Rust types (`String`, `serde_json::Value`, futures) across the dynamic-library boundary. **Rust does not guarantee ABI stability between compiler versions**, so a plugin and a core built with sufficiently different `rustc` versions can misbehave even when `ABI_VERSION` matches. Build plugins with a current stable toolchain, close to what OmniGet's release CI uses (both the core and the plugin templates build with `dtolnay/rust-toolchain@stable`). If you hit inexplicable crashes at load time, rebuild with the same toolchain as the core release.
- Within an ABI version, the SDK extends the `PluginHost` trait using **default method implementations** (`get_cookie_file`, `cookie_status`, `emit_download_log`), so plugins built against a newer SDK still load on older hosts and vice versa. Trait methods *without* defaults cannot be added without an ABI bump.
- Build against the SDK **from the omniget git repo** (`omniget-plugin-sdk = { git = "https://github.com/tonhowtf/omniget" }`), optionally pinned to a tag/rev matching the OmniGet release you target. The crate is not published to crates.io.
- `min_omniget_version` and `capabilities` are currently informational, not enforced. Do not rely on them as a compatibility mechanism.

Expect breaking changes between minor OmniGet versions while the SDK is pre-1.0. The upside: because official features are themselves plugins, breakage is felt (and fixed) by the core team first.

## 8. Distribution

### Release zips

Distribution is via **GitHub releases** on your plugin repo. The installer (`install_plugin_zip_from_repo` in `src-tauri/src/commands/plugins.rs`) fetches `https://api.github.com/repos/{repo}/releases/latest` and downloads the first asset whose name **contains the platform suffix and ends with `.zip`**:

- `windows-x86_64`, `linux-x86_64`, `macos-aarch64`, `macos-x86_64`

e.g. `omniget-plugin-myplugin-v0.1.0-linux-x86_64.zip`. The release **tag** (minus a leading `v`) becomes the installed version, so tag releases `v0.1.0` style.

The zip is extracted **as-is into `plugins/{id}/`**, so the manifest must be at the zip root:

```
plugin.json
libomniget_plugin_myplugin.so    <- the native lib for that platform
frontend/                        <- optional, prebuilt files only
i18n/
  en.json
assets/
  icon.png
```

The template's `.github/workflows/release.yml` builds all four targets and packages exactly this layout (the courses plugin's workflow additionally copies `frontend/build/*` and `i18n/` — the template's version currently forgets `i18n/`; add that line if you ship translations). After extraction the plugin is registered in `installed.json`, enabled, and hot-loaded.

### Getting listed in the registry

The in-app marketplace reads `plugins.json` from <https://github.com/tonhowtf/omniget-plugins> (with a jsDelivr fallback). To get listed, open a PR adding an entry:

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "description": "One-line description.",
  "author": "your-github-username",
  "repo": "your-github-username/omniget-plugin-myplugin",
  "homepage": "https://github.com/your-github-username/omniget-plugin-myplugin",
  "tags": ["example"],
  "official": false,
  "capabilities": ["core:events", "core:toast"]
}
```

`id` must match your `plugin.json` id, and `repo` must be a GitHub `owner/name` with platform release zips as described above.

Be aware of two behaviors that come with being listed (`ensure_default_plugins` / `auto_update_plugins` in `commands/plugins.rs`):

- **Every registry plugin is auto-installed on launch** for users who don't have it (unless they previously uninstalled it — uninstalls are remembered).
- **Installed registry plugins auto-update on launch** to the latest GitHub release.

So a registry listing is effectively "shipped to all users"; expect review of your plugin before a PR is merged.

## Questions?

Open an issue at <https://github.com/tonhowtf/omniget/issues> — questions about the plugin API are welcome, and they help decide what gets stabilized first.
