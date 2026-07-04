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
