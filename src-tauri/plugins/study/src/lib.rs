pub mod commands;
pub mod db;
pub mod scanner;

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
        let db = self.db.clone();
        Box::pin(async move { commands::dispatch(&db, &command, args).await })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "study:courses:list".to_string(),
            "study:courses:get".to_string(),
            "study:courses:scan".to_string(),
            "study:rescan".to_string(),
            "study:courses:toggle_favorite".to_string(),
            "study:courses:recents:touch".to_string(),
            "study:courses:recents:list".to_string(),
            "study:lesson:get".to_string(),
            "study:player:time_changed".to_string(),
            "study:player:seek".to_string(),
            "study:player:speed:get".to_string(),
            "study:player:speed:set".to_string(),
            "study:progress:mark_lesson".to_string(),
            "study:progress:mark_lessons_bulk".to_string(),
            "study:library:telegram:default_output_dir".to_string(),
        ]
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

    #[tokio::test]
    async fn test_study_plugin_course_flow() {
        let db = db::StudyDb::open_in_memory().expect("in-memory db");
        let plugin = StudyPlugin::with_db(db);

        // List initial (empty)
        let list = plugin
            .handle_command("study:courses:list".to_string(), serde_json::json!({}))
            .await
            .expect("courses:list");
        assert_eq!(list, serde_json::json!([]));
    }
}
