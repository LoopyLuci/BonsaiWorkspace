// All modules are declared in lib.rs; import them here
use bonsai_bot::{admin_api, buddy_client, config, dedup, health, metrics, platforms, router, scheduler, session, swarm_client};

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

/// Spawn a platform and restart it with exponential backoff (1→2→4→8→16→32s, max 10 retries)
/// if it exits. Each successful run resets the backoff counter.
fn spawn_platform_with_backoff(
    p: Arc<dyn MessagingPlatform>,
    tx: mpsc::Sender<InboundMessage>,
    shed_tx: mpsc::Sender<ShedNotice>,
) {
    tokio::spawn(async move {
        let mut delay_secs: u64 = 1;
        let mut attempts: u32   = 0;
        loop {
            let name  = p.name();
            let p2    = p.clone();
            let tx2   = tx.clone();
            let shed2 = shed_tx.clone();
            p2.run(tx2, shed2).await;
            attempts += 1;
            if attempts >= 10 {
                tracing::error!("[{name}] Platform exited 10 times — giving up");
                break;
            }
            tracing::warn!("[{name}] Platform exited (attempt {attempts}); reconnecting in {delay_secs}s");
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            delay_secs = (delay_secs * 2).min(32);
        }
    });
}

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

    // Config validation
    {
        let mut errs: Vec<&str> = Vec::new();
        if cfg.buddy_api_url.is_empty()     { errs.push("buddy_api_url is empty"); }
        if cfg.preferred_model_tags.is_empty() { errs.push("preferred_model_tags is empty"); }
        let any_platform = cfg.discord.enabled || cfg.telegram.enabled || cfg.email.enabled || cfg.matrix.enabled;
        if !any_platform { tracing::warn!("[bonsai-bot] No platforms are enabled — bot will not receive messages"); }
        for e in &errs { tracing::error!("[bonsai-bot] Config error: {e}"); }
        if !errs.is_empty() { std::process::exit(1); }
    }

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

    // Sync skill manifests from disk into DB on every startup
    {
        let db2 = db.clone();
        let paths = cfg.allowed_script_paths.clone();
        tokio::spawn(async move {
            session::sync_skills_from_disk(&db2, paths).await;
        });
    }

    // Shared state
    let metrics = Arc::new(Metrics::default());
    let breaker = CircuitBreaker::new(cfg.circuit_breaker.clone());
    let buddy   = Arc::new(BuddyClient::new(
        cfg.buddy_api_url.clone(),
        cfg.workspace_api_url.clone(),
        cfg.preferred_model_tags.clone(),
        breaker.clone(),
        metrics.clone(),
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

    let swarm = swarm_client::SwarmClient::new(cfg.swarm_peers.clone());

    // Per-platform connection state — written by platforms, read by /status
    let platform_states: PlatformStates = Arc::new(dashmap::DashMap::new());

    // Channels
    let (tx, mut rx) = mpsc::channel::<InboundMessage>(cfg.backpressure.inbound_queue_capacity);
    let (shed_tx, mut shed_rx) = mpsc::channel::<ShedNotice>(64);
    let (broadcast_tx, mut broadcast_rx) = mpsc::channel::<admin_api::BroadcastRequest>(64);

    // Admin API
    let admin = admin_api::start(
        cfg.admin_port,
        metrics.clone(),
        platform_states.clone(),
        db.clone(),
        broadcast_tx,
        admin_token,
    )
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
            platform_list.push(p2.clone());
            spawn_platform_with_backoff(p2, tx.clone(), shed_tx.clone());
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
            platform_list.push(p2.clone());
            spawn_platform_with_backoff(p2, tx.clone(), shed_tx.clone());
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
            platform_list.push(p2.clone());
            spawn_platform_with_backoff(p2, tx.clone(), shed_tx.clone());
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
            platform_list.push(p2.clone());
            spawn_platform_with_backoff(p2, tx.clone(), shed_tx.clone());
        } else {
            tracing::warn!("[matrix] No password in keychain — platform disabled");
        }
    }

    let platforms: Arc<Vec<Arc<dyn MessagingPlatform>>> = Arc::new(platform_list);

    // Scheduled tasks — fires synthetic InboundMessages into the same queue
    {
        let sched = scheduler::Scheduler::load();
        sched.spawn_all(tx.clone(), platforms.clone());
    }

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

    // Background confirm cleanup (every minute) + session TTL purge (daily)
    {
        let db2 = db.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            let mut daily_counter: u64 = 0;
            loop {
                ticker.tick().await;
                session::purge_expired_confirms(&db2).await;
                session::cleanup_stale(&db2).await;
                daily_counter += 1;
                // Purge hard-deleted sessions once per day (every 1440 ticks of 60s)
                if daily_counter % 1440 == 0 {
                    let purged = session::purge_old_sessions(&db2, 90).await;
                    if purged > 0 {
                        tracing::info!(purged, "session TTL purge completed");
                    }
                }
            }
        });
    }

    // Broadcast task: forward admin /broadcast messages to each target platform
    {
        let plats2 = platforms.clone();
        tokio::spawn(async move {
            while let Some(req) = broadcast_rx.recv().await {
                for plat in plats2.iter() {
                    if req.platforms.is_empty() || req.platforms.contains(&plat.name().to_string()) {
                        let _ = plat.send_reply("", "", &req.message, None).await;
                    }
                }
            }
        });
    }

    // Main processing loop — single receiver dispatches to per-message tasks,
    // bounded by global semaphore (cfg.backpressure.global_semaphore in-flight max)
    let semaphore = Arc::new(tokio::sync::Semaphore::new(cfg.backpressure.global_semaphore));
    let sem   = semaphore.clone();
    let rtr   = router.clone();
    let plats = platforms.clone();
    let swarm2 = swarm.clone();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("[bonsai-bot] SIGINT received — draining queue (30s max)");
                drop(rx); // Close the channel so no new messages are accepted
                let drain_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
                loop {
                    if semaphore.available_permits() == cfg.backpressure.global_semaphore {
                        break; // All in-flight tasks finished
                    }
                    if tokio::time::Instant::now() >= drain_deadline {
                        tracing::warn!("[bonsai-bot] Drain timeout — forcing shutdown");
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                break;
            }
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

                let rtr2   = rtr.clone();
                let swarm3 = swarm2.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    // Try swarm routing first if any peers are configured
                    if swarm3.has_peers() {
                        if let Some(peer) = swarm3.route(&msg).await {
                            if let Err(e) = swarm3.forward(peer, &msg).await {
                                tracing::warn!("[swarm] forward to '{}' failed: {e}; falling back to local", peer.name);
                            } else {
                                return;
                            }
                        }
                    }
                    rtr2.handle(msg, &plat).await;
                });
            }
        }
    }

    tracing::info!("[bonsai-bot] Shutdown complete");
}
