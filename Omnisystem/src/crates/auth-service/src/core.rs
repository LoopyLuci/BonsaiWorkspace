use dashmap::DashMap;
use std::sync::Arc;

pub struct C { d: Arc<DashMap<String, String>> }

impl C {
    pub fn new() -> Self { Self { d: Arc::new(DashMap::new()) } }
    pub fn s(&self, k: String, v: String) { self.d.insert(k, v); }
    pub fn g(&self, k: &str) -> Option<String> { self.d.get(k).map(|v| v.clone()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create() { let c = C::new(); assert_eq!(c.g("k"), None); }
    #[test]
    fn test_set_get() { let c = C::new(); c.s("k".into(), "v".into()); assert_eq!(c.g("k"), Some("v".into())); }
}
