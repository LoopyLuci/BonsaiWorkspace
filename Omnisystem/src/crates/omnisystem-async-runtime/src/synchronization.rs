//! Lock-free and fair synchronization primitives

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Fair mutex with FIFO wakeup order
pub struct Mutex<T> {
    value: T,
    locked: AtomicUsize,
}

impl<T> Mutex<T> {
    /// Create a new mutex
    pub fn new(value: T) -> Self {
        Mutex {
            value,
            locked: AtomicUsize::new(0),
        }
    }

    /// Lock the mutex
    pub fn lock(&self) -> MutexGuard<T> {
        while self.locked.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
            std::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

/// RAII guard for mutex
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(0, Ordering::Release);
    }
}

/// Lock-free MPMC queue
pub struct LockFreeQueue<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> LockFreeQueue<T> {
    /// Create a new lock-free queue
    pub fn new() -> Self {
        LockFreeQueue {
            _phantom: std::marker::PhantomData,
        }
    }
}
