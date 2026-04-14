use std::sync::LazyLock;
use std::sync::RwLock;

use crate::models::settings::ProxySettings;

static GLOBAL_PROXY: LazyLock<RwLock<ProxySettings>> =
    LazyLock::new(|| RwLock::new(ProxySettings::default()));

pub fn init_proxy(proxy: ProxySettings) {
    if let Ok(mut guard) = GLOBAL_PROXY.write() {
        *guard = proxy;
    }
}

pub fn get_proxy_snapshot() -> ProxySettings {
    GLOBAL_PROXY.read().map(|g| g.clone()).unwrap_or_default()
}

pub fn proxy_url() -> Option<String> {
    let proxy = get_proxy_snapshot();
    if !proxy.enabled || proxy.host.is_empty() {
        return None;
    }
    let scheme = match proxy.proxy_type.as_str() {
        "socks5" => "socks5",
        "https" => "https",
        _ => "http",
    };
    if !proxy.username.is_empty() {
        Some(format!(
            "{}://{}:{}@{}:{}",
            scheme, proxy.username, proxy.password, proxy.host, proxy.port
        ))
    } else {
        Some(format!("{}://{}:{}", scheme, proxy.host, proxy.port))
    }
}

pub fn apply_proxy(
    builder: reqwest::ClientBuilder,
    proxy: &ProxySettings,
) -> reqwest::ClientBuilder {
    if !proxy.enabled || proxy.host.is_empty() {
        return builder;
    }
    let scheme = match proxy.proxy_type.as_str() {
        "socks5" => "socks5",
        "https" => "https",
        _ => "http",
    };
    let proxy_url = if !proxy.username.is_empty() {
        format!(
            "{}://{}:{}@{}:{}",
            scheme, proxy.username, proxy.password, proxy.host, proxy.port
        )
    } else {
        format!("{}://{}:{}", scheme, proxy.host, proxy.port)
    };
    match reqwest::Proxy::all(&proxy_url) {
        Ok(p) => builder.proxy(p),
        Err(e) => {
            tracing::warn!("Invalid proxy URL: {}", e);
            builder
        }
    }
}

pub fn apply_global_proxy(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let proxy = get_proxy_snapshot();
    apply_proxy(builder, &proxy)
}

pub fn inject_ua_header(
    headers: &mut reqwest::header::HeaderMap,
    opts_ua: Option<&str>,
) {
    if let Some(ua) = opts_ua {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(ua) {
            headers.insert(reqwest::header::USER_AGENT, v);
        }
    }
}

pub fn ua_header_map(opts_ua: Option<&str>) -> Option<reqwest::header::HeaderMap> {
    let ua = opts_ua?;
    let value = reqwest::header::HeaderValue::from_str(ua).ok()?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, value);
    Some(headers)
}
