/// IPC resilience — async retry wrapper for Tauri command internals.
///
/// Use `with_retry` to wrap fallible async operations inside a command handler.
/// Failures are logged; the final error is returned so Tauri serialises it for
/// the frontend (which retries again via `resilientInvoke`).

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay:   Duration,
    pub max_delay:    Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay:   Duration::from_millis(300),
            max_delay:    Duration::from_secs(10),
        }
    }
}

/// Execute `op` with exponential back-off retries.
///
/// Returns `Ok(T)` on the first success or `Err(E)` after all attempts fail.
pub async fn with_retry<F, Fut, T, E>(
    label: &str,
    policy: RetryPolicy,
    mut op: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0u32;
    loop {
        match op().await {
            Ok(val) => {
                if attempt > 0 {
                    debug!("[ipc_resilience] '{label}' succeeded after {attempt} retries");
                }
                return Ok(val);
            }
            Err(e) => {
                attempt += 1;
                if attempt >= policy.max_attempts {
                    warn!("[ipc_resilience] '{label}' failed after {attempt} attempts: {e}");
                    return Err(e);
                }
                let base_ms = policy.base_delay.as_millis() as u64;
                let raw_ms  = base_ms.saturating_mul(1u64 << attempt.min(10));
                let cap_ms  = policy.max_delay.as_millis() as u64;
                // ±20 % jitter
                let jitter_ms = (raw_ms as f64 * 0.2 * (rand::random::<f64>() - 0.5)) as i64;
                let wait_ms   = (raw_ms.min(cap_ms) as i64 + jitter_ms).max(0) as u64;
                debug!(
                    "[ipc_resilience] '{label}' attempt {attempt} failed: {e}. Retry in {wait_ms}ms"
                );
                sleep(Duration::from_millis(wait_ms)).await;
            }
        }
    }
}
