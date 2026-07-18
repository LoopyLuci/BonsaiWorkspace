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
    /// Whether this access touched a pointer that was already freed, and if
    /// so by how many bytes it overran its allocation (if at all). Captured
    /// at the moment of the access rather than re-derived later, so a
    /// pointer's later deallocation can't retroactively flag earlier,
    /// perfectly valid accesses as use-after-free.
    use_after_free: bool,
    overflow_by: Option<usize>,
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

        // Snapshot validity against the allocation table *now*, at the
        // moment of the access, not later when issues are queried.
        let (use_after_free, overflow_by) = match self.allocations.get(&ptr) {
            Some(record) if !record.is_allocated => (true, None),
            Some(record) if size > record.size => (false, Some(size - record.size)),
            _ => (false, None),
        };

        self.accesses.push(AccessRecord {
            ptr,
            size,
            is_write,
            timestamp,
            use_after_free,
            overflow_by,
        });
    }

    pub fn get_issues(&self) -> Vec<super::MemoryIssue> {
        let mut issues = Vec::new();

        for access in &self.accesses {
            if access.use_after_free {
                issues.push(super::MemoryIssue {
                    issue_type: super::IssueType::UseAfterFree,
                    address: access.ptr,
                    size: access.size,
                    description: "Use after free detected".to_string(),
                });
            }

            if let Some(allocated_size) = access.overflow_by.map(|over| access.size - over) {
                issues.push(super::MemoryIssue {
                    issue_type: super::IssueType::BufferOverflow,
                    address: access.ptr,
                    size: access.size,
                    description: format!(
                        "Buffer overflow: accessed {} bytes but allocated {}",
                        access.size, allocated_size
                    ),
                });
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

    #[test]
    fn test_valid_access_not_retroactively_flagged_after_later_free() {
        // Regression test: a legitimate access that happened *while the
        // pointer was still allocated* must not turn into a false
        // use-after-free report just because the pointer is freed later.
        let mut tracker = MemoryTracker::new();
        tracker.track_allocation(0x1000, 100);
        tracker.track_access(0x1000, 50, false); // valid, allocated at the time
        tracker.track_deallocation(0x1000);

        let issues = tracker.get_issues();
        assert!(
            issues.is_empty(),
            "a pre-free access should never be reported as use-after-free: {:?}",
            issues
        );
    }
}
