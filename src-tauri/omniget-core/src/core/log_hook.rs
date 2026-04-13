use std::sync::{Arc, OnceLock};

pub type LogSink = Arc<dyn Fn(u64, &str) + Send + Sync + 'static>;

static SINK: OnceLock<LogSink> = OnceLock::new();

pub fn set_log_sink(sink: LogSink) {
    let _ = SINK.set(sink);
}

pub fn emit_log(id: u64, line: &str) {
    if let Some(s) = SINK.get() {
        s(id, line);
    }
}

tokio::task_local! {
    pub static CURRENT_DOWNLOAD_ID: u64;
}

pub fn current_download_id() -> Option<u64> {
    CURRENT_DOWNLOAD_ID.try_with(|v| *v).ok()
}
