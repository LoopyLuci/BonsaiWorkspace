//! Persistent HashMap - HAMT (Hash Array Mapped Trie)

pub struct PersistentHashMap<K, V> {
    root: im::HashMap<K, V>,
}

impl<K: Clone, V: Clone> Clone for PersistentHashMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<K, V> std::fmt::Debug for PersistentHashMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentHashMap").finish()
    }
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> PersistentHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            root: im::HashMap::new(),
        }
    }

    pub fn insert(&self, key: K, value: V) -> Self {
        let mut new_root = self.root.clone();
        new_root.insert(key, value);
        Self { root: new_root }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.root.get(key)
    }

    pub fn remove(&self, key: &K) -> (Option<V>, Self) {
        let mut new_root = self.root.clone();
        let value = new_root.remove(key);
        (value, Self { root: new_root })
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.root.iter()
    }
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> Default for PersistentHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashmap_operations() {
        let m = PersistentHashMap::<String, i32>::new();
        assert_eq!(m.len(), 0);

        let m2 = m.insert("a".to_string(), 1);
        assert_eq!(m2.len(), 1);
        assert_eq!(m2.get(&"a".to_string()), Some(&1));

        let m3 = m2.insert("b".to_string(), 2);
        assert_eq!(m3.len(), 2);

        let (removed, m4) = m3.remove(&"a".to_string());
        assert_eq!(removed, Some(1));
        assert_eq!(m4.len(), 1);
    }
}
