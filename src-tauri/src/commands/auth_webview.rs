use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthWebviewRequest {
    pub url: String,
    pub title: String,
    pub cookie_domains: Vec<String>,
    pub success_url_contains: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthWebviewResult {
    pub cookies: Vec<AuthCookie>,
    pub final_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

#[tauri::command]
pub async fn open_auth_webview(
    app: AppHandle,
    request: AuthWebviewRequest,
) -> Result<AuthWebviewResult, String> {
    let label = format!(
        "auth-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let width = request.width.unwrap_or(900.0);
    let height = request.height.unwrap_or(700.0);

    let parsed_url: url::Url = request
        .url
        .parse()
        .map_err(|e| format!("Invalid URL: {}", e))?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(2);

    let success_pattern = request.success_url_contains.clone();
    let tx_nav = tx.clone();

    let webview_window = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::External(parsed_url),
    )
    .title(&request.title)
    .inner_size(width, height)
    .center()
    .on_navigation(move |url| {
        let url_str = url.to_string();
        if let Some(ref pattern) = success_pattern {
            if url_str.contains(pattern) {
                let _ = tx_nav.try_send(url_str);
            }
        }
        true
    })
    .build()
    .map_err(|e| format!("Failed to create auth window: {}", e))?;

    let tx_close = tx.clone();
    drop(tx);

    webview_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = tx_close.try_send("__CLOSE_REQUESTED__".to_string());
        }
    });

    let final_url = rx
        .recv()
        .await
        .ok_or_else(|| "Auth cancelled".to_string())?;

    let default_domain = request.cookie_domains.first().cloned().unwrap_or_default();

    let cookies = extract_cookies(&webview_window, &default_domain).await;

    let _ = webview_window.destroy();

    Ok(AuthWebviewResult { cookies, final_url })
}

async fn extract_cookies(
    window: &tauri::WebviewWindow,
    default_domain: &str,
) -> Vec<AuthCookie> {
    let _ = window.eval("document.title = '__OMNIGET_COOKIES__' + document.cookie");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut cookies = Vec::new();
    if let Ok(title) = window.title() {
        if let Some(cookie_str) = title.strip_prefix("__OMNIGET_COOKIES__") {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some((name, value)) = part.split_once('=') {
                    cookies.push(AuthCookie {
                        name: name.trim().to_string(),
                        value: value.trim().to_string(),
                        domain: default_domain.to_string(),
                        path: "/".to_string(),
                    });
                }
            }
        }
    }
    cookies
}
