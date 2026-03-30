pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

pub mod models {
    pub use omniget_core::models::*;
}

use state::CoursesState;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("courses")
        .setup(|app, _api| {
            app.manage(CoursesState::default());
            Ok(())
        })
        .build()
}
