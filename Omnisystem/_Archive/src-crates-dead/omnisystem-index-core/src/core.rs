use dashmap::DashMap;
use std::sync::Arc;

pub struct Core { state: Arc<DashMap<String, String>> }

impl Core {
  pub fn new() -> Self { Self { state: Arc::new(DashMap::new()) } }
  pub fn set(&self, k: String, v: String) { self.state.insert(k, v); }
  pub fn get(&self, k: &str) -> Option<String> { self.state.get(k).map(|v| v.clone()) }
  pub fn count(&self) -> usize { self.state.len() }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_set() {
    let c = Core::new();
    c.set("k".to_string(), "v".to_string());
    assert_eq!(c.count(), 1);
  }
  #[test]
  fn test_get() {
    let c = Core::new();
    c.set("k".to_string(), "v".to_string());
    assert_eq!(c.get("k"), Some("v".to_string()));
  }
}
