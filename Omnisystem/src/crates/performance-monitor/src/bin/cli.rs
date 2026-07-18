//! CLI that exercises the performance monitor end to end.

use performance_monitor::{AlertManager, MetricsAggregator, PerformanceMonitor, ReportGenerator, SystemMetrics};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = PerformanceMonitor::new();
    monitor.record_metrics(SystemMetrics {
        name: "server1".to_string(),
        cpu_usage: 45.0,
        memory_usage: 60.0,
        disk_usage: 75.0,
        network_io: 1000,
        timestamp: 1000,
    })?;
    monitor.record_metrics(SystemMetrics {
        name: "server2".to_string(),
        cpu_usage: 85.0,
        memory_usage: 70.0,
        disk_usage: 85.0,
        network_io: 1500,
        timestamp: 1000,
    })?;

    println!(
        "Recorded {} metric(s), avg CPU {:.1}%, avg memory {:.1}%",
        monitor.metric_count(),
        monitor.get_avg_cpu(),
        monitor.get_avg_memory()
    );

    let alerts = AlertManager::new();
    let alert = alerts.check_alert(monitor.get_avg_cpu(), 80.0)?;
    println!("Alert triggered: {}", alert.threshold_exceeded);

    let samples = [40.0, 45.0, 85.0, 50.0, 55.0];
    let p95 = MetricsAggregator::calculate_percentile(&samples, 95)?;
    println!("p95 CPU: {p95}");

    let report = ReportGenerator::generate(&[(45.0, 60.0), (85.0, 70.0)]);
    println!("Report: {report:?}");

    Ok(())
}
