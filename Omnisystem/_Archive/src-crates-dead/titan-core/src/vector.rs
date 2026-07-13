//! Persistent Vector - O(log32 n) access and update

pub struct PersistentVector<T> {
    root: im::Vector<T>,
}

impl<T: Clone> Clone for PersistentVector<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<T> std::fmt::Debug for PersistentVector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentVector").finish()
    }
}

impl<T: Clone> PersistentVector<T> {
    pub fn new() -> Self {
        Self {
            root: im::Vector::new(),
        }
    }

    pub fn push(&self, value: T) -> Self {
        let mut new_root = self.root.clone();
        new_root.push_back(value);
        Self { root: new_root }
    }

    pub fn pop(&self) -> (Option<T>, Self) {
        let mut new_root = self.root.clone();
        let value = new_root.pop_back();
        (value, Self { root: new_root })
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.root.get(index)
    }

    pub fn set(&self, index: usize, value: T) -> Option<Self> {
        if index >= self.root.len() {
            return None;
        }
        let mut new_root = self.root.clone();
        new_root[index] = value;
        Some(Self { root: new_root })
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.root.iter()
    }
}

impl<T: Clone> Default for PersistentVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_operations() {
        let v = PersistentVector::<i32>::new();
        assert_eq!(v.len(), 0);

        let v2 = v.push(1);
        assert_eq!(v2.len(), 1);
        assert_eq!(v2.get(0), Some(&1));

        let v3 = v2.push(2).push(3);
        assert_eq!(v3.len(), 3);

        let (popped, v4) = v3.pop();
        assert_eq!(popped, Some(3));
        assert_eq!(v4.len(), 2);
    }

    #[test]
    fn test_vector_immutability() {
        let v1 = PersistentVector::new().push(1).push(2);
        let v2 = v1.push(3);

        assert_eq!(v1.len(), 2);
        assert_eq!(v2.len(), 3);
    }
}
