//! End-to-end check against a running omnidisc-server, using the same
//! functions the Tauri commands call (no `invoke` layer). Skipped unless
//! `OMNIDISC_TEST_URL` points at a server, e.g.
//! `OMNIDISC_TEST_URL=http://localhost:8080 cargo test --lib omnidisc -- --ignored`.

use super::api::Api;
use super::gateway::{run_gateway, GatewaySink, Status};
use super::{auth, store};
use reqwest::Method;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

enum Ev {
    Dispatch(String, Value),
    Status(Status, Option<String>),
}

struct ChanSink(mpsc::UnboundedSender<Ev>);

impl GatewaySink for ChanSink {
    fn dispatch(&self, _url: &str, t: &str, d: Value) {
        let _ = self.0.send(Ev::Dispatch(t.to_string(), d));
    }

    fn status(&self, _url: &str, status: Status, error: Option<&str>) {
        let _ = self.0.send(Ev::Status(status, error.map(str::to_string)));
    }
}

async fn wait_for<F: FnMut(&Ev) -> bool>(rx: &mut mpsc::UnboundedReceiver<Ev>, mut pred: F, what: &str) -> Ev {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let ev = tokio::time::timeout(remaining, rx.recv())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
            .unwrap_or_else(|| panic!("gateway channel closed waiting for {what}"));
        if pred(&ev) {
            return ev;
        }
    }
}

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{}{}{}", prefix, std::process::id() % 10_000, nanos % 100_000)
}

#[tokio::test]
#[ignore]
async fn register_gateway_ready_send_receive() {
    let Ok(url) = std::env::var("OMNIDISC_TEST_URL") else {
        eprintln!("OMNIDISC_TEST_URL not set; skipping");
        return;
    };
    let base = super::normalize_instance_url(&url).expect("valid url");
    let _session_dir = super::e2e_lock::SessionDirGuard::acquire();

    let alice_name = unique("alice");
    let bob_name = unique("bob");
    let password = "correct horse battery";

    let alice = auth::register(&base, &alice_name, password, Some("Alice"), None)
        .await
        .expect("register alice");
    assert_eq!(alice["username"], alice_name);
    let alice_token = store::load_token(&base).expect("store").expect("alice token stored");

    let bob = auth::register(&base, &bob_name, password, None, None)
        .await
        .expect("register bob");
    let bob_token = store::load_token(&base).expect("store").expect("bob token stored");
    assert_ne!(alice_token, bob_token);

    let alice_api = Api::with_token(base.clone(), alice_token.clone()).expect("api");
    let bob_api = Api::with_token(base.clone(), bob_token.clone()).expect("api");

    let guild: Value = alice_api
        .send(Method::POST, "/api/guilds", &[], Some(json!({ "name": "E2E" })))
        .await
        .expect("create guild");
    let guild_id = guild["id"].as_str().expect("guild id").to_string();
    let general = guild["channels"]
        .as_array()
        .expect("channels")
        .iter()
        .find(|c| c["type"] == 0)
        .expect("text channel")["id"]
        .as_str()
        .expect("channel id")
        .to_string();

    let invite: Value = alice_api
        .send(Method::POST, "/api/invites", &[], Some(json!({ "guild_id": guild_id })))
        .await
        .expect("create invite");
    let code = invite["code"].as_str().expect("code").to_string();
    let joined: Value = bob_api
        .send(Method::POST, &format!("/api/invites/{code}"), &[], Some(json!({})))
        .await
        .expect("join invite");
    assert_eq!(joined["id"], guild_id);

    let fetched: Value = bob_api
        .send(Method::GET, &format!("/api/users/{}", alice["id"].as_str().expect("id")), &[], None)
        .await
        .expect("get user");
    assert_eq!(fetched["display_name"], "Alice");

    let (a_tx, mut a_rx) = mpsc::unbounded_channel();
    let (_a_out_tx, a_out_rx) = mpsc::unbounded_channel::<String>();
    let a_cancel = CancellationToken::new();
    let a_task = tokio::spawn(run_gateway(
        base.clone(),
        alice_token.clone(),
        a_out_rx,
        a_cancel.clone(),
        Arc::new(ChanSink(a_tx)),
    ));
    let ready = wait_for(&mut a_rx, |e| matches!(e, Ev::Dispatch(t, _) if t == "READY"), "alice READY").await;
    let Ev::Dispatch(_, ready_d) = ready else { unreachable!() };
    assert_eq!(ready_d["user"]["username"], alice_name);
    assert_eq!(ready_d["guilds"][0]["id"], guild_id);
    let ready_status = wait_for(&mut a_rx, |e| matches!(e, Ev::Status(Status::Ready, _)), "alice ready status").await;
    assert!(matches!(ready_status, Ev::Status(_, None)));

    let (b_tx, mut b_rx) = mpsc::unbounded_channel();
    let (b_out_tx, b_out_rx) = mpsc::unbounded_channel::<String>();
    let b_cancel = CancellationToken::new();
    let b_task = tokio::spawn(run_gateway(
        base.clone(),
        bob_token.clone(),
        b_out_rx,
        b_cancel.clone(),
        Arc::new(ChanSink(b_tx)),
    ));
    wait_for(&mut b_rx, |e| matches!(e, Ev::Dispatch(t, _) if t == "READY"), "bob READY").await;
    wait_for(&mut b_rx, |e| matches!(e, Ev::Status(Status::Ready, _)), "bob ready status").await;

    b_out_tx
        .send(json!({ "op": 20, "d": { "channel_id": general } }).to_string())
        .expect("typing frame");
    let typing = wait_for(&mut a_rx, |e| matches!(e, Ev::Dispatch(t, _) if t == "TYPING_START"), "typing").await;
    let Ev::Dispatch(_, typing_d) = typing else { unreachable!() };
    assert_eq!(typing_d["user_id"], bob["id"]);

    let sent: Value = bob_api
        .send(
            Method::POST,
            &format!("/api/channels/{general}/messages"),
            &[],
            Some(json!({ "content": "hello from bob", "nonce": "n-1" })),
        )
        .await
        .expect("send message");
    let received = wait_for(
        &mut a_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "MESSAGE_CREATE" && d["content"] == "hello from bob"),
        "alice MESSAGE_CREATE",
    )
    .await;
    let Ev::Dispatch(_, msg) = received else { unreachable!() };
    assert_eq!(msg["id"], sent["id"]);
    assert_eq!(msg["author_id"], bob["id"]);

    let history: Value = alice_api
        .send(Method::GET, &format!("/api/channels/{general}/messages"), &[("limit", "10".into())], None)
        .await
        .expect("history");
    assert!(history.as_array().map(|a| a.iter().any(|m| m["id"] == sent["id"])).unwrap_or(false));

    let bob_id = bob["id"].as_str().expect("bob id").to_string();
    let alice_id = alice["id"].as_str().expect("alice id").to_string();

    bob_api
        .send_empty(
            Method::POST,
            "/api/users/@me/relationships",
            Some(json!({ "username": alice_name })),
        )
        .await
        .expect("friend request");
    let requested = wait_for(
        &mut a_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "RELATIONSHIP_ADD" && d["user_id"] == bob_id.as_str()),
        "alice RELATIONSHIP_ADD",
    )
    .await;
    let Ev::Dispatch(_, rel) = requested else { unreachable!() };
    assert_eq!(rel["kind"], "incoming_request");

    alice_api
        .send_empty(Method::PUT, &format!("/api/users/@me/relationships/{bob_id}"), None)
        .await
        .expect("accept friend");
    wait_for(
        &mut b_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "RELATIONSHIP_ADD" && d["kind"] == "friend"),
        "bob friend accepted",
    )
    .await;

    let relationships: Value = alice_api
        .send(Method::GET, "/api/users/@me/relationships", &[], None)
        .await
        .expect("relationships");
    assert!(relationships
        .as_array()
        .map(|a| a.iter().any(|r| r["user_id"] == bob_id.as_str() && r["kind"] == "friend"))
        .unwrap_or(false));

    let dm: Value = alice_api
        .send(
            Method::POST,
            "/api/users/@me/channels",
            &[],
            Some(json!({ "recipient_ids": [bob_id.clone()] })),
        )
        .await
        .expect("create dm");
    let dm_id = dm["id"].as_str().expect("dm id").to_string();
    let dm_sent: Value = alice_api
        .send(
            Method::POST,
            &format!("/api/channels/{dm_id}/messages"),
            &[],
            Some(json!({ "content": "dm from alice" })),
        )
        .await
        .expect("dm message");
    wait_for(
        &mut b_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "MESSAGE_CREATE" && d["id"] == dm_sent["id"]),
        "bob DM MESSAGE_CREATE",
    )
    .await;

    let found: Value = alice_api
        .send(
            Method::GET,
            &format!("/api/channels/{general}/messages/search"),
            &[("q", "hello".into())],
            None,
        )
        .await
        .expect("channel search");
    assert!(found["messages"]
        .as_array()
        .map(|a| a.iter().any(|m| m["id"] == sent["id"]))
        .unwrap_or(false));
    let guild_found: Value = alice_api
        .send(
            Method::GET,
            &format!("/api/guilds/{guild_id}/messages/search"),
            &[("q", "hello".into()), ("from", bob_id.clone())],
            None,
        )
        .await
        .expect("guild search");
    assert!(guild_found["messages"]
        .as_array()
        .map(|a| a.iter().any(|m| m["id"] == sent["id"]))
        .unwrap_or(false));

    let message_id = sent["id"].as_str().expect("message id").to_string();
    alice_api
        .send_empty(Method::PUT, &format!("/api/channels/{general}/pins/{message_id}"), None)
        .await
        .expect("pin message");
    wait_for(
        &mut a_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "CHANNEL_PINS_UPDATE" && d["channel_id"] == general.as_str()),
        "alice CHANNEL_PINS_UPDATE",
    )
    .await;
    let pins: Value = alice_api
        .send(Method::GET, &format!("/api/channels/{general}/pins"), &[], None)
        .await
        .expect("list pins");
    assert!(pins
        .as_array()
        .map(|a| a.iter().any(|m| m["id"] == message_id.as_str()))
        .unwrap_or(false));

    alice_api
        .send_empty(
            Method::DELETE,
            &format!("/api/guilds/{guild_id}/members/{bob_id}"),
            Some(json!({ "reason": "e2e" })),
        )
        .await
        .expect("kick bob");
    wait_for(
        &mut b_rx,
        |e| matches!(e, Ev::Dispatch(t, d) if t == "GUILD_DELETE" && d["id"] == guild_id.as_str()),
        "bob kicked out of the guild",
    )
    .await;

    let audit: Value = alice_api
        .send(
            Method::GET,
            &format!("/api/guilds/{guild_id}/audit-log"),
            &[("limit", "10".into())],
            None,
        )
        .await
        .expect("audit log");
    assert!(audit
        .as_array()
        .map(|a| a.iter().any(|e| e["action"] == "member.kick" && e["actor_id"] == alice_id.as_str()))
        .unwrap_or(false));

    let sessions: Value = alice_api
        .send(Method::GET, "/api/auth/sessions", &[], None)
        .await
        .expect("sessions");
    assert!(sessions
        .as_array()
        .map(|a| a.iter().any(|s| s["current"] == true))
        .unwrap_or(false));

    a_cancel.cancel();
    b_cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(5), a_task).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), b_task).await;
    wait_for(&mut a_rx, |e| matches!(e, Ev::Status(Status::Disconnected, _)), "alice disconnected").await;

    auth::logout(&base).await.expect("logout");
    assert!(store::load_token(&base).expect("store").is_none());
}
