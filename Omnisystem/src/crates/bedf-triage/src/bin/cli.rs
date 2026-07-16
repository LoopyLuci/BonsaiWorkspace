//! CLI: run a small batch of crash reports through the triage pipeline
//! and print deduplication + fix-suggestion results.

use bedf_triage::{CrashReport, TriageConfig, TriageEngine};

fn main() {
    let reports = vec![
        CrashReport::new("crash-001", "thread panicked at 'index out of bounds'"),
        CrashReport::new("crash-002", "thread panicked at 'index out of bounds'"),
        CrashReport::new("crash-003", "null pointer dereference in handler"),
        CrashReport::new("crash-004", "deadlock detected between lock_a and lock_b"),
        CrashReport::new("crash-005", "unrecognized failure signature XYZ"),
    ];

    let mut engine = TriageEngine::new(TriageConfig::default());
    let results = engine.triage_batch(&reports);

    for result in &results {
        println!(
            "[{}] signature={} duplicate={} fix={}",
            result.report_id,
            &result.signature[..8],
            result.is_duplicate,
            result
                .suggested_fix
                .as_ref()
                .map(|f| f.description.as_str())
                .unwrap_or("none")
        );
    }

    println!(
        "\n{} reports processed, {} unique crashes, {} fixes suggested",
        results.len(),
        engine.unique_crash_count(),
        engine.fixes_generated()
    );
}
