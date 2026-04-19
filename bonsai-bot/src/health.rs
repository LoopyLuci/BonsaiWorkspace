use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::config::CircuitBreakerConfig;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed   = 0,
    Open     = 1,
    HalfOpen = 2,
}

pub struct CircuitBreaker {
    state:        AtomicU8,
    fail_count:   AtomicU32,
    success_count: AtomicU32,
    cfg:          CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(cfg: CircuitBreakerConfig) -> Arc<Self> {
        Arc::new(Self {
            state:         AtomicU8::new(BreakerState::Closed as u8),
            fail_count:    AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            cfg,
        })
    }

    pub fn state(&self) -> BreakerState {
        match self.state.load(Ordering::Acquire) {
            0 => BreakerState::Closed,
            1 => BreakerState::Open,
            _ => BreakerState::HalfOpen,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state() == BreakerState::Open
    }

    pub fn record_success(&self) {
        self.fail_count.store(0, Ordering::Release);
        match self.state() {
            BreakerState::HalfOpen => {
                let n = self.success_count.fetch_add(1, Ordering::AcqRel) + 1;
                if n >= self.cfg.close_on_successes {
                    self.success_count.store(0, Ordering::Release);
                    self.state.store(BreakerState::Closed as u8, Ordering::Release);
                    tracing::info!("[breaker] Closed after probe succeeded");
                }
            }
            BreakerState::Open => {}
            BreakerState::Closed => {}
        }
    }

    pub fn record_failure(self: &Arc<Self>) {
        self.success_count.store(0, Ordering::Release);
        match self.state() {
            BreakerState::Closed => {
                let n = self.fail_count.fetch_add(1, Ordering::AcqRel) + 1;
                if n >= self.cfg.open_after_failures {
                    self.state.store(BreakerState::Open as u8, Ordering::Release);
                    tracing::warn!("[breaker] Opened after {n} consecutive failures");
                    let me = self.clone();
                    let probe_secs = self.cfg.half_open_probe_secs;
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(probe_secs)).await;
                        me.state.store(BreakerState::HalfOpen as u8, Ordering::Release);
                        me.success_count.store(0, Ordering::Release);
                        tracing::info!("[breaker] HalfOpen — probing");
                    });
                }
            }
            BreakerState::HalfOpen => {
                // Probe failed — stay open, schedule another probe
                self.state.store(BreakerState::Open as u8, Ordering::Release);
                let me = self.clone();
                let probe_secs = self.cfg.half_open_probe_secs;
                tokio::spawn(async move {
                    sleep(Duration::from_secs(probe_secs)).await;
                    me.state.store(BreakerState::HalfOpen as u8, Ordering::Release);
                    tracing::info!("[breaker] HalfOpen — retrying probe");
                });
            }
            BreakerState::Open => {}
        }
    }
}

/// Poll Buddy /health until up or timeout elapses.
pub async fn wait_for_buddy(buddy_url: &str, timeout_secs: u64) -> bool {
    let url = format!("{buddy_url}/health");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if let Ok(r) = reqwest::get(&url).await {
            if r.status().is_success() {
                return true;
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
    false
}
