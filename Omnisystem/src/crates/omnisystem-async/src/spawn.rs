/// Task spawning utilities

use crate::GLOBAL_RUNTIME;

pub type SpawnFuture<T> = tokio::task::JoinHandle<T>;

/// Spawn a task on the global runtime
pub fn spawn<F>(future: F) -> SpawnFuture<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    GLOBAL_RUNTIME.spawn(future)
}

/// Spawn a blocking task
pub fn spawn_blocking<F, R>(f: F) -> SpawnFuture<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    GLOBAL_RUNTIME.tokio_runtime().spawn_blocking(f)
}

/// Spawn multiple tasks and wait for all to complete
pub async fn join_all<F: std::future::Future + Send + 'static>(
    futures: Vec<SpawnFuture<F::Output>>,
) -> Vec<Result<F::Output, tokio::task::JoinError>>
where
    F::Output: Send + 'static,
{
    futures::future::join_all(futures).await
}

/// Sleep for duration
pub async fn sleep(duration: std::time::Duration) {
    tokio::time::sleep(duration).await
}

/// Sleep with timeout
pub async fn timeout<F: std::future::Future>(
    duration: std::time::Duration,
    future: F,
) -> Result<F::Output, tokio::time::error::Elapsed> {
    tokio::time::timeout(duration, future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn() {
        let handle = spawn(async { 42 });
        let result = handle.await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_spawn_blocking() {
        let handle = spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            "done"
        });

        let result = handle.await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
    }

    #[tokio::test]
    async fn test_timeout() {
        let result = timeout(std::time::Duration::from_millis(10), sleep(std::time::Duration::from_secs(1))).await;
        assert!(result.is_err());
    }
}
