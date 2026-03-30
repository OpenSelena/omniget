use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct ConvertState {
    pub active_conversions: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
}

impl Default for ConvertState {
    fn default() -> Self {
        Self {
            active_conversions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
}
