use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Block {
    pub address: u64,
    pub size: u64,
    pub allocated: bool,
}

pub struct MemoryAllocator {
    blocks: Arc<DashMap<u64, Block>>,
}

impl MemoryAllocator {
    pub fn new() -> Self {
        Self { blocks: Arc::new(DashMap::new()) }
    }
    
    pub fn allocate(&self, size: u64) -> Option<u64> {
        let address = self.blocks.len() as u64 * 4096;
        let block = Block { address, size, allocated: true };
        self.blocks.insert(address, block);
        Some(address)
    }
    
    pub fn deallocate(&self, address: u64) -> bool {
        self.blocks.remove(&address).is_some()
    }
    
    pub fn get_block(&self, address: u64) -> Option<Block> {
        self.blocks.get(&address).map(|b| b.clone())
    }
    
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocate() {
        let alloc = MemoryAllocator::new();
        let addr = alloc.allocate(4096);
        assert!(addr.is_some());
    }
    
    #[test]
    fn test_deallocate() {
        let alloc = MemoryAllocator::new();
        let addr = alloc.allocate(4096).unwrap();
        assert!(alloc.deallocate(addr));
    }
}
