use crate::{Result, ConnectorError};
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaId(Uuid);

impl ArenaId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ArenaId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Arena {
    memory: Mutex<Vec<u8>>,
    position: Mutex<usize>,
    id: ArenaId,
}

impl Arena {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            memory: Mutex::new(vec![0u8; capacity]),
            position: Mutex::new(0),
            id: ArenaId::new(),
        })
    }

    pub fn alloc<T: super::Connectable>(&self, value: T) -> Result<ArenaRef<T>> {
        let serialized = serde_json::to_vec(&value)?;
        let size = serialized.len();

        let mut pos = self.position.lock();
        let mut mem = self.memory.lock();

        if *pos + size > mem.len() {
            return Err(ConnectorError::AllocationFailed(
                "Arena full".to_string(),
            ));
        }

        let offset = *pos;
        mem[offset..offset + size].copy_from_slice(&serialized);
        *pos += size;

        Ok(ArenaRef {
            arena_id: self.id,
            offset,
            size,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn capacity(&self) -> usize {
        self.memory.lock().len()
    }

    pub fn used(&self) -> usize {
        *self.position.lock()
    }

    pub fn available(&self) -> usize {
        self.capacity() - self.used()
    }

    pub fn id(&self) -> ArenaId {
        self.id
    }
}

pub struct ArenaRef<T> {
    arena_id: ArenaId,
    offset: usize,
    size: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for ArenaRef<T> {
    fn clone(&self) -> Self {
        Self {
            arena_id: self.arena_id,
            offset: self.offset,
            size: self.size,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for ArenaRef<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_creation() {
        let arena = Arena::new(1024);
        assert_eq!(arena.capacity(), 1024);
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.available(), 1024);
    }

    #[test]
    fn test_arena_id_unique() {
        let id1 = ArenaId::new();
        let id2 = ArenaId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_arena_id_default() {
        let id = ArenaId::default();
        assert_eq!(id, id);
    }
}
