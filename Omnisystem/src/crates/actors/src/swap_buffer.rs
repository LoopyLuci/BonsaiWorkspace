use arc_swap::ArcSwap;
use std::sync::Arc;

/// Lock-free hot-swappable value. Writers swap in a new `Arc<T>`; readers
/// load the latest version without any blocking. Suitable for config/skill
/// hot-reload where reads vastly outnumber writes.
pub struct SwapBuffer<T> {
    inner: ArcSwap<T>,
}

impl<T> SwapBuffer<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: ArcSwap::new(Arc::new(value)),
        }
    }

    /// Atomically replace the stored value. Returns the old value.
    pub fn swap(&self, new_value: T) -> Arc<T> {
        self.inner.swap(Arc::new(new_value))
    }

    /// Load the current value (very cheap — no locking).
    pub fn load(&self) -> arc_swap::Guard<Arc<T>> {
        self.inner.load()
    }

    /// Clone the current `Arc<T>` for long-lived access.
    pub fn load_full(&self) -> Arc<T> {
        self.inner.load_full()
    }
}
