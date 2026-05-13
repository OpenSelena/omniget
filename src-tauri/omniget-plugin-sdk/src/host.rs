use std::path::PathBuf;

pub trait PluginHost: Send + Sync {
    fn emit_event(&self, name: &str, payload: serde_json::Value) -> anyhow::Result<()>;
    fn show_toast(&self, toast_type: &str, message: &str) -> anyhow::Result<()>;
    fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf;
    fn plugin_frontend_dir(&self, plugin_id: &str) -> PathBuf;
    fn get_settings(&self, plugin_id: &str) -> serde_json::Value;
    fn save_settings(&self, plugin_id: &str, settings: serde_json::Value) -> anyhow::Result<()>;
    fn proxy_config(&self) -> Option<ProxyConfig>;
    fn tool_path(&self, tool: &str) -> Option<PathBuf>;
    fn default_output_dir(&self) -> PathBuf;

    /// Returns a writable directory for large, regenerable, versionable caches
    /// that do NOT belong in `plugin_data_dir` (which is for backup-worthy user
    /// data like settings, notes, presets).
    ///
    /// Intended for: CommunityDragon hash tables, downloaded CDN assets, ML model
    /// caches, snapshots of expensive external APIs — anything sized in MB/GB that
    /// the plugin can always re-fetch if deleted.
    ///
    /// Do NOT use for: user settings (use `save_settings`), personal data
    /// (use `plugin_data_dir`), artifacts the user should see in their file
    /// manager (use `default_output_dir`).
    ///
    /// # Parameters
    ///
    /// - `plugin_id`: the caller plugin's id. Host uses this to scope caches
    ///   per-plugin so different plugins never collide.
    /// - `namespace`: an arbitrary plugin-controlled string used to partition
    ///   multiple caches inside the same plugin (e.g. `"lol-hashes"`,
    ///   `"cdragon-assets"`, `"patch-14-23"`). Typical shape is
    ///   `"{domain}-{version}"` so the plugin can purge old versions by
    ///   dropping one subdir.
    ///
    /// # Returns
    ///
    /// The full `PathBuf` to the directory. Unlike `plugin_data_dir` (which
    /// returns a path without creating it), this method guarantees the
    /// directory exists and is writable before returning — callers can write
    /// immediately without a separate `create_dir_all` step.
    ///
    /// # Platform layout
    ///
    /// - Windows: `%LOCALAPPDATA%\wtf.tonho.omniget\external-cache\{plugin_id}\{namespace}\`
    /// - Linux: `$XDG_CACHE_HOME/wtf.tonho.omniget/external-cache/{plugin_id}/{namespace}/` (falls back to `~/.cache/...`)
    /// - macOS: `~/Library/Caches/wtf.tonho.omniget/external-cache/{plugin_id}/{namespace}/`
    ///
    /// These are the OS-canonical cache locations (non-roaming on Windows,
    /// `$XDG_CACHE_HOME` on Linux, `Caches/` on macOS). They are deliberately
    /// separate from `plugin_data_dir`, which lives under the roaming /
    /// data directory and is expected to be backed up.
    ///
    /// # Lifecycle
    ///
    /// The host never cleans or expires these directories automatically. The
    /// plugin owns its own retention policy — bump the namespace string when
    /// the underlying data version changes and purge the old subdir when
    /// appropriate.
    ///
    /// # Capability declaration
    ///
    /// Plugins that use this method should declare `"core:external-data-cache"`
    /// in their `plugin.json` `capabilities` array for marketplace transparency.
    /// Note: capability declarations are currently informational — the host
    /// surfaces them to the user in the marketplace UI but does NOT enforce
    /// them at runtime for any SDK method. A future host revision may
    /// introduce uniform runtime enforcement for all capabilities at once.
    fn external_data_cache(&self, plugin_id: &str, namespace: &str) -> anyhow::Result<PathBuf>;

    /// Returns the on-disk path to a Netscape-format cookies file for the
    /// requested domain, suitable for handing to yt-dlp via `--cookies` or to
    /// any HTTP client that accepts Mozilla cookie jars.
    ///
    /// Returns `None` when:
    /// * the host does not yet support per-domain cookie storage (older host
    ///   versions running plugins compiled against a newer SDK — the default
    ///   impl below covers them by returning `None`),
    /// * the host has no cookies recorded for this domain,
    /// * the file would exist but is unreadable for some reason.
    ///
    /// Plugins should treat `None` as "no cookies for this domain" and fall
    /// back gracefully (skip auth, prompt the user to configure cookies, etc).
    ///
    /// # Parameters
    /// - `domain`: a hostname (`youtube.com`, `music.youtube.com`,
    ///   `soundcloud.com`). The host normalizes to the registrable root
    ///   internally, so `www.youtube.com` and `music.youtube.com` map to the
    ///   same `youtube.com` bucket.
    /// - `account`: optional slug for multi-account setups. `None` resolves to
    ///   the bucket's `_default` account, which is what plugins should use
    ///   unless they expose multi-account UX themselves.
    ///
    /// The default impl returns `None` so plugins built against SDK ≥ this
    /// version still load on hosts built against older SDKs without ABI
    /// breakage. Hosts that implement the method override it.
    fn get_cookie_file(&self, _domain: &str, _account: Option<&str>) -> Option<PathBuf> {
        None
    }

    /// Reports the freshness status of the host-managed cookies for a domain.
    ///
    /// Plugins can use this to surface "your cookies expired, reconnect"
    /// messaging in their UI without parsing the cookie file themselves.
    /// Default impl returns `CookieStatus::Missing` on older hosts.
    fn cookie_status(&self, _domain: &str) -> CookieStatus {
        CookieStatus::Missing
    }
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Freshness state of host-managed cookies for a particular domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieStatus {
    /// No cookies on disk for this domain — plugin should treat as anonymous.
    Missing,
    /// Cookies present, plus the path on disk and recency metadata.
    Available {
        path: PathBuf,
        last_modified_secs: i64,
        cookie_count: usize,
    },
}
