//! Reactive Queries — Data That Pushes Updates
//!
//! LiveSet<T> and Live<T> are collections that automatically update when
//! the underlying data changes, pushing deltas to subscribed actors.

use std::sync::Arc;
use parking_lot::RwLock;

/// A reactive collection — automatically stays in sync with the database
#[derive(Clone)]
pub struct LiveSet<T: Clone + Send + Sync + 'static> {
    data: Arc<RwLock<Vec<T>>>,
    subscribers: Arc<RwLock<Vec<Box<dyn Fn(SetDelta<T>) + Send + Sync>>>>,
}

/// A reactive single value — automatically stays in sync with the database
#[derive(Clone)]
pub struct Live<T: Clone + Send + Sync + 'static> {
    data: Arc<RwLock<T>>,
    subscribers: Arc<RwLock<Vec<Box<dyn Fn(T) + Send + Sync>>>>,
}

/// Delta representing changes to a LiveSet
#[derive(Debug, Clone)]
pub struct SetDelta<T: Clone> {
    pub added: Vec<T>,
    pub removed: Vec<T>,
    pub modified: Vec<T>,
}

impl<T: Clone + Send + Sync + 'static> LiveSet<T> {
    pub fn new(initial: Vec<T>) -> Self {
        Self {
            data: Arc::new(RwLock::new(initial)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Get a snapshot of the current data
    pub fn snapshot(&self) -> Vec<T> {
        self.data.read().clone()
    }

    /// Subscribe to changes
    pub fn observe<F>(&self, callback: F)
    where
        F: Fn(SetDelta<T>) + Send + Sync + 'static,
    {
        self.subscribers.write().push(Box::new(callback));
    }

    /// Push a delta to all subscribers
    pub fn notify(&self, delta: SetDelta<T>) {
        let subscribers = self.subscribers.read();
        for callback in subscribers.iter() {
            callback(delta.clone());
        }
    }

    /// Add items to the set and notify
    pub fn add(&self, items: Vec<T>) {
        self.data.write().extend(items.clone());
        self.notify(SetDelta {
            added: items,
            removed: Vec::new(),
            modified: Vec::new(),
        });
    }

    /// Remove items from the set and notify
    pub fn remove(&self, items: Vec<T>)
    where
        T: PartialEq,
    {
        let mut data = self.data.write();
        items.iter().for_each(|item| {
            data.retain(|x| x != item);
        });
        self.notify(SetDelta {
            added: Vec::new(),
            removed: items,
            modified: Vec::new(),
        });
    }

    /// Get the length of the set
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Iterate over the current snapshot
    pub fn iter<F>(&self, f: F)
    where
        F: Fn(&T),
    {
        let data = self.data.read();
        for item in data.iter() {
            f(item);
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Live<T> {
    pub fn new(initial: T) -> Self {
        Self {
            data: Arc::new(RwLock::new(initial)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the current value
    pub fn get(&self) -> T {
        self.data.read().clone()
    }

    /// Set the value and notify subscribers
    pub fn set(&self, value: T) {
        *self.data.write() = value.clone();
        self.notify(value);
    }

    /// Subscribe to changes
    pub fn observe<F>(&self, callback: F)
    where
        F: Fn(T) + Send + Sync + 'static,
    {
        self.subscribers.write().push(Box::new(callback));
    }

    /// Notify all subscribers of a change
    pub fn notify(&self, value: T) {
        let subscribers = self.subscribers.read();
        for callback in subscribers.iter() {
            callback(value.clone());
        }
    }

    /// Modify the value in-place using a closure
    pub fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        {
            let mut data = self.data.write();
            f(&mut data);
        }
        let value = self.get();
        self.notify(value);
    }
}

impl<T: Clone> SetDelta<T> {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

impl<T: Clone> Default for SetDelta<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_set() {
        let set = LiveSet::new(vec![1, 2, 3]);
        assert_eq!(set.len(), 3);

        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        set.observe(move |delta| {
            received_clone.lock().unwrap().push(delta);
        });

        set.add(vec![4]);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_live_value() {
        let val = Live::new(42);
        assert_eq!(val.get(), 42);

        val.set(100);
        assert_eq!(val.get(), 100);

        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        val.observe(move |v| {
            received_clone.lock().unwrap().push(v);
        });

        val.set(200);
        assert_eq!(val.get(), 200);
    }
}
