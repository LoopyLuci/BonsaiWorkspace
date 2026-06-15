use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Maximum consecutive restart attempts before giving up.
const MAX_RESTARTS: u32 = 5;
/// Initial back-off delay; doubles on each failure (capped at 30 s).
const INITIAL_BACKOFF_MS: u64 = 200;
const MAX_BACKOFF_MS: u64 = 30_000;

/// Factory type: a closure that produces a new future representing the actor.
/// Must be `Send + Sync` so the supervisor can call it from any thread.
pub type ActorFactory = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Descriptor for one supervised task.
pub struct ChildSpec {
    pub name: String,
    pub factory: ActorFactory,
}

impl ChildSpec {
    pub fn new<N, F, Fut>(name: N, factory: F) -> Self
    where
        N: Into<String>,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            name: name.into(),
            factory: Box::new(move || Box::pin(factory())),
        }
    }
}

/// One-for-one supervisor: when a child exits (panic or clean), it is restarted
/// with exponential back-off up to `MAX_RESTARTS` attempts.
///
/// Call `Supervisor::run(specs)` and await it; it drives the restart loop until
/// all children exhaust their restart budget or the Tokio runtime shuts down.
pub struct Supervisor;

impl Supervisor {
    /// Run all children under a one-for-one supervision strategy.
    /// This future does not return unless all children are permanently stopped.
    pub async fn run(specs: Vec<ChildSpec>) {
        let mut handles: Vec<(ChildSpec, JoinHandle<()>, u32, u64)> = specs
            .into_iter()
            .map(|spec| {
                let handle = tokio::spawn((spec.factory)());
                (spec, handle, 0u32, INITIAL_BACKOFF_MS)
            })
            .collect();

        loop {
            if handles.is_empty() {
                info!("supervisor: all children stopped — exiting");
                return;
            }

            // Wait for any child to finish, returning the index once found.
            let idx: usize = 'outer: loop {
                for (i, (_, handle, _, _)) in handles.iter_mut().enumerate() {
                    if handle.is_finished() {
                        break 'outer i;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            };
            let (spec, handle, restarts, backoff) = handles.remove(idx);

            // Inspect the exit status.
            let restart = match handle.await {
                Ok(()) => {
                    info!(child = %spec.name, "supervisor: child exited cleanly; restarting");
                    true
                }
                Err(e) if e.is_panic() => {
                    error!(child = %spec.name, "supervisor: child panicked; attempting restart");
                    true
                }
                Err(e) => {
                    warn!(child = %spec.name, error = %e, "supervisor: child cancelled");
                    false
                }
            };

            if restart && restarts < MAX_RESTARTS {
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                let new_handle = tokio::spawn((spec.factory)());
                let next_backoff = (backoff * 2).min(MAX_BACKOFF_MS);
                handles.push((spec, new_handle, restarts + 1, next_backoff));
            } else if restart {
                error!(child = %spec.name, restarts, "supervisor: restart budget exhausted — child will not be restarted");
            }
        }
    }
}
