//! Local forward proxy that keeps yt-dlp (and the ffmpeg it starts) on the
//! public internet.
//!
//! The entry check in `net_guard` only sees the first URL. yt-dlp then follows
//! redirects and resolves names on its own, so `https://evil.example/r` → `302`
//! → `http://192.168.1.1/…`, or a name that re-resolves to a private address
//! (DNS rebinding), would still reach the local network. Every yt-dlp process
//! is therefore pointed at this proxy through the standard proxy environment
//! variables. The proxy resolves each destination itself, refuses non-public
//! addresses and connects to the address it checked.
//!
//! The proxy listens on 127.0.0.1 only and requires a per-session token, so
//! other local programs cannot use it. When the user enabled "Allow local
//! network downloads", or configured their own proxy (which yt-dlp prefers
//! over the environment), the guard steps aside.

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, bail};
use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::core::net_guard::{
    is_local_host_name, is_non_public_ip, private_network_allowed, BLOCKED_MESSAGE,
};

const PROXY_USER: &str = "omniget";
const MAX_HEAD: usize = 32 * 1024;
const HEAD_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

struct Proxy {
    port: u16,
    token: String,
}

static PROXY: OnceLock<Option<Proxy>> = OnceLock::new();
static ALLOWED_DESTINATIONS: OnceLock<Mutex<HashSet<(String, u16)>>> = OnceLock::new();

fn allowed_destinations() -> &'static Mutex<HashSet<(String, u16)>> {
    ALLOWED_DESTINATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Lets yt-dlp reach one local service OmniGet configured itself, such as the
/// PO token provider on `127.0.0.1:4416`.
pub fn allow_destination(host: &str, port: u16) {
    if let Ok(mut set) = allowed_destinations().lock() {
        set.insert((normalize_host(host), port));
    }
}

fn normalize_host(host: &str) -> String {
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn proxy() -> Option<&'static Proxy> {
    PROXY.get_or_init(start).as_ref()
}

fn start() -> Option<Proxy> {
    let listener = match std::net::TcpListener::bind(("127.0.0.1", 0)) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("[egress-proxy] could not bind: {e}");
            return None;
        }
    };
    let port = listener.local_addr().ok()?.port();
    listener.set_nonblocking(true).ok()?;
    let token = uuid::Uuid::new_v4().simple().to_string();
    let expected = format!("{PROXY_USER}:{token}");
    let spawned = std::thread::Builder::new()
        .name("omniget-egress-proxy".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::warn!("[egress-proxy] runtime failed: {e}");
                    return;
                }
            };
            runtime.block_on(async move {
                let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                    return;
                };
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        continue;
                    };
                    let expected = expected.clone();
                    tokio::spawn(async move {
                        let _ = handle(stream, &expected).await;
                    });
                }
            });
        });
    if let Err(e) = spawned {
        tracing::warn!("[egress-proxy] thread failed: {e}");
        return None;
    }
    Some(Proxy { port, token })
}

/// Proxy URL (with its token) that child processes should use, or `None`
/// when the guard is off.
pub fn proxy_url() -> Option<String> {
    if private_network_allowed() {
        return None;
    }
    let p = proxy()?;
    Some(format!(
        "http://{PROXY_USER}:{}@127.0.0.1:{}",
        p.token, p.port
    ))
}

/// True when yt-dlp and ffmpeg are routed through the guard: the private
/// network is not allowed and no user proxy takes over. While it is on, a
/// `--proxy` in user flags must be refused, since it would bypass the guard.
pub fn guard_enabled() -> bool {
    !private_network_allowed() && !user_proxy_in_environment()
}

/// A proxy the user set in the environment (corporate networks) wins: yt-dlp
/// must keep using it, and that proxy is outside this computer anyway.
fn user_proxy_in_environment() -> bool {
    [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .iter()
    .any(|k| std::env::var(k).is_ok_and(|v| !v.trim().is_empty()))
}

fn proxy_env() -> Vec<(&'static str, String)> {
    if user_proxy_in_environment() {
        return Vec::new();
    }
    match proxy_url() {
        Some(url) => ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"]
            .into_iter()
            .map(|k| (k, url.clone()))
            .collect(),
        None => Vec::new(),
    }
}

const NO_PROXY_VARS: &[&str] = &["NO_PROXY", "no_proxy", "ALL_PROXY", "all_proxy"];

/// Routes a child process (yt-dlp, ffmpeg) through the guard.
pub fn apply_to_command(cmd: &mut tokio::process::Command) {
    let env = proxy_env();
    if env.is_empty() {
        return;
    }
    for var in NO_PROXY_VARS {
        cmd.env_remove(var);
    }
    cmd.envs(env);
}

/// Same as [`apply_to_command`] for a blocking `std::process::Command`.
pub fn apply_to_std_command(cmd: &mut std::process::Command) {
    let env = proxy_env();
    if env.is_empty() {
        return;
    }
    for var in NO_PROXY_VARS {
        cmd.env_remove(var);
    }
    cmd.envs(env);
}

/// Resolves a destination and keeps only addresses the guard allows.
pub(crate) async fn resolve_allowed(host: &str, port: u16) -> anyhow::Result<Vec<SocketAddr>> {
    let host = normalize_host(host);
    let trusted = private_network_allowed()
        || allowed_destinations()
            .lock()
            .map(|s| s.contains(&(host.clone(), port)))
            .unwrap_or(false);
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !trusted && is_non_public_ip(ip) {
            bail!(BLOCKED_MESSAGE);
        }
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    if !trusted && is_local_host_name(&host) {
        bail!(BLOCKED_MESSAGE);
    }
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| anyhow!("Could not resolve {host}: {e}"))?
        .filter(|a| trusted || !is_non_public_ip(a.ip()))
        .collect();
    if addrs.is_empty() {
        bail!(BLOCKED_MESSAGE);
    }
    Ok(addrs)
}

async fn connect_any(addrs: &[SocketAddr]) -> anyhow::Result<TcpStream> {
    let mut last = anyhow!("no address");
    for addr in addrs {
        match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(s)) => return Ok(s),
            Ok(Err(e)) => last = e.into(),
            Err(_) => last = anyhow!("connect timeout"),
        }
    }
    Err(last)
}

async fn read_head(stream: &mut TcpStream) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            bail!("closed before the request head");
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let rest = buf.split_off(pos + 4);
            return Ok((buf, rest));
        }
        if buf.len() > MAX_HEAD {
            bail!("request head too large");
        }
    }
}

async fn reply(stream: &mut TcpStream, status: &str, extra: &str, body: &str) {
    let msg = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(msg.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn authorized(headers: &[(String, String)], expected: &str) -> bool {
    headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("proxy-authorization")
            && v.split_once(' ').is_some_and(|(scheme, cred)| {
                scheme.eq_ignore_ascii_case("basic")
                    && base64::engine::general_purpose::STANDARD
                        .decode(cred.trim())
                        .is_ok_and(|d| d == expected.as_bytes())
            })
    })
}

/// Splits `host:port` (with optional IPv6 brackets).
fn split_authority(authority: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        let port = match tail.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None => default_port,
        };
        return Some((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((h, p)) => Some((h.to_string(), p.parse().ok()?)),
        None => Some((authority.to_string(), default_port)),
    }
}

async fn handle(mut client: TcpStream, expected: &str) -> anyhow::Result<()> {
    let (head, rest) = tokio::time::timeout(HEAD_TIMEOUT, read_head(&mut client)).await??;
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let headers: Vec<(String, String)> = lines
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        reply(&mut client, "400 Bad Request", "", "bad request").await;
        return Ok(());
    };
    if !authorized(&headers, expected) {
        reply(
            &mut client,
            "407 Proxy Authentication Required",
            "Proxy-Authenticate: Basic realm=\"omniget\"\r\n",
            "proxy authentication required",
        )
        .await;
        return Ok(());
    }

    if method.eq_ignore_ascii_case("CONNECT") {
        let Some((host, port)) = split_authority(target, 443) else {
            reply(&mut client, "400 Bad Request", "", "bad target").await;
            return Ok(());
        };
        let addrs = match resolve_allowed(&host, port).await {
            Ok(a) => a,
            Err(e) => {
                reply(
                    &mut client,
                    "403 private_network_blocked",
                    "",
                    &e.to_string(),
                )
                .await;
                return Ok(());
            }
        };
        let mut upstream = match connect_any(&addrs).await {
            Ok(s) => s,
            Err(e) => {
                reply(&mut client, "502 Bad Gateway", "", &e.to_string()).await;
                return Ok(());
            }
        };
        client
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await?;
        if !rest.is_empty() {
            upstream.write_all(&rest).await?;
        }
        let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
        return Ok(());
    }

    // Plain HTTP: absolute-form request ("GET http://host/path HTTP/1.1").
    let Ok(url) = url::Url::parse(target) else {
        reply(&mut client, "400 Bad Request", "", "absolute URL expected").await;
        return Ok(());
    };
    if url.scheme() != "http" {
        reply(&mut client, "400 Bad Request", "", "only http is proxied").await;
        return Ok(());
    }
    let Some(host) = url.host_str().map(str::to_string) else {
        reply(&mut client, "400 Bad Request", "", "no host").await;
        return Ok(());
    };
    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = match resolve_allowed(&host, port).await {
        Ok(a) => a,
        Err(e) => {
            reply(
                &mut client,
                "403 private_network_blocked",
                "",
                &e.to_string(),
            )
            .await;
            return Ok(());
        }
    };
    let mut upstream = match connect_any(&addrs).await {
        Ok(s) => s,
        Err(e) => {
            reply(&mut client, "502 Bad Gateway", "", &e.to_string()).await;
            return Ok(());
        }
    };
    let mut path = url.path().to_string();
    if let Some(q) = url.query() {
        path.push('?');
        path.push_str(q);
    }
    let mut out = format!("{method} {path} {version}\r\n");
    for (k, v) in &headers {
        let k_lower = k.to_ascii_lowercase();
        if matches!(
            k_lower.as_str(),
            "proxy-authorization" | "proxy-connection" | "connection" | "keep-alive"
        ) {
            continue;
        }
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    // One request per connection: a reused connection could otherwise carry
    // a request for another host to the address checked for this one.
    out.push_str("Connection: close\r\n\r\n");
    upstream.write_all(out.as_bytes()).await?;
    if !rest.is_empty() {
        upstream.write_all(&rest).await?;
    }
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn raw_request(request: String) -> String {
        let url = proxy_url().expect("proxy running");
        let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut buf = vec![0u8; 512];
        let n = tokio::time::timeout(Duration::from_secs(10), s.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    }

    fn auth() -> String {
        let url = proxy_url().unwrap();
        let cred = url
            .trim_start_matches("http://")
            .split('@')
            .next()
            .unwrap()
            .to_string();
        format!(
            "Proxy-Authorization: Basic {}\r\n",
            base64::engine::general_purpose::STANDARD.encode(cred)
        )
    }

    #[tokio::test]
    async fn private_destinations_are_refused_through_the_proxy() {
        for target in [
            "192.168.1.1:80",
            "127.0.0.1:22",
            "[::1]:443",
            "localhost:443",
        ] {
            let resp = raw_request(format!("CONNECT {target} HTTP/1.1\r\n{}\r\n", auth())).await;
            assert!(
                resp.starts_with("HTTP/1.1 403 private_network_blocked"),
                "{target}: {resp}"
            );
        }
        let resp = raw_request(format!(
            "GET http://10.0.0.1/admin HTTP/1.1\r\nHost: 10.0.0.1\r\n{}\r\n",
            auth()
        ))
        .await;
        assert!(resp.starts_with("HTTP/1.1 403"), "{resp}");
    }

    #[tokio::test]
    async fn the_proxy_requires_its_token() {
        let resp = raw_request("CONNECT example.com:443 HTTP/1.1\r\n\r\n".to_string()).await;
        assert!(resp.starts_with("HTTP/1.1 407"), "{resp}");
        let resp = raw_request(
            "CONNECT example.com:443 HTTP/1.1\r\nProxy-Authorization: Basic b21uaWdldDp4\r\n\r\n"
                .to_string(),
        )
        .await;
        assert!(resp.starts_with("HTTP/1.1 407"), "{resp}");
    }

    #[tokio::test]
    async fn allowed_local_service_is_reachable_and_plain_http_is_forwarded() {
        let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = server.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let n = s.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let ok = req.starts_with("GET /ping?x=1 HTTP/1.1")
                && !req.to_ascii_lowercase().contains("proxy-authorization");
            let body = if ok { "pong" } else { "bad" };
            let _ = s
                .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n{body}").as_bytes())
                .await;
        });
        allow_destination("127.0.0.1", port);
        let resp = raw_request(format!(
            "GET http://127.0.0.1:{port}/ping?x=1 HTTP/1.1\r\nHost: 127.0.0.1\r\n{}\r\n",
            auth()
        ))
        .await;
        assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
        assert!(resp.ends_with("pong"), "{resp}");
    }

    #[tokio::test]
    async fn connect_tunnels_bytes_to_an_allowed_destination() {
        let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = server.accept().await.unwrap();
            let mut buf = [0u8; 4];
            s.read_exact(&mut buf).await.unwrap();
            s.write_all(&buf).await.unwrap();
        });
        allow_destination("127.0.0.1", port);
        let url = proxy_url().unwrap();
        let proxy_port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
        let mut s = TcpStream::connect(("127.0.0.1", proxy_port)).await.unwrap();
        s.write_all(format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\n{}\r\n", auth()).as_bytes())
            .await
            .unwrap();
        let mut head = vec![0u8; 39];
        s.read_exact(&mut head).await.unwrap();
        assert!(String::from_utf8_lossy(&head).starts_with("HTTP/1.1 200"));
        s.write_all(b"ping").await.unwrap();
        let mut echo = [0u8; 4];
        s.read_exact(&mut echo).await.unwrap();
        assert_eq!(&echo, b"ping");
    }

    #[test]
    fn child_processes_get_the_proxy_environment() {
        let mut cmd = std::process::Command::new("yt-dlp");
        cmd.env("NO_PROXY", "*");
        apply_to_std_command(&mut cmd);
        let envs: Vec<(String, Option<String>)> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().to_string(),
                    v.map(|v| v.to_string_lossy().to_string()),
                )
            })
            .collect();
        let https = envs
            .iter()
            .find(|(k, _)| k == "HTTPS_PROXY")
            .and_then(|(_, v)| v.clone());
        assert_eq!(https, proxy_url());
        assert!(envs.iter().any(|(k, v)| k == "NO_PROXY" && v.is_none()));
    }

    /// End-to-end check with a real yt-dlp on PATH (`cargo test -- --ignored`).
    #[tokio::test]
    #[ignore]
    async fn real_ytdlp_cannot_reach_a_local_server() {
        let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();
        let mut cmd = tokio::process::Command::new("yt-dlp");
        cmd.args(["--ignore-config", "--no-cache-dir", "--simulate"])
            .arg(format!("http://127.0.0.1:{port}/video.mp4"));
        apply_to_command(&mut cmd);
        let out = cmd.output().await.expect("yt-dlp on PATH");
        let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
        assert!(!out.status.success());
        assert!(stderr.contains("private_network_blocked"), "{stderr}");
    }

    /// Public sites still work through the guard (`cargo test -- --ignored`).
    /// Needs real DNS: sandboxes that answer with 198.18.0.0/15 addresses are
    /// refused, as they should be.
    #[tokio::test]
    #[ignore]
    async fn real_ytdlp_reaches_public_sites() {
        let mut cmd = tokio::process::Command::new("yt-dlp");
        cmd.args([
            "--ignore-config",
            "--no-cache-dir",
            "--simulate",
            "--force-generic-extractor",
        ])
        .arg("https://www.w3schools.com/html/mov_bbb.mp4");
        apply_to_command(&mut cmd);
        let out = cmd.output().await.expect("yt-dlp on PATH");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{stderr}");
    }
}
