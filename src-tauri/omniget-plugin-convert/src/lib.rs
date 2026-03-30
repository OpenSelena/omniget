pub mod commands;
pub mod state;

use state::ConvertState;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("convert")
        .setup(|app, _api| {
            app.manage(ConvertState::default());
            Ok(())
        })
        .build()
}
