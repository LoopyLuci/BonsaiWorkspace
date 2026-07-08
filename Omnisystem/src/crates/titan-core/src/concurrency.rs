//! Clojure concurrency primitives: atom, ref, agent, var

use parking_lot::RwLock;
use std::sync::Arc;

/// Atom - shared mutable state with atomic updates
pub struct Atom<T> {
    value: Arc<RwLock<T>>,
}

impl<T: Clone> Atom<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }

    pub fn deref(&self) -> T {
        self.value.read().clone()
    }

    pub fn swap<F>(&self, f: F)
    where
        F: Fn(T) -> T,
    {
        let mut guard = self.value.write();
        *guard = f(guard.clone());
    }
}

impl<T: Clone> Clone for Atom<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

/// Ref - software transactional memory reference
pub struct Ref<T> {
    value: Arc<RwLock<T>>,
}

impl<T: Clone> Ref<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }

    pub fn deref(&self) -> T {
        self.value.read().clone()
    }

    pub fn alter<F>(&self, f: F)
    where
        F: Fn(T) -> T,
    {
        let mut guard = self.value.write();
        *guard = f(guard.clone());
    }
}

impl<T: Clone> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

/// Agent - asynchronous, independent state
pub struct Agent<T> {
    value: Arc<RwLock<T>>,
}

impl<T: Clone> Agent<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }

    pub fn deref(&self) -> T {
        self.value.read().clone()
    }

    pub fn send<F>(&self, f: F)
    where
        F: Fn(T) -> T,
    {
        let mut guard = self.value.write();
        *guard = f(guard.clone());
    }
}

impl<T: Clone> Clone for Agent<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom() {
        let atom = Atom::new(0);
        assert_eq!(atom.deref(), 0);
        atom.swap(|x| x + 1);
        assert_eq!(atom.deref(), 1);
    }

    #[test]
    fn test_ref() {
        let r = Ref::new(5);
        assert_eq!(r.deref(), 5);
        r.alter(|x| x * 2);
        assert_eq!(r.deref(), 10);
    }

    #[test]
    fn test_agent() {
        let agent = Agent::new(100);
        assert_eq!(agent.deref(), 100);
        agent.send(|x| x / 2);
        assert_eq!(agent.deref(), 50);
    }
}
