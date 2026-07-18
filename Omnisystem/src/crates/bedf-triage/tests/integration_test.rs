use bedf_triage::{CrashReport, TriageConfig, TriageEngine};

#[test]
fn end_to_end_triage_pipeline_dedups_and_suggests_fixes() {
    let mut engine = TriageEngine::new(TriageConfig::default());

    let reports = vec![
        CrashReport::new("a", "index out of bounds"),
        CrashReport::new("b", "index out of bounds"),
        CrashReport::new("c", "index out of bounds"),
        CrashReport::new("d", "use after free detected"),
    ];

    let results = engine.triage_batch(&reports);

    assert_eq!(results.len(), 4);
    assert!(!results[0].is_duplicate);
    assert!(results[1].is_duplicate);
    assert!(results[2].is_duplicate);
    assert!(!results[3].is_duplicate);

    // Only two distinct crash signatures were ever seen.
    assert_eq!(engine.unique_crash_count(), 2);

    // Fixes are only generated for the first occurrence of each unique crash.
    assert_eq!(engine.fixes_generated(), 2);
    assert!(results[0].suggested_fix.is_some());
    assert!(results[3].suggested_fix.is_some());
}
