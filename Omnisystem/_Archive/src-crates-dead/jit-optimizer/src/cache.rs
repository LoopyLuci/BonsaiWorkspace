use std::sync::Arc;
use dashmap::DashMap;
use core_ir::LairNode;

pub struct CodeCache {
    entries: Arc<DashMap<blake3::Hash, Vec<LairNode>>>,
}

impl CodeCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }
    
    pub fn get(&self, hash: &blake3::Hash) -> Option<Vec<LairNode>> {
        self.entries.get(hash).map(|e| e.clone())
    }
    
    pub fn put(&self, hash: blake3::Hash, body: Vec<LairNode>) {
        self.entries.insert(hash, body);
    }
}
