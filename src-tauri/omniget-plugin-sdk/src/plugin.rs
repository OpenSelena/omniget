use std::sync::Arc;

use crate::host::PluginHost;

pub trait OmnigetPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()>;
    fn shutdown(&self) {}

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    >;

    fn commands(&self) -> Vec<String>;
}

#[macro_export]
macro_rules! export_plugin {
    ($constructor:expr) => {
        #[no_mangle]
        pub extern "C" fn omniget_plugin_abi_version() -> u32 {
            $crate::ABI_VERSION
        }

        /// Toolchain handshake (ABI v3+). Returns a NUL-terminated
        /// `sdk=...;rustc=...` fingerprint the host compares against its own
        /// before trusting any non-C-ABI boundary (trait objects, String,
        /// serde_json::Value, boxed futures). Thin `*const c_char` return is
        /// FFI-safe on every rustc, so this call is safe even on a mismatched
        /// toolchain — unlike everything it protects.
        #[no_mangle]
        pub extern "C" fn omniget_plugin_build_info() -> *const ::std::os::raw::c_char {
            $crate::BUILD_INFO.as_ptr() as *const ::std::os::raw::c_char
        }

        #[no_mangle]
        pub extern "C" fn omniget_plugin_init() -> *mut dyn $crate::OmnigetPlugin {
            let plugin = $constructor;
            Box::into_raw(Box::new(plugin))
        }
    };
}
