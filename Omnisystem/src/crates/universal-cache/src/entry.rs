//! Cache Entry Types

/// Represents a single cache entry with metadata
#[derive(Clone, Debug)]
pub struct Entry<V> {
    pub value: V,
    pub size: usize,
    pub flags: EntryFlags,
}

#[derive(Clone, Debug, Default)]
pub struct EntryFlags {
    pub dirty: bool,
    pub replicated: bool,
    pub compressed: bool,
}

impl<V> Entry<V> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            size: std::mem::size_of::<V>(),
            flags: EntryFlags::default(),
        }
    }
}
