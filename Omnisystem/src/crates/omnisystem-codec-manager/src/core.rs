use dashmap::DashMap; use std::sync::Arc;
pub struct Core { data: Arc<DashMap<String, String>> }
impl Core {
  pub fn new() -> Self { Self { data: Arc::new(DashMap::new()) } }
  pub fn add(&self, k: String, v: String) { self.data.insert(k, v); }
  pub fn get(&self, k: &str) -> Option<String> { self.data.get(k).map(|v| v.clone()) }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_core() { let c = Core::new(); c.add("k".into(), "v".into()); assert_eq!(c.get("k"), Some("v".into())); }
  #[test]
  fn test_empty() { let c = Core::new(); assert_eq!(c.get("x"), None); }
}
