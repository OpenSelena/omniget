//! Parse cookie payloads from different formats into the canonical
//! `ExtensionCookie` shape used by `extension_storage` and yt-dlp.
//!
//! Two formats are accepted on import:
//! * Netscape (`# Netscape HTTP Cookie File`, tab-separated 7 fields) — the
//!   format yt-dlp itself uses; also produced by Loop and
//!   Get-cookies.txt-LOCALLY.
//! * JSON array — produced by Edit-This-Cookie and recent
//!   Get-cookies.txt-LOCALLY versions.
//!
//! Detection is content-based, not extension-based: the first non-whitespace
//! character drives the choice.

use serde::Deserialize;

use crate::extension_storage::ExtensionCookie;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieFormat {
    Netscape,
    Json,
    Unknown,
}

pub fn detect_format(content: &str) -> CookieFormat {
    let trimmed = content.trim_start();
    if trimmed.is_empty() {
        return CookieFormat::Unknown;
    }
    let first = trimmed.chars().next().unwrap_or(' ');
    if first == '[' || first == '{' {
        return CookieFormat::Json;
    }
    if trimmed.starts_with("# Netscape") || trimmed.starts_with("#HttpOnly_") || trimmed.contains('\t') {
        return CookieFormat::Netscape;
    }
    CookieFormat::Unknown
}

pub fn parse(content: &str) -> anyhow::Result<Vec<ExtensionCookie>> {
    match detect_format(content) {
        CookieFormat::Netscape => parse_netscape(content),
        CookieFormat::Json => parse_json(content),
        CookieFormat::Unknown => {
            anyhow::bail!("Unrecognized cookie format. Accepted: Netscape (yt-dlp), JSON (Edit-This-Cookie, Get-cookies.txt-LOCALLY).")
        }
    }
}

pub fn parse_netscape(content: &str) -> anyhow::Result<Vec<ExtensionCookie>> {
    let mut cookies = Vec::new();
    for raw in content.lines() {
        let trimmed = raw.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let (effective, http_only) = if let Some(rest) = trimmed.strip_prefix("#HttpOnly_") {
            (rest, true)
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            (trimmed, false)
        };
        let parts: Vec<&str> = effective.split('\t').collect();
        if parts.len() < 7 {
            continue;
        }
        let domain = parts[0].to_string();
        let include_subdomains = parts[1].eq_ignore_ascii_case("TRUE");
        let path = parts[2].to_string();
        let secure = parts[3].eq_ignore_ascii_case("TRUE");
        let expires = parts[4].parse::<i64>().unwrap_or(0);
        let name = parts[5].to_string();
        let value = parts[6..].join("\t");
        cookies.push(ExtensionCookie {
            domain,
            http_only,
            path,
            secure,
            expires,
            name,
            value,
            host_only: Some(!include_subdomains),
            same_site: None,
        });
    }
    Ok(cookies)
}

#[derive(Debug, Deserialize)]
struct JsonCookie {
    domain: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default, alias = "httpOnly")]
    http_only: Option<bool>,
    #[serde(default)]
    secure: Option<bool>,
    #[serde(default, alias = "expirationDate", alias = "expires")]
    expiration: Option<f64>,
    name: String,
    value: String,
    #[serde(default, alias = "hostOnly")]
    host_only: Option<bool>,
    #[serde(default, alias = "sameSite")]
    same_site: Option<String>,
    #[serde(default)]
    session: Option<bool>,
}

pub fn parse_json(content: &str) -> anyhow::Result<Vec<ExtensionCookie>> {
    let raw: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
    let arr = match raw {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(_) => vec![raw],
        _ => anyhow::bail!("Expected a JSON array of cookie objects."),
    };
    let mut cookies = Vec::with_capacity(arr.len());
    for item in arr {
        let parsed: JsonCookie = match serde_json::from_value(item) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let expires = match (parsed.session.unwrap_or(false), parsed.expiration) {
            (true, _) => 0,
            (false, Some(f)) => f as i64,
            (false, None) => 0,
        };
        cookies.push(ExtensionCookie {
            domain: parsed.domain,
            http_only: parsed.http_only.unwrap_or(false),
            path: parsed.path.unwrap_or_else(|| "/".to_string()),
            secure: parsed.secure.unwrap_or(false),
            expires,
            name: parsed.name,
            value: parsed.value,
            host_only: parsed.host_only,
            same_site: parsed.same_site,
        });
    }
    Ok(cookies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_netscape_header() {
        assert_eq!(detect_format("# Netscape HTTP Cookie File\n.foo\tTRUE\t/\tTRUE\t0\ta\tb"), CookieFormat::Netscape);
    }

    #[test]
    fn detect_json_array() {
        assert_eq!(detect_format("[{\"domain\":\"x.com\"}]"), CookieFormat::Json);
    }

    #[test]
    fn detect_tab_separated_without_header() {
        assert_eq!(detect_format(".x.com\tTRUE\t/\tTRUE\t0\tname\tvalue"), CookieFormat::Netscape);
    }

    #[test]
    fn detect_unknown_returns_unknown() {
        assert_eq!(detect_format("just some text"), CookieFormat::Unknown);
        assert_eq!(detect_format(""), CookieFormat::Unknown);
    }

    #[test]
    fn parse_netscape_basic() {
        let raw = "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t1830000000\tSID\tabc123\n";
        let cookies = parse_netscape(raw).unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].domain, ".youtube.com");
        assert_eq!(cookies[0].name, "SID");
        assert_eq!(cookies[0].value, "abc123");
        assert_eq!(cookies[0].expires, 1830000000);
        assert!(cookies[0].secure);
    }

    #[test]
    fn parse_netscape_httponly_prefix() {
        let raw = "#HttpOnly_.youtube.com\tTRUE\t/\tTRUE\t1830000000\tHSID\txyz\n";
        let cookies = parse_netscape(raw).unwrap();
        assert_eq!(cookies.len(), 1);
        assert!(cookies[0].http_only);
        assert_eq!(cookies[0].domain, ".youtube.com");
    }

    #[test]
    fn parse_netscape_skips_comments_and_short_lines() {
        let raw = "# header\n.x.com\tTRUE\t/\tTRUE\t0\tk\tv\nbroken line\n";
        let cookies = parse_netscape(raw).unwrap();
        assert_eq!(cookies.len(), 1);
    }

    #[test]
    fn parse_json_edit_this_cookie() {
        let raw = r#"[{"domain":".youtube.com","expirationDate":1830000000.5,"hostOnly":false,"httpOnly":true,"name":"SID","path":"/","sameSite":"no_restriction","secure":true,"session":false,"value":"abc123"}]"#;
        let cookies = parse_json(raw).unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].domain, ".youtube.com");
        assert_eq!(cookies[0].name, "SID");
        assert_eq!(cookies[0].expires, 1830000000);
        assert!(cookies[0].http_only);
        assert!(cookies[0].secure);
    }

    #[test]
    fn parse_json_session_cookie_zeros_expiration() {
        let raw = r#"[{"domain":"x.com","name":"sess","value":"v","session":true,"path":"/","secure":false}]"#;
        let cookies = parse_json(raw).unwrap();
        assert_eq!(cookies[0].expires, 0);
    }

    #[test]
    fn parse_json_skips_malformed_entries_but_keeps_valid() {
        let raw = r#"[{"name":"missing_domain","value":"v"},{"domain":".ok.com","name":"k","value":"v","path":"/"}]"#;
        let cookies = parse_json(raw).unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].domain, ".ok.com");
    }

    #[test]
    fn dispatch_via_parse() {
        assert_eq!(parse("[]").unwrap().len(), 0);
        assert_eq!(parse(".x.com\tTRUE\t/\tTRUE\t0\tk\tv").unwrap().len(), 1);
        assert!(parse("just text").is_err());
    }
}
