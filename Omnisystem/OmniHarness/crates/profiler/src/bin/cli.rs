//! CLI — record synthetic allocations/operations and print a profiling summary.

use profiler::{AggregateMetrics, AllocationTracker, HotPathDetector, OperationMetrics};

fn main() {
    let allocator = AllocationTracker::new();
    allocator.record_allocation(1024);
    allocator.record_allocation(4096);
    allocator.record_deallocation(1024);

    println!(
        "Allocations: current={}B peak={}B total_allocs={} total_deallocs={} efficiency={:.2}",
        allocator.current_bytes(),
        allocator.peak_bytes(),
        allocator.total_allocations(),
        allocator.total_deallocations(),
        allocator.efficiency()
    );

    let hotpaths = HotPathDetector::new();
    hotpaths.record("db::query", 25.0);
    hotpaths.record("db::query", 30.0);
    hotpaths.record("cache::get", 2.0);

    println!("Hottest paths:");
    for (path, total_ms) in hotpaths.get_hottest_paths(5) {
        println!("  {path}: {total_ms:.1}ms total");
    }

    let now = chrono::Utc::now();
    let ops = vec![
        OperationMetrics { operation: "db::query".into(), latency_ms: 25.0, memory_mb: 1.2, timestamp: now },
        OperationMetrics { operation: "db::query".into(), latency_ms: 30.0, memory_mb: 1.5, timestamp: now },
        OperationMetrics { operation: "db::query".into(), latency_ms: 18.0, memory_mb: 1.1, timestamp: now },
    ];
    let aggregate = AggregateMetrics::from_operations("db::query", &ops);
    println!(
        "db::query: count={} avg={:.1}ms p99={:.1}ms",
        aggregate.count, aggregate.avg_time_ms, aggregate.percentiles.p99
    );
}
