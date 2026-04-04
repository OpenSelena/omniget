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
    tracing::info!(
        "[auth_webview] opening: url={}, success_pattern={:?}, domains={:?}",
        request.url,
        request.success_url_contains,
        request.cookie_domains
    );

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

    let login_path = parsed_url.path().to_string();
    let login_host = parsed_url.host_str().unwrap_or("").to_string();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(4);

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
        tracing::info!("[auth_webview] navigation: {}", url_str);

        let mut is_success = false;

        if let Some(ref pattern) = success_pattern {
            if url_str.contains(pattern) {
                tracing::info!("[auth_webview] success pattern matched: {}", pattern);
                is_success = true;
            }
        }

        if !is_success && !login_host.is_empty() {
            if let Ok(nav_url) = url::Url::parse(&url_str) {
                let nav_host = nav_url.host_str().unwrap_or("");
                let nav_path = nav_url.path();
                if nav_host.contains(&login_host) || login_host.contains(nav_host) {
                    if nav_path != login_path
                        && !nav_path.contains("login")
                        && !nav_path.contains("signin")
                        && !nav_path.contains("auth")
                    {
                        tracing::info!(
                            "[auth_webview] redirect away from login detected: {} -> {}",
                            login_path,
                            nav_path
                        );
                        is_success = true;
                    }
                }
            }
        }

        if is_success {
            let _ = tx_nav.try_send(url_str);
        }

        true
    })
    .build()
    .map_err(|e| format!("Failed to create auth window: {}", e))?;

    let tx_close = tx.clone();
    drop(tx);

    let ww_clone = webview_window.clone();
    webview_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            tracing::info!("[auth_webview] close requested by user");
            api.prevent_close();
            let _ = tx_close.try_send("__CLOSE_REQUESTED__".to_string());
        }
    });

    let final_url = tokio::select! {
        msg = rx.recv() => {
            msg.ok_or_else(|| "Auth cancelled".to_string())?
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {
            tracing::warn!("[auth_webview] timed out after 5 minutes");
            let _ = ww_clone.destroy();
            return Err("Auth timed out".to_string());
        }
    };

    tracing::info!("[auth_webview] signal received: {}", if final_url == "__CLOSE_REQUESTED__" { "close" } else { &final_url });

    let default_domain = request.cookie_domains.first().cloned().unwrap_or_default();
    let cookies = extract_cookies(&webview_window, &default_domain).await;

    tracing::info!("[auth_webview] extracted {} cookies", cookies.len());
    for c in &cookies {
        tracing::info!("[auth_webview]   cookie: {}={}", c.name, &c.value[..c.value.len().min(20)]);
    }

    let _ = webview_window.destroy();

    Ok(AuthWebviewResult { cookies, final_url })
}

async fn extract_cookies(
    window: &tauri::WebviewWindow,
    default_domain: &str,
) -> Vec<AuthCookie> {
    let js = r#"
(function() {
    var result = { cookies: document.cookie, storage: {} };
    try {
        for (var i = 0; i < localStorage.length; i++) {
            var key = localStorage.key(i);
            if (/token|auth|access|session|jwt|csrf/i.test(key)) {
                result.storage[key] = localStorage.getItem(key);
            }
        }
    } catch(e) {}
    try {
        for (var i = 0; i < sessionStorage.length; i++) {
            var key = sessionStorage.key(i);
            if (/token|auth|access|session|jwt|csrf/i.test(key)) {
                result.storage['ss:' + key] = sessionStorage.getItem(key);
            }
        }
    } catch(e) {}
    document.title = '__OMNIGET_COOKIES__' + JSON.stringify(result);
})()
"#;

    if let Err(e) = window.eval(js) {
        tracing::error!("[auth_webview] eval failed: {}", e);
    }
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut cookies = Vec::new();
    match window.title() {
        Ok(title) => {
            tracing::info!(
                "[auth_webview] title after eval: {}",
                &title[..title.len().min(200)]
            );
            if let Some(data_str) = title.strip_prefix("__OMNIGET_COOKIES__") {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(data_str) {
                    if let Some(cookie_str) = data["cookies"].as_str() {
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

                    if let Some(storage) = data["storage"].as_object() {
                        for (key, value) in storage {
                            if let Some(val) = value.as_str() {
                                if !val.is_empty() {
                                    cookies.push(AuthCookie {
                                        name: key.clone(),
                                        value: val.to_string(),
                                        domain: default_domain.to_string(),
                                        path: "/".to_string(),
                                    });
                                }
                            }
                        }
                    }
                } else {
                    tracing::warn!("[auth_webview] failed to parse JSON from title, trying plain cookie format");
                    for part in data_str.split(';') {
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
            } else {
                tracing::warn!("[auth_webview] title does not have cookie prefix");
            }
        }
        Err(e) => {
            tracing::error!("[auth_webview] failed to read title: {}", e);
        }
    }
    cookies
}
