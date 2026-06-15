use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Item { pub id: String, pub data: String }

pub struct Manager { items: Arc<DashMap<String, Item>> }

impl Manager {
  pub fn new() -> Self { Self { items: Arc::new(DashMap::new()) } }
  pub fn add(&self, id: String, data: String) { 
    self.items.insert(id.clone(), Item { id, data }); 
  }
  pub fn get(&self, id: &str) -> Option<Item> { 
    self.items.get(id).map(|i| i.clone()) 
  }
  pub fn remove(&self, id: &str) -> bool { 
    self.items.remove(id).is_some() 
  }
  pub fn count(&self) -> usize { self.items.len() }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_add() {
    let m = Manager::new();
    m.add("id1".to_string(), "data1".to_string());
    assert_eq!(m.count(), 1);
  }
  #[test]
  fn test_remove() {
    let m = Manager::new();
    m.add("id1".to_string(), "data1".to_string());
    assert!(m.remove("id1"));
  }
}
