mod admin_api;
mod buddy_client;
mod config;
mod dedup;
mod formatter;
mod health;
mod metrics;
mod platforms;
mod router;
mod sanitizer;
mod session;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::admin_api::PlatformStates;
use crate::buddy_client::BuddyClient;
use crate::config::{ensure_admin_token, keyring_get, load_config};
use crate::dedup::DedupCache;
use crate::health::{wait_for_buddy, CircuitBreaker};
use crate::metrics::Metrics;
use crate::platforms::{InboundMessage, MessagingPlatform, ShedNotice};
use crate::router::Router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    tracing::info!("[bonsai-bot] Starting v{}", env!("CARGO_PKG_VERSION"));

    let cfg = load_config();
    let admin_token = ensure_admin_token().expect("keychain access for admin token");

    // Wait for Buddy
    if !wait_for_buddy(&cfg.buddy_api_url, 60).await {
        tracing::warn!("[bonsai-bot] Buddy not reachable after 60s — starting with circuit breaker open");
    }

    // SQLite
    let db = Arc::new(
        tokio_rusqlite::Connection::open(&cfg.db_path)
            .await
            .expect("SQLite open")
    );

    session::migrate(&db).await.expect("DB migrate");

    // Shared state
    let metrics = Arc::new(Metrics::default());
    let breaker = CircuitBreaker::new(cfg.circuit_breaker.clone());
    let buddy   = Arc::new(BuddyClient::new(
        cfg.buddy_api_url.clone(), breaker.clone(), metrics.clone()
    ));
    let dedup   = Arc::new(DedupCache::new(10_000, 600));

    // Router created first — Discord/Telegram platforms need it for button callback handling
    let router = Arc::new(Router::new(
        buddy.clone(),
        dedup.clone(),
        db.clone(),
        metrics.clone(),
        cfg.clone(),
    ));

    // Per-platform connection state — written by platforms, read by /status
    let platform_states: PlatformStates = Arc::new(dashmap::DashMap::new());

    // Channels
    let (tx, mut rx) = mpsc::channel::<InboundMessage>(cfg.backpressure.inbound_queue_capacity);
    let (shed_tx, mut shed_rx) = mpsc::channel::<ShedNotice>(64);

    // Admin API
    let admin = admin_api::start(cfg.admin_port, metrics.clone(), platform_states.clone(), admin_token)
        .await
        .expect("admin API startup");
    tracing::info!("[bonsai-bot] Admin API on port {}", admin.port);

    // Platform tasks
    let mut platform_list: Vec<Arc<dyn MessagingPlatform>> = Vec::new();

    #[cfg(feature = "discord")]
    if cfg.discord.enabled {
        if let Some(token) = keyring_get("discord_token") {
            use crate::platforms::discord::DiscordPlatform;
            let p = DiscordPlatform::new(token, cfg.discord.config.clone(), metrics.clone(), router.clone(), platform_states.clone());
            let p2 = p.clone() as Arc<dyn MessagingPlatform>;
            platform_list.push(p2);
            let tx2    = tx.clone();
            let shed2  = shed_tx.clone();
            tokio::spawn(async move { p.run(tx2, shed2).await });
        } else {
            tracing::warn!("[discord] No token in keychain — platform disabled");
        }
    }

    #[cfg(feature = "telegram")]
    if cfg.telegram.enabled {
        if let Some(token) = keyring_get("telegram_token") {
            use crate::platforms::telegram::TelegramPlatform;
            let p = TelegramPlatform::new(token, cfg.telegram.config.clone(), metrics.clone(), router.clone(), platform_states.clone());
            let p2 = p.clone() as Arc<dyn MessagingPlatform>;
            platform_list.push(p2);
            let tx2   = tx.clone();
            let shed2 = shed_tx.clone();
            tokio::spawn(async move { p.run(tx2, shed2).await });
        } else {
            tracing::warn!("[telegram] No token in keychain — platform disabled");
        }
    }

    #[cfg(feature = "email")]
    if cfg.email.enabled {
        if let (Some(imap_pass), Some(smtp_pass)) =
            (keyring_get("email_imap_password"), keyring_get("email_smtp_password"))
        {
            use crate::platforms::email::EmailPlatform;
            let p = EmailPlatform::new(imap_pass, smtp_pass, cfg.email.config.clone(), metrics.clone(), platform_states.clone());
            let p2 = p.clone() as Arc<dyn MessagingPlatform>;
            platform_list.push(p2);
            let tx2   = tx.clone();
            let shed2 = shed_tx.clone();
            tokio::spawn(async move { p.run(tx2, shed2).await });
        } else {
            tracing::warn!("[email] No password in keychain — platform disabled");
        }
    }

    #[cfg(feature = "matrix")]
    if cfg.matrix.enabled {
        if let Some(password) = keyring_get("matrix_password") {
            use crate::platforms::matrix::MatrixPlatform;
            let p = MatrixPlatform::new(password, cfg.matrix.config.clone(), metrics.clone(), platform_states.clone());
            let p2 = p.clone() as Arc<dyn MessagingPlatform>;
            platform_list.push(p2);
            let tx2   = tx.clone();
            let shed2 = shed_tx.clone();
            tokio::spawn(async move { p.run(tx2, shed2).await });
        } else {
            tracing::warn!("[matrix] No password in keychain — platform disabled");
        }
    }

    let platforms: Arc<Vec<Arc<dyn MessagingPlatform>>> = Arc::new(platform_list);

    // Shed reply task (low-cost control path, outside worker pool)
    tokio::spawn(async move {
        while let Some(notice) = shed_rx.recv().await {
            tracing::debug!("[router] Queue full shed for platform={}", notice.platform);
        }
    });

    // Pending confirm recovery on startup
    {
        let pending = session::load_unresolved_confirms(&db).await;
        tracing::info!("[bonsai-bot] {} pending confirms on startup", pending.len());
        for pc in pending {
            if let Ok(nonce) = session::mark_prompted(&db, pc.token.clone()).await {
                for plat in platforms.iter() {
                    if plat.name() == pc.platform {
                        let _ = plat.send_confirm_prompt(
                            &pc.chat_id, &pc.user_id, &pc.token, &pc.prompt, nonce,
                        ).await;
                    }
                }
            }
        }
    }

    // Background confirm cleanup
    {
        let db2 = db.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;
                session::purge_expired_confirms(&db2).await;
                session::cleanup_stale(&db2).await;
            }
        });
    }

    // Main processing loop — single receiver dispatches to per-message tasks,
    // bounded by global semaphore (cfg.backpressure.global_semaphore in-flight max)
    let semaphore = Arc::new(tokio::sync::Semaphore::new(cfg.backpressure.global_semaphore));
    let sem   = semaphore.clone();
    let rtr   = router.clone();
    let plats = platforms.clone();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                let msg = match msg { Some(m) => m, None => break };

                let platform_name = msg.platform.clone();
                let plat = plats.iter().find(|p| p.name() == platform_name).cloned();
                let plat = match plat {
                    Some(p) => p,
                    None => {
                        tracing::warn!("[router] No platform handler for '{platform_name}'");
                        continue;
                    }
                };

                let permit = match sem.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        let _ = plat.send_reply(&msg.platform_id, &msg.user_id,
                            "🔄 At capacity. Please wait.", msg.reply_to.as_deref()).await;
                        continue;
                    }
                };

                let rtr2 = rtr.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    rtr2.handle(msg, &plat).await;
                });
            }
        }
    }

    tracing::info!("[bonsai-bot] Shutdown complete");
}
