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

        #[no_mangle]
        pub extern "C" fn omniget_plugin_init() -> *mut dyn $crate::OmnigetPlugin {
            let plugin = $constructor;
            Box::into_raw(Box::new(plugin))
        }
    };
}
