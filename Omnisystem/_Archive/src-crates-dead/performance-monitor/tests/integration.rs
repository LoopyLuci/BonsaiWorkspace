use performance_monitor::*;

#[test]
fn test_full_monitoring_workflow() {
    let monitor = monitor::PerformanceMonitor::new();
    
    let metrics1 = monitor::SystemMetrics {
        name: "server1".to_string(),
        cpu_usage: 45.0,
        memory_usage: 60.0,
        disk_usage: 75.0,
        network_io: 1000,
        timestamp: 1000,
    };
    
    let metrics2 = monitor::SystemMetrics {
        name: "server2".to_string(),
        cpu_usage: 55.0,
        memory_usage: 70.0,
        disk_usage: 85.0,
        network_io: 1500,
        timestamp: 1000,
    };
    
    monitor.record_metrics(metrics1).unwrap();
    monitor.record_metrics(metrics2).unwrap();
    
    assert_eq!(monitor.metric_count(), 2);
    
    let avg_cpu = monitor.get_avg_cpu();
    assert!(avg_cpu > 0.0 && avg_cpu <= 100.0);
}
