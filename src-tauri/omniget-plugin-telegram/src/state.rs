use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::platforms::telegram::auth::{TelegramSessionHandle, TelegramState};

pub struct TelegramPluginState {
    pub telegram_session: TelegramSessionHandle,
    pub active_generic_downloads: Arc<tokio::sync::Mutex<HashMap<u64, (String, CancellationToken)>>>,
}

impl Default for TelegramPluginState {
    fn default() -> Self {
        Self {
            telegram_session: Arc::new(tokio::sync::Mutex::new(TelegramState::new())),
            active_generic_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
}
