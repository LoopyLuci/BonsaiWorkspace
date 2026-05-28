/// Integration tests: MockBuddyApi + MockPlatform roundtrip and circuit-breaker behaviour.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use axum::{routing::post, Json, Router as AxumRouter};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use bonsai_bot::platforms::{InboundMessage, MessagingPlatform, ShedNotice};

// ── MockPlatform ──────────────────────────────────────────────────────────────

struct MockPlatform {
    replies: tokio::sync::Mutex<Vec<String>>,
}

impl MockPlatform {
    fn new() -> Arc<Self> {
        Arc::new(Self { replies: tokio::sync::Mutex::new(Vec::new()) })
    }
    async fn replies(&self) -> Vec<String> {
        self.replies.lock().await.clone()
    }
}

#[async_trait]
impl MessagingPlatform for MockPlatform {
    fn name(&self) -> &'static str { "mock" }
    async fn run(self: Arc<Self>, _: mpsc::Sender<InboundMessage>, _: mpsc::Sender<ShedNotice>) {}
    async fn send_reply(&self, _: &str, _: &str, text: &str, _: Option<&str>) -> Result<(), String> {
        self.replies.lock().await.push(text.to_string());
        Ok(())
    }
    async fn send_confirm_prompt(&self, _: &str, _: &str, token: &str, _: &str, nonce: i64) -> Result<String, String> {
        Ok(format!("{token}:{nonce}"))
    }
}

// ── MockBuddyApi ─────────────────────────────────────────────────────────────

/// Binds a minimal Buddy-compatible `/v1/chat/completions` endpoint on a random port.
async fn start_mock_buddy(response_text: &'static str) -> (String, Arc<AtomicU32>) {
    let call_count = Arc::new(AtomicU32::new(0));
    let count2     = call_count.clone();

    let app = AxumRouter::new().route("/v1/chat/completions", post(move || {
        let count3 = count2.clone();
        async move {
            count3.fetch_add(1, Ordering::Relaxed);
            Json(json!({
                "choices": [{ "message": { "content": response_text }, "finish_reason": "stop" }]
            }))
        }
    }));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port     = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://127.0.0.1:{port}"), call_count)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn make_router(buddy_url: String) -> Arc<bonsai_bot::router::Router> {
    use bonsai_bot::{buddy_client::BuddyClient, dedup::DedupCache, health::CircuitBreaker, metrics::Metrics, mgmt_client::MgmtClient};
    use bonsai_bot::config::{BotConfig, CircuitBreakerConfig};
    use std::sync::Arc;
    use tokio_rusqlite::Connection;

    let metrics = Arc::new(Metrics::default());
    let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    let buddy   = Arc::new(BuddyClient::new(
        buddy_url,
        "http://127.0.0.1:11369".to_string(),
        vec!["chatbot".to_string()],
        breaker,
        metrics.clone(),
    ));
    let mgmt  = MgmtClient::new("http://127.0.0.1:11369", String::new());
    let dedup = Arc::new(DedupCache::new(1_000, 60));

    // In-memory SQLite for tests
    let db = Arc::new(Connection::open_in_memory().await.unwrap());
    bonsai_bot::session::migrate(&db).await.unwrap();

    Arc::new(bonsai_bot::router::Router::new(buddy, mgmt, dedup, db, metrics, BotConfig::default()))
}

fn make_inbound(text: &str) -> InboundMessage {
    InboundMessage {
        platform:     "mock".to_string(),
        platform_id:  "chan1".to_string(),
        user_id:      "user1".to_string(),
        display_name: "Tester".to_string(),
        event_id:     uuid::Uuid::new_v4().to_string(),
        text:         text.to_string(),
        reply_to:     None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn roundtrip_basic_message() {
    let (url, call_count) = start_mock_buddy("Hello from Bonsai!").await;
    let router   = make_router(url).await;
    let platform = MockPlatform::new();

    router.handle(make_inbound("hi"), &(platform.clone() as Arc<dyn MessagingPlatform>)).await;

    assert_eq!(call_count.load(Ordering::Relaxed), 1, "buddy called once");
    let replies = platform.replies().await;
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0], "Hello from Bonsai!");
}

#[tokio::test]
async fn dedup_drops_duplicate_event() {
    let (url, call_count) = start_mock_buddy("pong").await;
    let router   = make_router(url).await;
    let platform = MockPlatform::new();

    let mut msg = make_inbound("ping");
    msg.event_id = "fixed-event-id".to_string();

    let plat: Arc<dyn MessagingPlatform> = platform.clone();
    router.handle(msg.clone(), &plat).await;
    router.handle(msg.clone(), &plat).await; // duplicate

    // Buddy should only be called once
    assert_eq!(call_count.load(Ordering::Relaxed), 1, "dedup suppressed second call");
    assert_eq!(platform.replies().await.len(), 1);
}

#[tokio::test]
async fn circuit_breaker_opens_after_failures() {
    use bonsai_bot::{buddy_client::BuddyClient, dedup::DedupCache, health::CircuitBreaker, metrics::Metrics, mgmt_client::MgmtClient};
    use bonsai_bot::config::{BotConfig, CircuitBreakerConfig};
    use tokio_rusqlite::Connection;

    let metrics = Arc::new(Metrics::default());
    let cb_cfg  = CircuitBreakerConfig { open_after_failures: 2, half_open_probe_secs: 60, close_on_successes: 1 };
    let breaker = CircuitBreaker::new(cb_cfg);
    let buddy   = Arc::new(BuddyClient::new(
        "http://127.0.0.1:19999".to_string(), // nothing listening here
        "http://127.0.0.1:11369".to_string(),
        vec![],
        breaker.clone(),
        metrics.clone(),
    ));
    let mgmt  = MgmtClient::new("http://127.0.0.1:11369", String::new());
    let dedup = Arc::new(DedupCache::new(100, 60));
    let db    = Arc::new(Connection::open_in_memory().await.unwrap());
    bonsai_bot::session::migrate(&db).await.unwrap();
    let router   = Arc::new(bonsai_bot::router::Router::new(buddy, mgmt, dedup, db, metrics, BotConfig::default()));
    let platform = MockPlatform::new();
    let plat: Arc<dyn MessagingPlatform> = platform.clone();

    // Two failures should open the circuit
    router.handle(make_inbound("msg1"), &plat).await;
    router.handle(make_inbound("msg2"), &plat).await;

    assert!(breaker.is_open(), "circuit should be open after 2 failures");
}
