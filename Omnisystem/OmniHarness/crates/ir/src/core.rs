use dashmap::DashMap;
use std::sync::Arc;

/// Generic in-memory key/value cache. Used by `IrCompiler` to memoize
/// generated Rust source for a given (module_name, source) pair, so
/// re-compiling identical source skips the parse + codegen pipeline.
#[derive(Clone)]
pub struct Core { data: Arc<DashMap<String, String>> }

impl Core {
    pub fn new() -> Self {
        Self { data: Arc::new(DashMap::new()) }
    }

    pub fn set(&self, k: String, v: String) {
        self.data.insert(k, v);
    }

    pub fn get(&self, k: &str) -> Option<String> {
        self.data.get(k).map(|v| v.clone())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_new() {
        let _c = Core::new();
    }

    #[test]
    fn test_core_set_get() {
        let c = Core::new();
        c.set("key".into(), "value".into());
        assert_eq!(c.get("key"), Some("value".into()));
    }
}
