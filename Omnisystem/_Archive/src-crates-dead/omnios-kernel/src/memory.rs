use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub address: u64,
    pub size: u64,
    pub allocated: bool,
}

pub struct MemoryManager {
    blocks: Arc<DashMap<u64, MemoryBlock>>,
    total_memory: u64,
}

impl MemoryManager {
    pub fn new(total: u64) -> Self {
        Self {
            blocks: Arc::new(DashMap::new()),
            total_memory: total,
        }
    }

    pub fn allocate(&self, size: u64) -> Option<u64> {
        let address = self.blocks.len() as u64 * 4096;
        if address + size <= self.total_memory {
            let block = MemoryBlock {
                address,
                size,
                allocated: true,
            };
            self.blocks.insert(address, block);
            Some(address)
        } else {
            None
        }
    }

    pub fn deallocate(&self, address: u64) -> bool {
        self.blocks.remove(&address).is_some()
    }

    pub fn get_block(&self, address: u64) -> Option<MemoryBlock> {
        self.blocks.get(&address).map(|b| b.clone())
    }

    pub fn allocated_blocks(&self) -> usize {
        self.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation() {
        let mm = MemoryManager::new(1024 * 1024);
        let addr = mm.allocate(4096).unwrap();
        assert!(addr >= 0);
    }

    #[test]
    fn test_memory_deallocation() {
        let mm = MemoryManager::new(1024 * 1024);
        let addr = mm.allocate(4096).unwrap();
        assert!(mm.deallocate(addr));
    }
}
