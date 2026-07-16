//! CLI that exercises the metrics collector, aggregation, and dashboard.

use omnisystem_analytics::{Dashboard, MetricPoint, MetricType, MetricsCollector};

fn main() {
    let collector = MetricsCollector::new();
    for value in [42.0, 55.0, 38.0, 61.0] {
        collector.record(MetricPoint::new(MetricType::CpuUsage, value));
    }

    if let Some(aggregated) = collector.get_aggregated(MetricType::CpuUsage) {
        println!(
            "CPU usage over {} samples: min={:.1} max={:.1} avg={:.1}",
            aggregated.count, aggregated.min, aggregated.max, aggregated.avg
        );
    }

    println!("{}", Dashboard::new().generate_report());
}
