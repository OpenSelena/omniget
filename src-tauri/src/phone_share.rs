//! "Send to phone": serves one finished download to a phone on the same
//! local network through a QR code.
//!
//! Each share starts its own small HTTP server on the computer's private LAN
//! address (never on a public interface) and on a random port. The link holds
//! a random 256-bit token, works once, and stops after 10 minutes. The server
//! shuts itself down when the file has been sent or the link expires.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Serialize;
use tokio::sync::Notify;
use tokio_util::io::ReaderStream;

/// How long a link stays valid.
pub const SHARE_TTL: Duration = Duration::from_secs(10 * 60);

/// Why a request for a shared file was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    UnknownLink,
    Expired,
    AlreadyUsed,
}

/// One single-use link to one file.
pub struct Ticket {
    token: String,
    path: PathBuf,
    expires_at: Instant,
    used: AtomicBool,
    stop: Notify,
}

impl Ticket {
    pub fn new(token: String, path: PathBuf, now: Instant) -> Self {
        Self {
            token,
            path,
            expires_at: now + SHARE_TTL,
            used: AtomicBool::new(false),
            stop: Notify::new(),
        }
    }

    /// Accepts the request at most once, and only before the link expires.
    pub fn claim(&self, token: &str, now: Instant) -> Result<&Path, Refusal> {
        if !constant_time_eq(token.as_bytes(), self.token.as_bytes()) {
            return Err(Refusal::UnknownLink);
        }
        if now >= self.expires_at {
            return Err(Refusal::Expired);
        }
        if self.used.swap(true, Ordering::SeqCst) {
            return Err(Refusal::AlreadyUsed);
        }
        Ok(&self.path)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Only private IPv4 LAN addresses are accepted, so the file is never offered
/// on a public interface.
pub fn is_shareable_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private(),
        IpAddr::V6(_) => false,
    }
}

/// Address of the interface used for the local network. Connecting a UDP
/// socket sends no packet; it only selects the outgoing interface.
fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    is_shareable_ip(ip).then_some(ip)
}

/// `Content-Disposition` value with an ASCII fallback and the exact UTF-8
/// name (RFC 6266 / RFC 5987).
pub fn content_disposition(file_name: &str) -> String {
    let ascii: String = file_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " ._-()[]".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for b in file_name.bytes() {
        if b.is_ascii_alphanumeric() || b"._-".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

async fn serve(State(ticket): State<Arc<Ticket>>, AxumPath(token): AxumPath<String>) -> Response {
    let path = match ticket.claim(&token, Instant::now()) {
        Ok(p) => p.to_path_buf(),
        Err(Refusal::UnknownLink) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::GONE.into_response(),
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => {
            ticket.stop.notify_one();
            return StatusCode::NOT_FOUND.into_response();
        }
    };
    let len = file.metadata().await.map(|m| m.len()).unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());
    // The response keeps streaming during graceful shutdown.
    ticket.stop.notify_one();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, crate::local_bridge::media_mime(&path))
        .header(header::CONTENT_LENGTH, len)
        .header(header::CONTENT_DISPOSITION, content_disposition(&name))
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReaderStream::new(file)))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn router(ticket: Arc<Ticket>) -> Router {
    Router::new()
        .route("/s/{token}", get(serve))
        .with_state(ticket)
}

fn active() -> &'static Mutex<HashMap<String, Arc<Ticket>>> {
    static ACTIVE: OnceLock<Mutex<HashMap<String, Arc<Ticket>>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize)]
pub struct PhoneShare {
    pub id: String,
    pub url: String,
    pub qr_svg: String,
    pub expires_in_secs: u64,
}

fn qr_svg(content: &str) -> Result<String, String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .quiet_zone(true)
        .build())
}

#[tauri::command]
pub async fn phone_share_start(path: String) -> Result<PhoneShare, String> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let ip = lan_ip().ok_or_else(|| "No local network connection found".to_string())?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::new(ip, 0))
        .await
        .map_err(|e| format!("Could not open the sharing port: {e}"))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;

    let token = crate::local_bridge::generate_token();
    let id = crate::local_bridge::generate_token();
    let ticket = Arc::new(Ticket::new(token.clone(), path, Instant::now()));
    let url = format!("http://{addr}/s/{token}");
    let qr = qr_svg(&url)?;

    if let Ok(mut map) = active().lock() {
        map.insert(id.clone(), ticket.clone());
    }
    let router = router(ticket.clone());
    let id_for_task = id.clone();
    tokio::spawn(async move {
        let wait = ticket.clone();
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = wait.stop.notified() => {}
                    _ = tokio::time::sleep(SHARE_TTL) => {}
                }
            })
            .await;
        if let Ok(mut map) = active().lock() {
            map.remove(&id_for_task);
        }
    });

    Ok(PhoneShare {
        id,
        url,
        qr_svg: qr,
        expires_in_secs: SHARE_TTL.as_secs(),
    })
}

/// Closes a link before it is used or expires.
#[tauri::command]
pub async fn phone_share_stop(id: String) -> Result<(), String> {
    let ticket = active().lock().ok().and_then(|mut m| m.remove(&id));
    if let Some(t) = ticket {
        t.used.store(true, Ordering::SeqCst);
        t.stop.notify_one();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(now: Instant) -> Ticket {
        Ticket::new("secret-token".into(), PathBuf::from("/tmp/a.mp4"), now)
    }

    #[test]
    fn link_works_once() {
        let now = Instant::now();
        let t = ticket(now);
        assert!(t.claim("secret-token", now).is_ok());
        assert_eq!(t.claim("secret-token", now), Err(Refusal::AlreadyUsed));
    }

    #[test]
    fn link_expires_after_ten_minutes() {
        let now = Instant::now();
        let t = ticket(now);
        let later = now + Duration::from_secs(10 * 60);
        assert_eq!(t.claim("secret-token", later), Err(Refusal::Expired));
        // An expired request must not burn the link either way.
        assert!(t
            .claim("secret-token", now + Duration::from_secs(599))
            .is_ok());
    }

    #[test]
    fn wrong_token_is_refused_without_using_the_link() {
        let now = Instant::now();
        let t = ticket(now);
        assert_eq!(t.claim("secret-tokeX", now), Err(Refusal::UnknownLink));
        assert_eq!(t.claim("short", now), Err(Refusal::UnknownLink));
        assert!(t.claim("secret-token", now).is_ok());
    }

    async fn get_status(addr: SocketAddr, path: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
        s.write_all(
            format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn server_sends_the_file_once() {
        let dir = std::env::temp_dir().join(format!("phone-share-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("clip.mp4");
        std::fs::write(&file, b"hello").unwrap();
        let ticket = Arc::new(Ticket::new("tok".into(), file, Instant::now()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router(ticket)).await });

        assert!(get_status(addr, "/s/nope")
            .await
            .starts_with("HTTP/1.1 404"));
        let first = get_status(addr, "/s/tok").await;
        assert!(first.starts_with("HTTP/1.1 200"), "{first}");
        assert!(first.ends_with("hello"));
        assert!(get_status(addr, "/s/tok").await.starts_with("HTTP/1.1 410"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_private_ipv4_is_shareable() {
        assert!(is_shareable_ip("192.168.1.20".parse().unwrap()));
        assert!(is_shareable_ip("10.0.0.5".parse().unwrap()));
        assert!(!is_shareable_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_shareable_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_shareable_ip("::1".parse().unwrap()));
    }

    #[test]
    fn content_disposition_encodes_names() {
        let v = content_disposition("Été \"clip\".mp4");
        assert!(v.starts_with("attachment; filename=\""));
        assert!(!v[..v.find(';').unwrap() + 20].contains('É'));
        assert!(
            v.contains("filename*=UTF-8''%C3%89t%C3%A9%20%22clip%22.mp4"),
            "{v}"
        );
        assert!(!v.contains("\"clip\""));
    }
}
