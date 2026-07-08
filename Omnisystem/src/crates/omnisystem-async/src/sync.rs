/// Async synchronization primitives

use tokio::sync::{Mutex, RwLock, Semaphore, Barrier};
use std::sync::Arc;

pub use tokio::sync::broadcast;
pub use tokio::sync::mpsc;

/// Async lock wrapper
pub struct AsyncLock<T> {
    inner: Mutex<T>,
}

impl<T> AsyncLock<T> {
    pub fn new(value: T) -> Self {
        AsyncLock {
            inner: Mutex::new(value),
        }
    }

    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, T> {
        self.inner.lock().await
    }

    pub fn blocking_lock(&self) -> tokio::sync::MutexGuard<'_, T> {
        // In async context, use tokio's blocking variant
        let rt = tokio::runtime::Handle::try_current();
        if rt.is_ok() {
            // We're in async context, can't block
            panic!("Cannot use blocking_lock in async context");
        }
        // This would need to be handled differently in sync contexts
        panic!("blocking_lock not supported");
    }
}

/// Async RwLock wrapper
pub struct AsyncRwLock<T> {
    inner: RwLock<T>,
}

impl<T> AsyncRwLock<T> {
    pub fn new(value: T) -> Self {
        AsyncRwLock {
            inner: RwLock::new(value),
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.inner.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().await
    }
}

/// Async semaphore wrapper
pub struct AsyncSemaphore {
    inner: Semaphore,
}

impl AsyncSemaphore {
    pub fn new(permits: usize) -> Self {
        AsyncSemaphore {
            inner: Semaphore::new(permits),
        }
    }

    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> {
        self.inner.acquire().await
    }

    pub async fn acquire_many(&self, n: u32) -> Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> {
        self.inner.acquire_many(n).await
    }
}

/// Async barrier wrapper
pub struct AsyncBarrier {
    inner: Arc<Barrier>,
}

impl AsyncBarrier {
    pub fn new(n: usize) -> Self {
        AsyncBarrier {
            inner: Arc::new(Barrier::new(n)),
        }
    }

    pub async fn wait(&self) {
        self.inner.wait().await;
    }

    pub fn clone(&self) -> Self {
        AsyncBarrier {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Channel creation helpers
pub fn create_channel<T>(capacity: usize) -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    mpsc::channel(capacity)
}

pub fn create_broadcast<T: Clone>(capacity: usize) -> broadcast::Sender<T> {
    let (tx, _rx) = broadcast::channel(capacity);
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_lock() {
        let lock = AsyncLock::new(42);
        {
            let mut value = lock.lock().await;
            *value = 100;
        }
        let value = lock.lock().await;
        assert_eq!(*value, 100);
    }

    #[tokio::test]
    async fn test_async_rwlock() {
        let lock = AsyncRwLock::new("hello");
        {
            let value = lock.read().await;
            assert_eq!(*value, "hello");
        }
        {
            let mut value = lock.write().await;
            *value = "world";
        }
    }

    #[tokio::test]
    async fn test_async_semaphore() {
        let sem = AsyncSemaphore::new(2);
        let _permit1 = sem.acquire().await;
        let _permit2 = sem.acquire().await;
    }

    #[test]
    fn test_create_channel() {
        let (_tx, _rx) = create_channel::<i32>(10);
    }
}
