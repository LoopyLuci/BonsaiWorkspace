use omnisystem_observability::*;

#[test]
fn test_metrics_collector() {
    let collector = MetricsCollector::new();
    collector.record("test".to_string(), 1.0);
    assert!(collector.get("test").is_some());
}

#[test]
fn test_trace_context() {
    let ctx = TraceContext::new("t1".to_string(), "s1".to_string());
    assert_eq!(ctx.trace_id, "t1");
}

#[test]
fn test_logger() {
    let logger = Logger::new();
    logger.log("msg1".to_string());
    assert_eq!(logger.get_logs().len(), 1);
}
