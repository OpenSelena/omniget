pub mod db;

use omniget_plugin_sdk::{OmnigetPlugin, PluginHost, PluginManifest};
use std::sync::Arc;

pub struct StudyPlugin {
    host: Option<Arc<dyn PluginHost>>,
    db: db::StudyDb,
}

impl StudyPlugin {
    pub fn new() -> Self {
        let db = db::StudyDb::open_default().unwrap_or_else(|e| {
            tracing::warn!(
                "[study] failed to open default study.db: {}, falling back to in-memory",
                e
            );
            db::StudyDb::open_in_memory().expect("in-memory db must succeed")
        });
        Self { host: None, db }
    }

    pub fn with_db(db: db::StudyDb) -> Self {
        Self { host: None, db }
    }

    pub fn db(&self) -> &db::StudyDb {
        &self.db
    }
}

impl Default for StudyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

pub fn manifest() -> PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid study manifest")
}

impl OmnigetPlugin for StudyPlugin {
    fn id(&self) -> &str {
        "study"
    }

    fn name(&self) -> &str {
        "Study"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        tracing::info!("[study] initialized native Study plugin");
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    > {
        Box::pin(async move {
            match command.as_str() {
                // Catch-all safety dispatcher: prevents UI unhandled exception crashes
                other => {
                    tracing::debug!(
                        "[study] unhandled command fallback: '{}' with args: {:?}",
                        other,
                        args
                    );
                    Ok(serde_json::json!({}))
                }
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_study_plugin_catch_all_dispatcher() {
        let plugin = StudyPlugin::new();
        let res = plugin
            .handle_command("study:unknown:test".to_string(), serde_json::json!({}))
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), serde_json::json!({}));
    }
}
