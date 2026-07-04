/// Resource Manager
/// Cross-system resource allocation and management

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Resource Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    CpuCores,
    MemoryMb,
    DiskGb,
    NetworkBandwidth,
    FileDescriptors,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::CpuCores => write!(f, "cpu_cores"),
            ResourceType::MemoryMb => write!(f, "memory_mb"),
            ResourceType::DiskGb => write!(f, "disk_gb"),
            ResourceType::NetworkBandwidth => write!(f, "network_bandwidth"),
            ResourceType::FileDescriptors => write!(f, "file_descriptors"),
        }
    }
}

/// Resource Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub request_id: String,
    pub requesting_system: String,
    pub resource_type: ResourceType,
    pub amount: u64,
    pub priority: u8,
    pub timeout_ms: u64,
}

impl ResourceRequest {
    pub fn new(
        requesting_system: String,
        resource_type: ResourceType,
        amount: u64,
    ) -> Self {
        ResourceRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            requesting_system,
            resource_type,
            amount,
            priority: 5,
            timeout_ms: 5000,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Resource Allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub allocation_id: String,
    pub request_id: String,
    pub system: String,
    pub resource_type: ResourceType,
    pub allocated: u64,
    pub status: AllocationStatus,
}

/// Allocation Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStatus {
    Pending,
    Granted,
    Denied,
    Revoked,
}

/// Resource Pool
#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub resource_type: ResourceType,
    pub total: u64,
    pub allocated: u64,
    pub reserved: u64,
}

impl ResourcePool {
    pub fn new(resource_type: ResourceType, total: u64) -> Self {
        ResourcePool {
            resource_type,
            total,
            allocated: 0,
            reserved: 0,
        }
    }

    pub fn available(&self) -> u64 {
        self.total.saturating_sub(self.allocated + self.reserved)
    }

    pub fn can_allocate(&self, amount: u64) -> bool {
        self.available() >= amount
    }
}

/// Resource Manager
pub struct ResourceManager {
    pools: Arc<DashMap<ResourceType, ResourcePool>>,
    allocations: Arc<DashMap<String, ResourceAllocation>>,
    stats: Arc<DashMap<String, u64>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let manager = ResourceManager {
            pools: Arc::new(DashMap::new()),
            allocations: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
        };

        // Initialize default pools
        manager.pools.insert(ResourceType::CpuCores, ResourcePool::new(ResourceType::CpuCores, 64));
        manager.pools.insert(ResourceType::MemoryMb, ResourcePool::new(ResourceType::MemoryMb, 1024 * 64)); // 64GB
        manager.pools.insert(ResourceType::DiskGb, ResourcePool::new(ResourceType::DiskGb, 10 * 1024)); // 10TB
        manager.pools.insert(ResourceType::NetworkBandwidth, ResourcePool::new(ResourceType::NetworkBandwidth, 10000)); // 10Gbps
        manager.pools.insert(ResourceType::FileDescriptors, ResourcePool::new(ResourceType::FileDescriptors, 1000000));

        manager
    }

    pub async fn allocate(&self, request: ResourceRequest) -> anyhow::Result<ResourceAllocation> {
        if let Some(mut pool) = self.pools.get_mut(&request.resource_type) {
            if pool.can_allocate(request.amount) {
                pool.allocated += request.amount;

                let allocation = ResourceAllocation {
                    allocation_id: uuid::Uuid::new_v4().to_string(),
                    request_id: request.request_id.clone(),
                    system: request.requesting_system.clone(),
                    resource_type: request.resource_type,
                    allocated: request.amount,
                    status: AllocationStatus::Granted,
                };

                self.allocations.insert(allocation.allocation_id.clone(), allocation.clone());
                self.stats
                    .entry(format!("allocated_{}", request.resource_type))
                    .and_modify(|c| *c += request.amount)
                    .or_insert(request.amount);

                Ok(allocation)
            } else {
                Err(anyhow::anyhow!(
                    "Insufficient resources: need {}, available {}",
                    request.amount,
                    pool.available()
                ))
            }
        } else {
            Err(anyhow::anyhow!("Unknown resource type"))
        }
    }

    pub fn deallocate(&self, allocation_id: &str) -> anyhow::Result<()> {
        if let Some((_, allocation)) = self.allocations.remove(allocation_id) {
            if let Some(mut pool) = self.pools.get_mut(&allocation.resource_type) {
                pool.allocated = pool.allocated.saturating_sub(allocation.allocated);
                self.stats
                    .entry(format!("released_{}", allocation.resource_type))
                    .and_modify(|c| *c += allocation.allocated)
                    .or_insert(allocation.allocated);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Resource pool not found"))
            }
        } else {
            Err(anyhow::anyhow!("Allocation not found"))
        }
    }

    pub fn get_pool_status(&self, resource_type: ResourceType) -> Option<ResourcePool> {
        self.pools.get(&resource_type).map(|pool| pool.clone())
    }

    pub fn list_allocations(&self, system: &str) -> Vec<ResourceAllocation> {
        self.allocations
            .iter()
            .filter(|entry| entry.value().system == system)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn cleanup(&self) -> anyhow::Result<()> {
        self.allocations.clear();
        Ok(())
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_allocation() {
        let manager = ResourceManager::new();
        let request = ResourceRequest::new(
            "sys_1".to_string(),
            ResourceType::CpuCores,
            4,
        );

        let allocation = manager.allocate(request).await.unwrap();
        assert_eq!(allocation.allocated, 4);
        assert_eq!(allocation.status, AllocationStatus::Granted);
    }

    #[tokio::test]
    async fn test_resource_deallocation() {
        let manager = ResourceManager::new();
        let request = ResourceRequest::new(
            "sys_1".to_string(),
            ResourceType::CpuCores,
            4,
        );

        let allocation = manager.allocate(request).await.unwrap();
        assert!(manager.deallocate(&allocation.allocation_id).is_ok());
    }

    #[test]
    fn test_pool_status() {
        let manager = ResourceManager::new();
        let pool = manager.get_pool_status(ResourceType::CpuCores).unwrap();
        assert_eq!(pool.total, 64);
        assert_eq!(pool.allocated, 0);
    }
}
