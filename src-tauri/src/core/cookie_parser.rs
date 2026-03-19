use std::collections::HashMap;

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
            for key in &["access_token", "token", "idToken", "bearerToken", "bearer_token"] {
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
                    if lower.contains("token") || lower.contains("auth") || lower.contains("session") || lower.contains("sid") {
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
