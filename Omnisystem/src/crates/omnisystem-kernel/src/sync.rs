use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

/// Simple spinlock (busy-wait)
pub struct SpinLock {
    locked: std::sync::atomic::AtomicBool,
}

impl SpinLock {
    pub fn new() -> Self {
        SpinLock {
            locked: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        while self
            .locked
            .compare_exchange(false, true, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed)
            .is_err()
        {
            // Busy wait
            std::hint::spin_loop();
        }
    }

    pub fn unlock(&self) {
        self.locked
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed)
            .is_ok()
    }
}

/// Semaphore - synchronization primitive for counting
pub struct Semaphore {
    counter: Mutex<i32>,
}

impl Semaphore {
    pub fn new(initial_count: i32) -> Self {
        Semaphore {
            counter: Mutex::new(initial_count),
        }
    }

    pub fn wait(&self) {
        let mut count = self.counter.lock();
        while *count <= 0 {
            drop(count);
            std::thread::yield_now();
            count = self.counter.lock();
        }
        *count -= 1;
    }

    pub fn signal(&self) {
        let mut count = self.counter.lock();
        *count += 1;
    }

    pub fn try_wait(&self) -> bool {
        let mut count = self.counter.lock();
        if *count > 0 {
            *count -= 1;
            true
        } else {
            false
        }
    }

    pub fn value(&self) -> i32 {
        *self.counter.lock()
    }
}

/// Mutex using parking_lot for efficiency
pub fn new_mutex<T>(value: T) -> Mutex<T> {
    Mutex::new(value)
}

/// RwLock for reader-writer synchronization
pub fn new_rwlock<T>(value: T) -> RwLock<T> {
    RwLock::new(value)
}

/// Event - synchronization for wait/notify pattern
pub struct Event {
    signaled: Mutex<bool>,
}

impl Event {
    pub fn new() -> Self {
        Event {
            signaled: Mutex::new(false),
        }
    }

    pub fn signal(&self) {
        *self.signaled.lock() = true;
    }

    pub fn reset(&self) {
        *self.signaled.lock() = false;
    }

    pub fn wait(&self) {
        while !*self.signaled.lock() {
            std::thread::yield_now();
        }
    }

    pub fn try_wait(&self) -> bool {
        *self.signaled.lock()
    }
}

/// Barrier - synchronization for multiple threads meeting
pub struct Barrier {
    count: Mutex<usize>,
    total: usize,
    signaled: Mutex<bool>,
}

impl Barrier {
    pub fn new(count: usize) -> Self {
        Barrier {
            count: Mutex::new(0),
            total: count,
            signaled: Mutex::new(false),
        }
    }

    pub fn wait(&self) {
        let mut count = self.count.lock();
        *count += 1;

        if *count >= self.total {
            *self.signaled.lock() = true;
        } else {
            drop(count);
            while !*self.signaled.lock() {
                std::thread::yield_now();
            }
        }
    }

    pub fn reset(&self) {
        *self.count.lock() = 0;
        *self.signaled.lock() = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinlock() {
        let lock = SpinLock::new();
        assert!(lock.try_lock());
        lock.unlock();
    }

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(2);
        assert_eq!(sem.value(), 2);
        sem.wait();
        assert_eq!(sem.value(), 1);
        sem.signal();
        assert_eq!(sem.value(), 2);
    }

    #[test]
    fn test_event() {
        let event = Event::new();
        assert!(!event.try_wait());
        event.signal();
        assert!(event.try_wait());
    }
}
