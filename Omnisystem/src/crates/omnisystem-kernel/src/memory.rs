use crate::KernelError;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::info;

pub const PAGE_SIZE: usize = 4096;
pub const MAX_PAGES: usize = 1_000_000_000; // 4TB virtual memory

/// Physical page frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageFrame(u64);

impl PageFrame {
    pub fn new(frame_number: u64) -> Self {
        PageFrame(frame_number)
    }

    pub fn physical_address(&self) -> u64 {
        self.0 * PAGE_SIZE as u64
    }
}

/// Virtual memory page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPage(u64);

impl VirtualPage {
    pub fn new(page_number: u64) -> Self {
        VirtualPage(page_number)
    }

    pub fn virtual_address(&self) -> u64 {
        self.0 * PAGE_SIZE as u64
    }
}

/// Page table entry (flags)
#[derive(Debug, Clone, Copy)]
pub struct PageFlags {
    pub present: bool,
    pub writable: bool,
    pub executable: bool,
    pub cached: bool,
    pub user_accessible: bool,
}

impl Default for PageFlags {
    fn default() -> Self {
        PageFlags {
            present: true,
            writable: true,
            executable: false,
            cached: true,
            user_accessible: false,
        }
    }
}

/// Page table entry
#[derive(Debug, Clone)]
pub struct PageTableEntry {
    pub frame: Option<PageFrame>,
    pub flags: PageFlags,
}

/// Virtual address space
pub struct AddressSpace {
    page_tables: RwLock<BTreeMap<VirtualPage, PageTableEntry>>,
}

impl AddressSpace {
    pub fn new() -> Self {
        AddressSpace {
            page_tables: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn map_page(
        &self,
        virtual_page: VirtualPage,
        physical_frame: PageFrame,
        flags: PageFlags,
    ) -> Result<(), KernelError> {
        let mut tables = self.page_tables.write();

        if tables.contains_key(&virtual_page) {
            return Err(KernelError::MemoryError(
                "Virtual page already mapped".to_string(),
            ));
        }

        tables.insert(
            virtual_page,
            PageTableEntry {
                frame: Some(physical_frame),
                flags,
            },
        );

        Ok(())
    }

    pub fn unmap_page(&self, virtual_page: VirtualPage) -> Result<(), KernelError> {
        let mut tables = self.page_tables.write();
        tables.remove(&virtual_page);
        Ok(())
    }

    pub fn get_page(&self, virtual_page: VirtualPage) -> Option<PageTableEntry> {
        self.page_tables.read().get(&virtual_page).cloned()
    }
}

/// Physical memory manager
pub struct MemoryManager {
    free_frames: RwLock<Vec<PageFrame>>,
    total_frames: u64,
    allocated_frames: RwLock<u64>,
}

impl MemoryManager {
    pub fn new() -> Result<Self, KernelError> {
        info!("Initializing memory manager (4GB addressable)");

        // Assume 4GB physical RAM for now
        let total_frames = (4 * 1024 * 1024 * 1024) / PAGE_SIZE as u64;

        let free_frames = (0..total_frames).rev().map(PageFrame::new).collect();

        Ok(MemoryManager {
            free_frames: RwLock::new(free_frames),
            total_frames,
            allocated_frames: RwLock::new(0),
        })
    }

    pub fn allocate_page(&self) -> Result<PageFrame, KernelError> {
        let mut frames = self.free_frames.write();

        match frames.pop() {
            Some(frame) => {
                *self.allocated_frames.write() += 1;
                Ok(frame)
            }
            None => Err(KernelError::MemoryError("Out of physical memory".to_string())),
        }
    }

    pub fn allocate_pages(&self, count: usize) -> Result<Vec<PageFrame>, KernelError> {
        let mut frames = self.free_frames.write();

        if frames.len() < count {
            return Err(KernelError::MemoryError(
                "Insufficient physical memory".to_string(),
            ));
        }

        let start_idx = frames.len() - count;
        let allocated: Vec<PageFrame> = frames
            .drain(start_idx..)
            .collect();

        *self.allocated_frames.write() += count as u64;

        Ok(allocated)
    }

    pub fn deallocate_page(&self, frame: PageFrame) -> Result<(), KernelError> {
        self.free_frames.write().push(frame);
        *self.allocated_frames.write() -= 1;
        Ok(())
    }

    pub fn get_stats(&self) -> MemoryStats {
        let allocated = *self.allocated_frames.read();
        MemoryStats {
            total_frames: self.total_frames,
            allocated_frames: allocated,
            free_frames: self.total_frames - allocated,
            total_memory_bytes: self.total_frames * PAGE_SIZE as u64,
            allocated_memory_bytes: allocated * PAGE_SIZE as u64,
            free_memory_bytes: (self.total_frames - allocated) * PAGE_SIZE as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_frames: u64,
    pub allocated_frames: u64,
    pub free_frames: u64,
    pub total_memory_bytes: u64,
    pub allocated_memory_bytes: u64,
    pub free_memory_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let mm = MemoryManager::new();
        assert!(mm.is_ok());
    }

    #[test]
    fn test_page_allocation() {
        let mm = MemoryManager::new().unwrap();
        let frame = mm.allocate_page();
        assert!(frame.is_ok());
    }

    #[test]
    fn test_address_space() {
        let as_ = AddressSpace::new();
        let vpage = VirtualPage::new(0);
        let pframe = PageFrame::new(0);
        let flags = PageFlags::default();

        let result = as_.map_page(vpage, pframe, flags);
        assert!(result.is_ok());

        let entry = as_.get_page(vpage);
        assert!(entry.is_some());
    }
}
