/// Memory Management System
/// Virtual memory, paging, and protection across all platforms

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Memory Protection Flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryProtection {
    None,
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    WriteExecute,
    ReadWriteExecute,
}

impl MemoryProtection {
    pub fn allows_read(&self) -> bool {
        matches!(self, MemoryProtection::Read | MemoryProtection::ReadWrite |
                 MemoryProtection::ReadExecute | MemoryProtection::ReadWriteExecute)
    }

    pub fn allows_write(&self) -> bool {
        matches!(self, MemoryProtection::Write | MemoryProtection::ReadWrite |
                 MemoryProtection::WriteExecute | MemoryProtection::ReadWriteExecute)
    }

    pub fn allows_execute(&self) -> bool {
        matches!(self, MemoryProtection::Execute | MemoryProtection::ReadExecute |
                 MemoryProtection::WriteExecute | MemoryProtection::ReadWriteExecute)
    }
}

/// Virtual Memory Page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub address: u64,
    pub size: usize,
    pub protection: MemoryProtection,
    pub owner_pid: u64,
    pub data: Vec<u8>,
    pub dirty: bool,
}

impl Page {
    pub fn new(address: u64, size: usize, owner_pid: u64) -> Self {
        Page {
            address,
            size,
            protection: MemoryProtection::ReadWrite,
            owner_pid,
            data: vec![0u8; size],
            dirty: false,
        }
    }

    pub fn with_protection(mut self, protection: MemoryProtection) -> Self {
        self.protection = protection;
        self
    }
}

/// Page Table - Maps virtual to physical addresses
#[derive(Clone)]
pub struct PageTable {
    pages: Arc<DashMap<u64, Page>>,
}

impl PageTable {
    pub fn new() -> Self {
        PageTable {
            pages: Arc::new(DashMap::new()),
        }
    }

    pub fn allocate(&self, address: u64, size: usize, owner_pid: u64) -> anyhow::Result<Page> {
        if self.pages.contains_key(&address) {
            return Err(anyhow::anyhow!("Address already allocated: {:x}", address));
        }

        let page = Page::new(address, size, owner_pid);
        self.pages.insert(address, page.clone());
        Ok(page)
    }

    pub fn get(&self, address: u64) -> Option<Page> {
        self.pages.get(&address).map(|p| p.clone())
    }

    pub fn deallocate(&self, address: u64) -> anyhow::Result<()> {
        self.pages.remove(&address)
            .ok_or_else(|| anyhow::anyhow!("Page not found: {:x}", address))?;
        Ok(())
    }

    pub fn protect(&self, address: u64, protection: MemoryProtection) -> anyhow::Result<()> {
        match self.pages.get_mut(&address) {
            Some(mut page) => {
                page.protection = protection;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Page not found: {:x}", address)),
        }
    }

    pub fn list_pages(&self) -> Vec<Page> {
        self.pages.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn list_pages_for_process(&self, pid: u64) -> Vec<Page> {
        self.pages
            .iter()
            .filter(|entry| entry.value().owner_pid == pid)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory Manager - Overall memory system
pub struct MemoryManager {
    page_tables: Arc<DashMap<u64, PageTable>>,
    total_allocated: Arc<std::sync::atomic::AtomicUsize>,
    max_memory: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        MemoryManager {
            page_tables: Arc::new(DashMap::new()),
            total_allocated: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            max_memory: 4 * 1024 * 1024 * 1024, // 4GB default
        }
    }

    pub fn with_max_memory(mut self, max_bytes: usize) -> Self {
        self.max_memory = max_bytes;
        self
    }

    /// Get or create page table for process
    fn get_or_create_page_table(&self, pid: u64) -> PageTable {
        if let Some(pt) = self.page_tables.get(&pid) {
            pt.clone()
        } else {
            let pt = PageTable::new();
            self.page_tables.insert(pid, pt.clone());
            pt
        }
    }

    /// Allocate memory for a process
    pub fn allocate(&self, pid: u64, address: u64, size: usize, protection: MemoryProtection) -> anyhow::Result<Page> {
        // Check total allocation limit
        let current = self.total_allocated.load(std::sync::atomic::Ordering::SeqCst);
        if current + size > self.max_memory {
            return Err(anyhow::anyhow!("Memory allocation would exceed limit"));
        }

        let pt = self.get_or_create_page_table(pid);
        let page = pt.allocate(address, size, pid)?;

        self.total_allocated.fetch_add(size, std::sync::atomic::Ordering::SeqCst);
        tracing::debug!("Allocated {} bytes at {:x} for process {}", size, address, pid);

        Ok(page)
    }

    /// Deallocate memory
    pub fn deallocate(&self, pid: u64, address: u64, size: usize) -> anyhow::Result<()> {
        if let Some(pt) = self.page_tables.get(&pid) {
            pt.deallocate(address)?;
            self.total_allocated.fetch_sub(size, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!("Deallocated {} bytes at {:x} for process {}", size, address, pid);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No page table for process: {}", pid))
        }
    }

    /// Protect memory region
    pub fn protect(&self, pid: u64, address: u64, protection: MemoryProtection) -> anyhow::Result<()> {
        if let Some(pt) = self.page_tables.get(&pid) {
            pt.protect(address, protection)
        } else {
            Err(anyhow::anyhow!("No page table for process: {}", pid))
        }
    }

    /// Get memory stats
    pub fn stats(&self) -> MemoryStats {
        let allocated = self.total_allocated.load(std::sync::atomic::Ordering::SeqCst);
        MemoryStats {
            allocated,
            max_memory: self.max_memory,
            free: self.max_memory - allocated,
            pages: self.page_tables.len(),
        }
    }

    /// Cleanup memory for terminated process
    pub async fn cleanup_process(&self, pid: u64) -> anyhow::Result<()> {
        if let Some((_, pt)) = self.page_tables.remove(&pid) {
            let pages = pt.list_pages();
            let freed: usize = pages.iter().map(|p| p.size).sum();
            self.total_allocated.fetch_sub(freed, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!("Cleaned up {} bytes for process {}", freed, pid);
        }
        Ok(())
    }

    /// System-wide memory cleanup
    pub async fn cleanup(&self) -> anyhow::Result<()> {
        // Mark pages as clean, trigger garbage collection if needed
        for entry in self.page_tables.iter() {
            for page in entry.value().list_pages() {
                if page.dirty {
                    // In a real system, would flush to disk or persist
                    tracing::trace!("Dirty page at {:x} for process {}", page.address, page.owner_pid);
                }
            }
        }
        Ok(())
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory Statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub allocated: usize,
    pub max_memory: usize,
    pub free: usize,
    pub pages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation() {
        let manager = MemoryManager::new();
        let page = manager.allocate(1, 0x1000, 4096, MemoryProtection::ReadWrite).unwrap();

        assert_eq!(page.address, 0x1000);
        assert_eq!(page.size, 4096);
        assert_eq!(page.owner_pid, 1);
    }

    #[test]
    fn test_protection() {
        let protection = MemoryProtection::ReadExecute;
        assert!(protection.allows_read());
        assert!(!protection.allows_write());
        assert!(protection.allows_execute());
    }

    #[test]
    fn test_memory_limit() {
        let manager = MemoryManager::new().with_max_memory(8192);
        let page1 = manager.allocate(1, 0x1000, 4096, MemoryProtection::ReadWrite);
        assert!(page1.is_ok());

        let page2 = manager.allocate(1, 0x2000, 4096, MemoryProtection::ReadWrite);
        assert!(page2.is_ok());

        let page3 = manager.allocate(1, 0x3000, 4096, MemoryProtection::ReadWrite);
        assert!(page3.is_err()); // Should exceed limit
    }

    #[test]
    fn test_memory_stats() {
        let manager = MemoryManager::new().with_max_memory(1024 * 1024);
        manager.allocate(1, 0x1000, 4096, MemoryProtection::ReadWrite).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.allocated, 4096);
        assert_eq!(stats.free, 1024 * 1024 - 4096);
    }
}
