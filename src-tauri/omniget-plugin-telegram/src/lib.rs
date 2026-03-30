pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

use state::TelegramPluginState;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("telegram")
        .setup(|app, _api| {
            app.manage(TelegramPluginState::default());
            Ok(())
        })
        .build()
}
