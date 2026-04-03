use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use omniget_plugin_sdk::{OmnigetPlugin, PluginHost, PluginManifest, InstalledPlugin, ABI_VERSION};
use tracing;

pub struct LoadedPlugin {
    _lib: Option<libloading::Library>,
    pub plugin: Box<dyn OmnigetPlugin>,
    pub manifest: PluginManifest,
}

unsafe impl Send for LoadedPlugin {}
unsafe impl Sync for LoadedPlugin {}

pub struct PluginManager {
    plugins_dir: PathBuf,
    loaded: HashMap<String, LoadedPlugin>,
    installed: Vec<InstalledPlugin>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        let installed = load_installed_list(&plugins_dir);
        Self {
            plugins_dir,
            loaded: HashMap::new(),
            installed,
        }
    }

    pub fn load_all(&mut self, host: Arc<dyn PluginHost>) {
        let enabled: Vec<_> = self
            .installed
            .iter()
            .filter(|p| p.enabled)
            .cloned()
            .collect();

        for entry in &enabled {
            let plugin_dir = self.plugins_dir.join(&entry.id);
            match load_single_plugin(&plugin_dir, host.clone()) {
                Ok(loaded) => {
                    tracing::info!("Loaded plugin: {} v{}", loaded.manifest.id, loaded.manifest.version);
                    self.loaded.insert(entry.id.clone(), loaded);
                }
                Err(e) => {
                    tracing::warn!("Plugin {} not loaded: {e}", entry.id);
                }
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&LoadedPlugin> {
        self.loaded.get(id)
    }

    pub fn loaded_plugins(&self) -> Vec<&LoadedPlugin> {
        self.loaded.values().collect()
    }

    pub fn installed_plugins(&self) -> &[InstalledPlugin] {
        &self.installed
    }

    pub fn loaded_manifests(&self) -> Vec<&PluginManifest> {
        self.loaded.values().map(|p| &p.manifest).collect()
    }

    pub async fn handle_command(
        &self,
        plugin_id: &str,
        command: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let loaded = self
            .loaded
            .get(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not loaded", plugin_id))?;

        loaded.plugin.handle_command(command.to_string(), args).await
    }

    pub fn save_installed(&self) -> anyhow::Result<()> {
        save_installed_list(&self.plugins_dir, &self.installed)
    }

    pub fn register_installed(&mut self, entry: InstalledPlugin) -> anyhow::Result<()> {
        self.installed.retain(|p| p.id != entry.id);
        self.installed.push(entry);
        self.save_installed()
    }

    pub fn unregister(&mut self, plugin_id: &str) -> anyhow::Result<()> {
        if let Some(mut loaded) = self.loaded.remove(plugin_id) {
            loaded.plugin.shutdown();
            let _leaked_lib = loaded._lib.take();
            std::mem::forget(_leaked_lib);
        }
        self.installed.retain(|p| p.id != plugin_id);
        self.save_installed()?;

        let plugin_dir = self.plugins_dir.join(plugin_id);
        if plugin_dir.exists() {
            fs::remove_dir_all(&plugin_dir)?;
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> anyhow::Result<()> {
        if let Some(entry) = self.installed.iter_mut().find(|p| p.id == plugin_id) {
            entry.enabled = enabled;
        }
        self.save_installed()
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }
}

fn load_single_plugin(
    plugin_dir: &Path,
    host: Arc<dyn PluginHost>,
) -> anyhow::Result<LoadedPlugin> {
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest_str = fs::read_to_string(&manifest_path)
        .map_err(|e| anyhow::anyhow!("Cannot read plugin.json: {e}"))?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_str)
        .map_err(|e| anyhow::anyhow!("Invalid plugin.json: {e}"))?;

    let lib_path = find_native_lib(plugin_dir)
        .ok_or_else(|| anyhow::anyhow!("No native library found in {}", plugin_dir.display()))?;

    let lib = unsafe { libloading::Library::new(&lib_path) }
        .map_err(|e| anyhow::anyhow!("Failed to load {}: {e}", lib_path.display()))?;

    let abi_fn: libloading::Symbol<extern "C" fn() -> u32> =
        unsafe { lib.get(b"omniget_plugin_abi_version") }
            .map_err(|_| anyhow::anyhow!("Missing omniget_plugin_abi_version symbol"))?;

    let plugin_abi = abi_fn();
    if plugin_abi != ABI_VERSION {
        anyhow::bail!(
            "ABI mismatch: plugin has v{}, core expects v{}",
            plugin_abi,
            ABI_VERSION
        );
    }

    let init_fn: libloading::Symbol<extern "C" fn() -> *mut dyn OmnigetPlugin> =
        unsafe { lib.get(b"omniget_plugin_init") }
            .map_err(|_| anyhow::anyhow!("Missing omniget_plugin_init symbol"))?;

    let mut plugin = unsafe { Box::from_raw(init_fn()) };
    plugin.initialize(host)?;

    Ok(LoadedPlugin {
        _lib: Some(lib),
        plugin,
        manifest,
    })
}

fn find_native_lib(dir: &Path) -> Option<PathBuf> {
    let extensions = if cfg!(target_os = "windows") {
        &["dll"][..]
    } else if cfg!(target_os = "macos") {
        &["dylib"][..]
    } else {
        &["so"][..]
    };

    for entry in fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if let Some(ext) = path.extension() {
            if extensions.contains(&ext.to_str().unwrap_or("")) {
                return Some(path);
            }
        }
    }
    None
}

fn load_installed_list(plugins_dir: &Path) -> Vec<InstalledPlugin> {
    let path = plugins_dir.join("installed.json");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);

    #[derive(serde::Deserialize)]
    struct InstalledFile {
        plugins: Vec<InstalledPlugin>,
    }

    serde_json::from_str::<InstalledFile>(content)
        .map(|f| f.plugins)
        .unwrap_or_default()
}

fn save_installed_list(plugins_dir: &Path, plugins: &[InstalledPlugin]) -> anyhow::Result<()> {
    fs::create_dir_all(plugins_dir)?;
    let path = plugins_dir.join("installed.json");

    #[derive(serde::Serialize)]
    struct InstalledFile<'a> {
        plugins: &'a [InstalledPlugin],
    }

    let content = serde_json::to_string_pretty(&InstalledFile { plugins })?;
    fs::write(&path, content)?;
    Ok(())
}
