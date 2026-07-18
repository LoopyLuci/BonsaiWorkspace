//! bedf-sanitizers CLI: feeds a handful of allocation/access events into a
//! MemoryTracker (one clean pair, one use-after-free, one buffer overflow),
//! then rolls the detected issues up into a SanitizerReport.

use bedf_sanitizers::{MemoryTracker, SanitizerReport};

fn main() {
    let mut tracker = MemoryTracker::new();

    // A clean allocate/access/free -- should not raise any issue.
    tracker.track_allocation(0x1000, 64);
    tracker.track_access(0x1000, 32, false);
    tracker.track_deallocation(0x1000);

    // Use-after-free: access after deallocation.
    tracker.track_access(0x1000, 16, true);

    // Buffer overflow: access larger than the allocation.
    tracker.track_allocation(0x2000, 32);
    tracker.track_access(0x2000, 128, true);

    let issues = tracker.get_issues();
    let report = SanitizerReport::from_issues(issues, 0.05);

    println!("{}", report.summary());
    for issue in &report.issues {
        println!("  {:?} at 0x{:x}: {}", issue.issue_type, issue.address, issue.description);
    }
}
