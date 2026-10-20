#![allow(clippy::all)]
#![allow(warnings)]

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tauri::command;

#[derive(Debug, Deserialize)]
pub struct HttpRequestOptions {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub cookie_path: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponsePayload {
    pub status: u16,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn parse_netscape_cookies(content: &str, filter_domain: Option<&str>) -> Vec<(String, String)> {
    let mut cookies = Vec::new();
    for raw_line in content.lines() {
        let mut line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("#HttpOnly_") {
            line = stripped;
        } else if line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            let domain = parts[0].to_lowercase();
            let name = parts[5].to_string();
            let value = parts[6].to_string();

            if let Some(filter) = filter_domain {
                let clean_filter = filter.trim_start_matches('.').to_lowercase();
                let clean_domain = domain.trim_start_matches('.').to_lowercase();
                if !clean_domain.ends_with(&clean_filter) && !clean_filter.ends_with(&clean_domain)
                {
                    continue;
                }
            }

            if !name.is_empty() {
                cookies.push((name, value));
            }
        }
    }
    cookies
}

#[command]
pub async fn open_nami_http_fetch(
    options: HttpRequestOptions,
) -> Result<HttpResponsePayload, String> {
    let timeout = Duration::from_millis(options.timeout_ms.unwrap_or(15_000));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let method_str = options
        .method
        .unwrap_or_else(|| "GET".to_string())
        .to_uppercase();
    let method = match method_str.as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let mut request_builder = client.request(method, &options.url);

    let mut header_map = HeaderMap::new();
    if let Some(custom_headers) = options.headers {
        for (k, v) in custom_headers {
            if let (Ok(hn), Ok(hv)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
                header_map.insert(hn, hv);
            }
        }
    }

    if let Some(cookie_path) = options.cookie_path {
        if let Ok(cookie_content) = fs::read_to_string(&cookie_path) {
            let cookies = parse_netscape_cookies(&cookie_content, None);
            if !cookies.is_empty() {
                let formatted = cookies
                    .into_iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("; ");
                if let Ok(hv) = HeaderValue::from_str(&formatted) {
                    header_map.insert(reqwest::header::COOKIE, hv);
                }
            }
        }
    }

    request_builder = request_builder.headers(header_map);

    if let Some(body_text) = options.body {
        request_builder = request_builder.body(body_text);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let status = response.status().as_u16();
    let ok = response.status().is_success();

    let mut response_headers = HashMap::new();
    for (name, value) in response.headers() {
        if let Ok(val_str) = value.to_str() {
            response_headers.insert(name.as_str().to_lowercase(), val_str.to_string());
        }
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(HttpResponsePayload {
        status,
        ok,
        headers: response_headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_netscape_cookies_rust() {
        let raw = ".instagram.com\tTRUE\t/\tTRUE\t1799999999\tsessionid\t12345%3Aabc\n#HttpOnly_.instagram.com\tTRUE\t/\tTRUE\t1799999999\tds_user_id\t98765\n";
        let cookies = parse_netscape_cookies(raw, Some("instagram.com"));
        assert_eq!(cookies.len(), 2);
        assert_eq!(
            cookies[0],
            ("sessionid".to_string(), "12345%3Aabc".to_string())
        );
        assert_eq!(cookies[1], ("ds_user_id".to_string(), "98765".to_string()));
    }
}
