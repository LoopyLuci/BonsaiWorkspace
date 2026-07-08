use observability::{
    MetricsCollector, SLATracker, SLATarget, AlertEngine, AlertRule, AlertSeverity,
    AlertComparison, DashboardConfig, ObservabilityStack,
};

#[test]
fn test_metrics_collection_integration() {
    let collector = MetricsCollector::new();

    // Simulate operations
    collector.record("api_call", 45.5, true);
    collector.record("api_call", 52.3, true);
    collector.record("api_call", 48.2, false);

    // Verify counters
    assert_eq!(
        collector.get_counter("operations_total{operation=\"api_call\"}"),
        Some(3)
    );
    assert_eq!(
        collector.get_counter("errors_total{operation=\"api_call\"}"),
        Some(1)
    );
}

#[test]
fn test_sla_tracking_integration() {
    let target = SLATarget {
        p95_latency_ms: 100.0,
        p99_latency_ms: 200.0,
        availability_percent: 99.95,
    };
    let tracker = SLATracker::new(target);

    // Record observations within SLA
    for i in 0..95 {
        tracker.record("request", (i as f64) * 0.5 + 10.0, true);
    }

    // Record a few observations exceeding SLA
    for _ in 0..5 {
        tracker.record("request", 250.0, true);
    }

    let compliance = tracker.get_compliance();
    assert!(compliance.compliance_percent > 0.0);
    assert!(compliance.current_p95_ms > 0.0);
}

#[test]
fn test_alert_engine_integration() {
    let engine = AlertEngine::new();

    // Add alert rules
    let rule1 = AlertRule {
        name: "high_latency".to_string(),
        metric: "latency_ms".to_string(),
        threshold: 100.0,
        severity: AlertSeverity::Error,
        comparison: AlertComparison::GreaterThan,
    };

    let rule2 = AlertRule {
        name: "low_throughput".to_string(),
        metric: "throughput".to_string(),
        threshold: 10.0,
        severity: AlertSeverity::Warning,
        comparison: AlertComparison::LessThan,
    };

    engine.add_rule(rule1);
    engine.add_rule(rule2);

    assert_eq!(engine.get_rules().len(), 2);

    // Test rule evaluation
    let alert = engine.check_rules("latency_ms", 150.0);
    assert!(alert.is_some());
    assert_eq!(alert.unwrap().severity, AlertSeverity::Error);

    let alert = engine.check_rules("throughput", 5.0);
    assert!(alert.is_some());
    assert_eq!(alert.unwrap().severity, AlertSeverity::Warning);

    // Test no alert
    let alert = engine.check_rules("latency_ms", 50.0);
    assert!(alert.is_none());
}

#[test]
fn test_dashboard_configuration() {
    let dashboard = DashboardConfig::default_ecosystem();
    assert_eq!(dashboard.name, "Bonsai Ecosystem Overview");
    assert!(!dashboard.panels.is_empty());

    // Test JSON export
    let json = dashboard.to_json();
    assert!(json.is_ok());

    // Test Grafana JSON export
    let grafana = dashboard.to_grafana_json();
    assert!(grafana.is_ok());
}

#[test]
fn test_system_specific_dashboard() {
    let dashboard = DashboardConfig::per_system("CI/CD");
    assert_eq!(dashboard.name, "CI/CD System Dashboard");
    assert!(!dashboard.panels.is_empty());
}

#[test]
fn test_performance_dashboard() {
    let dashboard = DashboardConfig::performance();
    assert_eq!(dashboard.name, "Performance Analysis");
    assert!(!dashboard.panels.is_empty());
}

#[tokio::test]
async fn test_observability_stack_integration() {
    let target = SLATarget {
        p95_latency_ms: 100.0,
        p99_latency_ms: 200.0,
        availability_percent: 99.95,
    };
    let stack = ObservabilityStack::new(target);

    // Initialize
    assert!(stack.initialize().await.is_ok());

    // Record operations
    stack.record_operation("api_request", 45.0, true);
    stack.record_operation("api_request", 52.0, true);
    stack.record_operation("api_request", 48.0, false);

    // Get SLA compliance
    let compliance = stack.get_sla_compliance();
    assert!(compliance.compliance_percent >= 0.0);

    // Export metrics
    let metrics = stack.export_prometheus().await;
    assert!(metrics.is_ok());
    let prometheus_text = metrics.unwrap();
    assert!(prometheus_text.contains("operations_total"));
}

#[test]
fn test_multi_operation_tracking() {
    let target = SLATarget {
        p95_latency_ms: 100.0,
        p99_latency_ms: 200.0,
        availability_percent: 99.95,
    };
    let tracker = SLATracker::new(target);
    let collector = MetricsCollector::new();

    // Track multiple operations
    for op_name in &["op1", "op2", "op3"] {
        for i in 0..50 {
            let latency = (i as f64) * 0.5 + 10.0;
            tracker.record(op_name, latency, i % 50 != 0);
            collector.record(op_name, latency, i % 50 != 0);
        }
    }

    // Verify per-operation compliance
    let op1_compliance = tracker.get_operation_compliance("op1");
    assert!(op1_compliance.compliance_percent >= 0.0);

    // Verify metrics per operation
    assert_eq!(
        collector.get_counter("operations_total{operation=\"op1\"}"),
        Some(50)
    );
}

#[test]
fn test_alert_severity_ordering() {
    use std::cmp::Ordering;
    assert_eq!(AlertSeverity::Info < AlertSeverity::Warning, true);
    assert_eq!(AlertSeverity::Warning < AlertSeverity::Error, true);
    assert_eq!(AlertSeverity::Error < AlertSeverity::Critical, true);
}

#[test]
fn test_alert_clearing() {
    let engine = AlertEngine::new();
    let rule = AlertRule {
        name: "test_rule".to_string(),
        metric: "test_metric".to_string(),
        threshold: 100.0,
        severity: AlertSeverity::Warning,
        comparison: AlertComparison::GreaterThan,
    };

    engine.add_rule(rule);

    // Fire alert
    engine.check_rules("test_metric", 150.0);
    assert_eq!(engine.get_active_alerts().len(), 1);

    // Clear resolved
    engine.clear_resolved("test_rule");
    assert_eq!(engine.get_active_alerts().len(), 0);
}

#[test]
fn test_gauge_operations() {
    let collector = MetricsCollector::new();

    collector.set_gauge("cpu_usage_percent", 45.5);
    assert_eq!(collector.get_gauge("cpu_usage_percent"), Some(45.5));

    collector.set_gauge("cpu_usage_percent", 62.3);
    assert_eq!(collector.get_gauge("cpu_usage_percent"), Some(62.3));
}

#[tokio::test]
async fn test_prometheus_export_format() {
    let collector = MetricsCollector::new();
    collector.record("api_call", 45.0, true);
    collector.record("api_call", 52.0, false);
    collector.set_gauge("system_health", 95.0);

    let prometheus = collector.export_prometheus().await;
    assert!(prometheus.is_ok());

    let text = prometheus.unwrap();
    assert!(text.contains("# HELP"));
    assert!(text.contains("# TYPE"));
    assert!(text.contains("operations_total"));
    assert!(text.contains("errors_total"));
}

#[test]
fn test_json_export() {
    let collector = MetricsCollector::new();
    collector.record("test_op", 25.0, true);

    let json = collector.export_json();
    assert!(json.is_ok());
    let json_str = json.unwrap();
    assert!(json_str.contains("counters"));
    assert!(json_str.contains("gauges"));
}
