use crate::Result;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LockFreeQueue<T> {
    _phantom: std::marker::PhantomData<T>,
    operations: AtomicUsize,
}

pub struct LockFreeStack<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send> LockFreeQueue<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            operations: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, _item: T) -> Result<()> {
        self.operations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn pop(&self) -> Result<Option<T>> {
        Ok(None)
    }

    pub fn throughput(&self) -> u64 {
        self.operations.load(Ordering::Relaxed) as u64
    }
}

impl<T> Default for LockFreeQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LockFreeStack<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
