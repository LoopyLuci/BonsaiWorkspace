use dashmap::DashMap;
use std::sync::Arc;

pub struct Core { data: Arc<DashMap<String, String>> }

impl Core {
    pub fn new() -> Self {
        Self { data: Arc::new(DashMap::new()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_core() { let _ = Core::new(); }
}
