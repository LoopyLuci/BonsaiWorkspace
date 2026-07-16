//! Memory management for snapshots

use crate::error::Result;

/// Memory region for snapshot
#[derive(Clone, Debug)]
pub struct MemoryRegion {
    pub address: u64,
    pub size: u64,
    pub data: Vec<u8>,
}

impl MemoryRegion {
    pub fn new(address: u64, size: u64) -> Self {
        Self {
            address,
            size,
            data: vec![0u8; size as usize],
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.data.clone())
    }
}

/// Memory manager for vault snapshots
pub struct MemoryManager {
    regions: Vec<MemoryRegion>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn add_region(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }

    pub fn serialize_all(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        for region in &self.regions {
            data.extend_from_slice(&region.serialize()?);
        }
        Ok(data)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_zero_initialized() {
        let region = MemoryRegion::new(0x1000, 16);
        assert_eq!(region.data.len(), 16);
        assert!(region.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_memory_manager_serialize_all_concatenates_regions() {
        let mut mgr = MemoryManager::new();
        mgr.add_region(MemoryRegion::new(0, 4));
        let mut second = MemoryRegion::new(4, 4);
        second.data = vec![1, 2, 3, 4];
        mgr.add_region(second);

        let serialized = mgr.serialize_all().unwrap();
        assert_eq!(serialized.len(), 8);
        assert_eq!(&serialized[4..], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_memory_manager_default_has_no_data() {
        let mgr = MemoryManager::default();
        assert_eq!(mgr.serialize_all().unwrap(), Vec::<u8>::new());
    }
}
