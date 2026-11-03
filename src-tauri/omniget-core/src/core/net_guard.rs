//! Keeps direct-link downloads away from this computer and the local network.
//!
//! A pasted link, a browser extension request or an `omniget://` link can point
//! at `http://127.0.0.1:…` or a router admin page. Unless the user enabled
//! "Allow local network downloads", such hosts are refused, including when a
//! public name resolves to a private address or a server redirects there.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, bail};

/// Error text shown when a link is refused. The frontend maps it to a
/// translated message through `errors.private_network_blocked`.
pub const BLOCKED_MESSAGE: &str = "private_network_blocked: this link points to this computer or the local network. Enable \"Allow local network downloads\" in Settings > Downloads to download it.";

static ALLOW_PRIVATE_NETWORK: AtomicBool = AtomicBool::new(false);

pub fn set_allow_private_network(allow: bool) {
    ALLOW_PRIVATE_NETWORK.store(allow, Ordering::Relaxed);
}

pub fn private_network_allowed() -> bool {
    ALLOW_PRIVATE_NETWORK.load(Ordering::Relaxed)
}

/// Loopback, private, link-local, carrier-grade NAT, multicast and other
/// non-public ranges.
pub fn is_non_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_non_public_v4(v4),
        IpAddr::V6(v6) => is_non_public_v6(v6),
    }
}

fn is_non_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240
}

fn is_non_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_non_public_v4(v4);
    }
    let first = ip.segments()[0];
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first == 0x2001 && ip.segments()[1] == 0x0db8)
}

/// Names that always mean "this machine or the local network".
pub fn is_local_host_name(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".home.arpa")
        || host.ends_with(".lan")
        || !host.contains('.')
}

fn host_is_blocked(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Ipv4(ip) => is_non_public_v4(*ip),
        url::Host::Ipv6(ip) => is_non_public_v6(*ip),
        url::Host::Domain(name) => is_local_host_name(name),
    }
}

/// Checks the URL text only (no DNS). Used for redirects and as a first pass.
pub fn check_url_without_dns(raw: &str) -> anyhow::Result<()> {
    if private_network_allowed() {
        return Ok(());
    }
    let url = url::Url::parse(raw).map_err(|e| anyhow!("Invalid URL: {e}"))?;
    let host = url.host().ok_or_else(|| anyhow!("Invalid URL: no host"))?;
    if host_is_blocked(&host) {
        bail!(BLOCKED_MESSAGE);
    }
    Ok(())
}

/// Full check before contacting a direct link: the URL text, then every
/// address its name resolves to.
pub async fn ensure_public_url(raw: &str) -> anyhow::Result<()> {
    check_url_without_dns(raw)?;
    if private_network_allowed() {
        return Ok(());
    }
    let url = url::Url::parse(raw).map_err(|e| anyhow!("Invalid URL: {e}"))?;
    if let Some(url::Host::Domain(name)) = url.host() {
        let port = url.port_or_known_default().unwrap_or(443);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host((name, port))
            .await
            .map_err(|e| anyhow!("Could not resolve {name}: {e}"))?
            .collect();
        if addrs.iter().any(|a| is_non_public_ip(a.ip())) {
            bail!(BLOCKED_MESSAGE);
        }
    }
    Ok(())
}

/// DNS resolver that drops non-public addresses, so a name that changes its
/// answer between the check and the connection (DNS rebinding) still cannot
/// reach the local network.
struct PublicOnlyResolver;

impl reqwest::dns::Resolve for PublicOnlyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            if is_local_host_name(&host) {
                return Err(BLOCKED_MESSAGE.into());
            }
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|a| !is_non_public_ip(a.ip()))
                .collect();
            if addrs.is_empty() {
                return Err(BLOCKED_MESSAGE.into());
            }
            let addrs: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

/// Applies the guard to an HTTP client used for direct links: public-only DNS
/// and a redirect policy that refuses hops to local addresses.
pub fn guard_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    if private_network_allowed() {
        return builder;
    }
    builder
        .dns_resolver(Arc::new(PublicOnlyResolver))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many redirects")
            } else if attempt.url().host().is_some_and(|h| host_is_blocked(&h)) {
                attempt.error(BLOCKED_MESSAGE)
            } else {
                attempt.follow()
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_and_private_addresses_are_refused() {
        for url in [
            "http://127.0.0.1:8080/file.zip",
            "http://localhost/file.zip",
            "http://LOCALHOST./file.zip",
            "http://foo.localhost/file.zip",
            "http://192.168.1.1/backup.bin",
            "http://10.0.0.5/a.iso",
            "http://172.16.3.4/a.iso",
            "http://169.254.169.254/latest/meta-data",
            "http://100.64.0.1/a.zip",
            "http://0.0.0.0/a.zip",
            "http://[::1]/a.zip",
            "http://[fd00::1]/a.zip",
            "http://[fe80::1]/a.zip",
            "http://[::ffff:127.0.0.1]/a.zip",
            "http://nas.local/a.zip",
            "http://router/a.zip",
        ] {
            assert!(
                check_url_without_dns(url).is_err(),
                "{url} should be refused"
            );
        }
    }

    #[test]
    fn public_addresses_are_allowed() {
        for url in [
            "https://example.com/file.zip",
            "https://8.8.8.8/file.zip",
            "https://[2606:4700::1111]/file.zip",
            "https://cdn.example.org:8443/a.bin",
        ] {
            assert!(
                check_url_without_dns(url).is_ok(),
                "{url} should be allowed"
            );
        }
    }

    #[test]
    fn blocked_message_carries_a_stable_code() {
        let err = check_url_without_dns("http://127.0.0.1/a.zip").unwrap_err();
        assert!(err.to_string().starts_with("private_network_blocked:"));
    }
}
