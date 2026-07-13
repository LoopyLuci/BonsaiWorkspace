use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AllocationRecord {
    pub ptr: u64,
    pub size: usize,
    pub is_allocated: bool,
}

pub struct MemoryTracker {
    allocations: HashMap<u64, AllocationRecord>,
    accesses: Vec<AccessRecord>,
}

#[derive(Debug, Clone)]
pub struct AccessRecord {
    pub ptr: u64,
    pub size: usize,
    pub is_write: bool,
    pub timestamp: u64,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            accesses: Vec::new(),
        }
    }

    pub fn track_allocation(&mut self, ptr: u64, size: usize) {
        self.allocations.insert(
            ptr,
            AllocationRecord {
                ptr,
                size,
                is_allocated: true,
            },
        );
    }

    pub fn track_deallocation(&mut self, ptr: u64) {
        if let Some(record) = self.allocations.get_mut(&ptr) {
            record.is_allocated = false;
        }
    }

    pub fn track_access(&mut self, ptr: u64, size: usize, is_write: bool) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        self.accesses.push(AccessRecord {
            ptr,
            size,
            is_write,
            timestamp,
        });
    }

    pub fn get_issues(&self) -> Vec<super::MemoryIssue> {
        let mut issues = Vec::new();

        // Check for use-after-free
        for access in &self.accesses {
            if let Some(record) = self.allocations.get(&access.ptr) {
                if !record.is_allocated {
                    issues.push(super::MemoryIssue {
                        issue_type: super::IssueType::UseAfterFree,
                        address: access.ptr,
                        size: access.size,
                        description: "Use after free detected".to_string(),
                    });
                }
            }
        }

        // Check for buffer overflow
        for access in &self.accesses {
            if let Some(record) = self.allocations.get(&access.ptr) {
                if access.size > record.size {
                    issues.push(super::MemoryIssue {
                        issue_type: super::IssueType::BufferOverflow,
                        address: access.ptr,
                        size: access.size,
                        description: format!(
                            "Buffer overflow: accessed {} bytes but allocated {}",
                            access.size, record.size
                        ),
                    });
                }
            }
        }

        issues
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = MemoryTracker::new();
        assert_eq!(tracker.allocations.len(), 0);
    }

    #[test]
    fn test_track_allocation() {
        let mut tracker = MemoryTracker::new();
        tracker.track_allocation(0x1000, 100);
        assert_eq!(tracker.allocations.len(), 1);
    }

    #[test]
    fn test_track_deallocation() {
        let mut tracker = MemoryTracker::new();
        tracker.track_allocation(0x1000, 100);
        tracker.track_deallocation(0x1000);

        let record = tracker.allocations.get(&0x1000).unwrap();
        assert!(!record.is_allocated);
    }

    #[test]
    fn test_use_after_free_detection() {
        let mut tracker = MemoryTracker::new();
        tracker.track_allocation(0x1000, 100);
        tracker.track_deallocation(0x1000);
        tracker.track_access(0x1000, 50, true);

        let issues = tracker.get_issues();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_buffer_overflow_detection() {
        let mut tracker = MemoryTracker::new();
        tracker.track_allocation(0x1000, 100);
        tracker.track_access(0x1000, 200, true); // Access 200 bytes but only allocated 100

        let issues = tracker.get_issues();
        assert!(!issues.is_empty());
    }
}
