pub mod commands;
pub mod platforms;
pub mod settings_helper;
pub mod state;

use omniget_plugin_sdk::{OmnigetPlugin, PluginHost};
use state::TelegramPluginState;
use std::sync::Arc;

pub struct TelegramPlugin {
    host: Option<Arc<dyn PluginHost>>,
    state: Arc<TelegramPluginState>,
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self {
            host: None,
            state: Arc::new(TelegramPluginState::default()),
        }
    }
}

pub fn manifest() -> omniget_plugin_sdk::PluginManifest {
    serde_json::from_str(include_str!("../plugin.json")).expect("valid telegram manifest")
}

impl OmnigetPlugin for TelegramPlugin {
    fn id(&self) -> &str {
        "telegram"
    }

    fn name(&self) -> &str {
        "Telegram Downloader"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self, host: Arc<dyn PluginHost>) -> anyhow::Result<()> {
        self.host = Some(host);
        Ok(())
    }

    fn handle_command(
        &self,
        command: String,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    > {
        let host = self.host.clone();
        let state = self.state.clone();
        Box::pin(async move {
            match command.as_str() {
                "telegram_check_session" => {
                    let res = commands::telegram::telegram_check_session(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_qr_start" => {
                    let res = commands::telegram::telegram_qr_start(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_qr_poll" => {
                    let res = commands::telegram::telegram_qr_poll(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_send_code" => {
                    let phone: String = serde_json::from_value(
                        args.get("phone").cloned().ok_or("missing 'phone'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    commands::telegram::telegram_send_code(&state, phone).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_verify_code" => {
                    let code: String =
                        serde_json::from_value(args.get("code").cloned().ok_or("missing 'code'")?)
                            .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_verify_code(&state, code).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_verify_2fa" => {
                    let password: String = serde_json::from_value(
                        args.get("password").cloned().ok_or("missing 'password'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_verify_2fa(&state, password).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_logout" => {
                    commands::telegram::telegram_logout(&state).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_list_chats" => {
                    let res = commands::telegram::telegram_list_chats(&state).await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_list_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args
                        .get("media_type")
                        .or(args.get("mediaType"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let offset: i32 = args
                        .get("offset")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(0);
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let res = commands::telegram::telegram_list_media(
                        &state, chat_id, chat_type, media_type, offset, limit,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_download_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let message_id: i32 = serde_json::from_value(
                        args.get("message_id")
                            .or(args.get("messageId"))
                            .cloned()
                            .ok_or("missing 'message_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let file_name: String = serde_json::from_value(
                        args.get("file_name")
                            .or(args.get("fileName"))
                            .cloned()
                            .ok_or("missing 'file_name'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let output_dir: String = args
                        .get("output_dir")
                        .or(args.get("outputDir"))
                        .or(args.get("save_path"))
                        .or(args.get("savePath"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| {
                            host.as_ref()
                                .map(|h| h.default_output_dir().to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    let res = commands::telegram::telegram_download_media(
                        host.clone(),
                        &state,
                        chat_id,
                        chat_type,
                        message_id,
                        file_name,
                        output_dir,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_download_batch" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_title: String = serde_json::from_value(
                        args.get("chat_title")
                            .or(args.get("chatTitle"))
                            .cloned()
                            .unwrap_or(serde_json::json!("Telegram Download")),
                    )
                    .unwrap_or_else(|_| "Telegram Download".to_string());
                    let items: Vec<commands::telegram::BatchItem> = serde_json::from_value(
                        args.get("items").cloned().ok_or("missing 'items'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let output_dir: String = args
                        .get("output_dir")
                        .or(args.get("outputDir"))
                        .or(args.get("save_dir"))
                        .or(args.get("saveDir"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| {
                            host.as_ref()
                                .map(|h| h.default_output_dir().to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    let res = commands::telegram::telegram_download_batch(
                        host.clone(),
                        &state,
                        chat_id,
                        chat_type,
                        chat_title,
                        items,
                        output_dir,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_cancel_batch" => {
                    let batch_id: u64 = serde_json::from_value(
                        args.get("batch_id")
                            .or(args.get("batchId"))
                            .cloned()
                            .ok_or("missing 'batch_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    commands::telegram::telegram_cancel_batch(&state, batch_id).await?;
                    Ok(serde_json::Value::Null)
                }
                "telegram_get_thumbnail" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .unwrap_or(serde_json::json!("channel")),
                    )
                    .unwrap_or_else(|_| "channel".to_string());
                    let message_id: i32 = serde_json::from_value(
                        args.get("message_id")
                            .or(args.get("messageId"))
                            .cloned()
                            .ok_or("missing 'message_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let res = commands::telegram::telegram_get_thumbnail(
                        &state, chat_id, chat_type, message_id,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_search_media" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .ok_or("missing 'chat_type'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let query: String = serde_json::from_value(
                        args.get("query").cloned().ok_or("missing 'query'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let media_type: Option<String> = args
                        .get("media_type")
                        .or(args.get("mediaType"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let limit: u32 = args
                        .get("limit")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or(50);
                    let res = commands::telegram::telegram_search_media(
                        &state, chat_id, chat_type, query, media_type, limit,
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_get_chat_photo" => {
                    let chat_id: i64 = serde_json::from_value(
                        args.get("chat_id")
                            .or(args.get("chatId"))
                            .cloned()
                            .ok_or("missing 'chat_id'")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let chat_type: String = serde_json::from_value(
                        args.get("chat_type")
                            .or(args.get("chatType"))
                            .cloned()
                            .unwrap_or(serde_json::json!("channel")),
                    )
                    .unwrap_or_else(|_| "channel".to_string());
                    let res =
                        commands::telegram::telegram_get_chat_photo(&state, chat_id, chat_type)
                            .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                "telegram_clear_thumbnail_cache" => {
                    commands::telegram::telegram_clear_thumbnail_cache().await?;
                    Ok(serde_json::Value::Null)
                }
                _ => Err(format!("Unknown command: {}", command)),
            }
        })
    }

    fn commands(&self) -> Vec<String> {
        vec![
            "telegram_check_session".into(),
            "telegram_qr_start".into(),
            "telegram_qr_poll".into(),
            "telegram_send_code".into(),
            "telegram_verify_code".into(),
            "telegram_verify_2fa".into(),
            "telegram_logout".into(),
            "telegram_list_chats".into(),
            "telegram_list_media".into(),
            "telegram_download_media".into(),
            "telegram_download_batch".into(),
            "telegram_cancel_batch".into(),
            "telegram_get_thumbnail".into(),
            "telegram_search_media".into(),
            "telegram_get_chat_photo".into(),
            "telegram_clear_thumbnail_cache".into(),
        ]
    }
}

omniget_plugin_sdk::export_plugin!(TelegramPlugin::new());
