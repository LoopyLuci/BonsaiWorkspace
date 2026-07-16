use bedf_sanitizers::{IssueType, MemoryTracker, SanitizerConfig, SanitizerReport};

#[test]
fn test_full_sanitizer_workflow() {
    let config = SanitizerConfig::default();
    assert!(config.enabled);
    assert!(config.enable_asan);

    let mut tracker = MemoryTracker::new();

    // Clean allocation lifecycle: no issues.
    tracker.track_allocation(0x1000, 64);
    tracker.track_access(0x1000, 64, false);
    tracker.track_deallocation(0x1000);
    assert!(tracker.get_issues().is_empty());

    // Use-after-free.
    tracker.track_access(0x1000, 16, true);

    // Buffer overflow on a separate allocation.
    tracker.track_allocation(0x2000, 16);
    tracker.track_access(0x2000, 64, true);

    let issues = tracker.get_issues();
    assert_eq!(issues.len(), 2);
    assert!(issues.iter().any(|i| i.issue_type == IssueType::UseAfterFree));
    assert!(issues.iter().any(|i| i.issue_type == IssueType::BufferOverflow));

    let report = SanitizerReport::from_issues(issues, 1.0);
    assert_eq!(report.total_issues, 2);
    assert_eq!(report.asan_issues, 2);
    assert_eq!(report.msan_issues, 0);
    assert!(report.summary().contains("2 issues"));
}
