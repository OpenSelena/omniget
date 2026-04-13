use std::collections::HashMap;
use std::sync::Arc;

pub fn load_extension_cookies_for_domain(domain: &str) -> Option<Arc<reqwest::cookie::Jar>> {
    let cookie_path = crate::core::ytdlp::ext_cookie_path_if_fresh()?;
    let content = std::fs::read_to_string(&cookie_path).ok()?;

    let jar = reqwest::cookie::Jar::default();
    let mut count = 0usize;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 7 {
            continue;
        }

        let cookie_domain = parts[0].trim_start_matches('.');

        if !cookie_domain.contains(domain) && !domain.contains(cookie_domain) {
            continue;
        }

        let name = parts[5];
        let value = parts[6];
        let url_scheme = if parts[3] == "TRUE" { "https" } else { "http" };
        let url_str = format!("{}://{}/", url_scheme, cookie_domain);

        if let Ok(url) = url_str.parse::<reqwest::Url>() {
            let cookie_str = format!("{}={}", name, value);
            jar.add_cookie_str(&cookie_str, &url);
            count += 1;
        }
    }

    if count == 0 {
        return None;
    }

    tracing::debug!(
        "[cookies] loaded {} extension cookies for {}",
        count,
        domain
    );
    Some(Arc::new(jar))
}

pub fn load_extension_cookies_for_url(url: &str) -> Option<Arc<reqwest::cookie::Jar>> {
    let domains = normalize_cookie_domains(url);
    for domain in &domains {
        if let Some(jar) = load_extension_cookies_for_domain(domain) {
            return Some(jar);
        }
    }
    None
}

fn normalize_cookie_domains(url: &str) -> Vec<String> {
    let parsed = match url::Url::parse(url) {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let host = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return vec![],
    };

    let mut domains = vec![];

    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        domains.push(format!(
            "{}.{}",
            parts[parts.len() - 2],
            parts[parts.len() - 1]
        ));
    }

    if host.contains("cdninstagram.com") || host.contains("fbcdn.net") {
        domains.push("instagram.com".to_string());
    }
    if host.contains("twimg.com") {
        domains.push("x.com".to_string());
        domains.push("twitter.com".to_string());
    }
    if host.contains("redd.it") || host.contains("redditstatic.com") {
        domains.push("reddit.com".to_string());
    }
    if host.contains("pstatic.net") || host.contains("pinimg.com") {
        domains.push("pinterest.com".to_string());
    }
    if host.contains("tiktokcdn.com") || host.contains("tiktokv.com") {
        domains.push("tiktok.com".to_string());
    }
    if host.contains("biliapi.net") || host.contains("bilivideo.com") || host.contains("hdslb.com")
    {
        domains.push("bilibili.com".to_string());
    }
    if host.contains("googlevideo.com") || host.contains("ytimg.com") {
        domains.push("youtube.com".to_string());
        domains.push("google.com".to_string());
    }

    domains
}

pub struct ParsedInput {
    pub token: String,
    pub cookie_string: String,
    pub cookies: HashMap<String, String>,
    pub extra_fields: HashMap<String, String>,
}

pub fn parse_cookie_input(input: &str, target_cookie: &str) -> ParsedInput {
    let trimmed = input.trim();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let cookie_array = if let Some(arr) = val.get("cookies").and_then(|c| c.as_array()) {
                arr.clone()
            } else if let Some(arr) = val.as_array() {
                arr.clone()
            } else if val.get("name").is_some() && val.get("value").is_some() {
                vec![val.clone()]
            } else {
                Vec::new()
            };

            if !cookie_array.is_empty() {
                let mut cookies = HashMap::new();
                let mut parts = Vec::new();

                for cookie_obj in &cookie_array {
                    if let (Some(name), Some(value)) = (
                        cookie_obj.get("name").and_then(|n| n.as_str()),
                        cookie_obj.get("value").and_then(|v| v.as_str()),
                    ) {
                        cookies.insert(name.to_string(), value.to_string());
                        parts.push(format!("{}={}", name, value));
                    }
                }

                let cookie_string = parts.join("; ");

                let token = if !target_cookie.is_empty() {
                    if let Some(t) = cookies.get(target_cookie) {
                        t.clone()
                    } else {
                        cookies
                            .values()
                            .find(|v| v.starts_with("eyJ"))
                            .cloned()
                            .unwrap_or_default()
                    }
                } else {
                    cookies
                        .values()
                        .find(|v| v.starts_with("eyJ"))
                        .cloned()
                        .unwrap_or_default()
                };

                return ParsedInput {
                    token,
                    cookie_string,
                    cookies,
                    extra_fields: HashMap::new(),
                };
            }
        }
    }

    if trimmed.contains("; ") || (trimmed.contains('=') && !trimmed.starts_with("eyJ")) {
        let mut cookies = HashMap::new();
        for pair in trimmed.split("; ") {
            if let Some(idx) = pair.find('=') {
                let name = pair[..idx].trim().to_string();
                let value = pair[idx + 1..].trim().to_string();
                cookies.insert(name, value);
            }
        }

        let token = if !target_cookie.is_empty() {
            cookies.get(target_cookie).cloned().unwrap_or_default()
        } else {
            cookies
                .values()
                .find(|v| v.starts_with("eyJ"))
                .cloned()
                .unwrap_or_default()
        };

        return ParsedInput {
            token,
            cookie_string: trimmed.to_string(),
            cookies,
            extra_fields: HashMap::new(),
        };
    }

    let token = trimmed.to_string();
    let cookie_string = if !target_cookie.is_empty() {
        format!("{}={}", target_cookie, token)
    } else {
        String::new()
    };
    let mut cookies = HashMap::new();
    if !target_cookie.is_empty() {
        cookies.insert(target_cookie.to_string(), token.clone());
    }

    ParsedInput {
        token,
        cookie_string,
        cookies,
        extra_fields: HashMap::new(),
    }
}

pub fn parse_bearer_input(input: &str) -> String {
    let trimmed = input.trim();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            for key in &[
                "access_token",
                "token",
                "idToken",
                "bearerToken",
                "bearer_token",
            ] {
                if let Some(t) = val.get(*key).and_then(|v| v.as_str()) {
                    return t.to_string();
                }
            }

            let cookie_array = if let Some(arr) = val.get("cookies").and_then(|c| c.as_array()) {
                arr.clone()
            } else if let Some(arr) = val.as_array() {
                arr.clone()
            } else {
                Vec::new()
            };

            for cookie_obj in &cookie_array {
                if let Some(value) = cookie_obj.get("value").and_then(|v| v.as_str()) {
                    if value.starts_with("eyJ") && value.len() > 50 {
                        return value.to_string();
                    }
                }
            }

            for cookie_obj in &cookie_array {
                if let (Some(name), Some(value)) = (
                    cookie_obj.get("name").and_then(|n| n.as_str()),
                    cookie_obj.get("value").and_then(|v| v.as_str()),
                ) {
                    let lower = name.to_lowercase();
                    if lower.contains("token")
                        || lower.contains("auth")
                        || lower.contains("session")
                        || lower.contains("sid")
                    {
                        if value.len() > 20 {
                            return value.to_string();
                        }
                    }
                }
            }
        }
    }

    trimmed.to_string()
}
