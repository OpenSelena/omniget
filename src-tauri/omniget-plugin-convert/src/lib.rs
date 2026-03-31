use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};

pub struct ConvertPlugin {
    host: Option<Arc<dyn PluginHost>>,
    active_conversions: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
}

impl ConvertPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            active_conversions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
}

impl OmnigetPlugin for ConvertPlugin {
    fn id(&self) -> &str { "convert" }
    fn name(&self) -> &str { "Media Converter" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>> {
        let host = self.host.clone();
        let active = self.active_conversions.clone();
        Box::pin(async move {
            match command.as_str() {
                "probe_file" => {
                    let path: String = serde_json::from_value(
                        args.get("path").cloned().ok_or("missing 'path'")?
                    ).map_err(|e| e.to_string())?;
                    let info = omniget_core::core::ffmpeg::probe(std::path::Path::new(&path))
                        .await
                        .map_err(|e| e.to_string())?;
                    serde_json::to_value(info).map_err(|e| e.to_string())
                }

                "convert_file" => {
                    let options: omniget_core::core::ffmpeg::ConversionOptions =
                        serde_json::from_value(
                            args.get("options").cloned().ok_or("missing 'options'")?
                        ).map_err(|e| e.to_string())?;

                    let conversion_id = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let cancel_token = CancellationToken::new();
                    active.lock().await.insert(conversion_id, cancel_token.clone());

                    let host = host.clone().ok_or("Plugin not initialized")?;
                    let active = active.clone();

                    tokio::spawn(async move {
                        let (tx, mut rx) = mpsc::channel::<f64>(32);

                        let host_progress = host.clone();
                        let cid = conversion_id;
                        let progress_forwarder = tokio::spawn(async move {
                            while let Some(percent) = rx.recv().await {
                                let _ = host_progress.emit_event(
                                    "convert-progress",
                                    serde_json::json!({ "id": cid, "percent": percent }),
                                );
                            }
                        });

                        let result = omniget_core::core::ffmpeg::convert(&options, cancel_token, tx).await;
                        let _ = progress_forwarder.await;
                        active.lock().await.remove(&conversion_id);

                        match result {
                            Ok(conv_result) => {
                                let _ = host.emit_event("convert-complete", serde_json::json!({
                                    "id": conversion_id,
                                    "success": conv_result.success,
                                    "result": conv_result,
                                    "error": conv_result.error,
                                }));
                            }
                            Err(e) => {
                                let _ = host.emit_event("convert-complete", serde_json::json!({
                                    "id": conversion_id,
                                    "success": false,
                                    "result": null,
                                    "error": e.to_string(),
                                }));
                            }
                        }
                    });

                    serde_json::to_value(conversion_id).map_err(|e| e.to_string())
                }

                "cancel_conversion" => {
                    let conversion_id: u64 = serde_json::from_value(
                        args.get("conversionId").or(args.get("conversion_id")).cloned()
                            .ok_or("missing 'conversionId'")?
                    ).map_err(|e| e.to_string())?;

                    let mut map = active.lock().await;
                    match map.remove(&conversion_id) {
                        Some(token) => {
                            token.cancel();
                            Ok(serde_json::json!("Conversion cancelled"))
                        }
                        None => Err("No active conversion for this ID".to_string()),
                    }
                }

                "get_hwaccel_info" => {
                    let info = omniget_core::core::hwaccel::detect_hwaccel().await;
                    serde_json::to_value(info).map_err(|e| e.to_string())
                }

                _ => Err(format!("Unknown command: {}", command)),
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "probe_file".into(),
            "convert_file".into(),
            "cancel_conversion".into(),
            "get_hwaccel_info".into(),
        ]
    }
}

omniget_plugin_sdk::export_plugin!(ConvertPlugin::new());
