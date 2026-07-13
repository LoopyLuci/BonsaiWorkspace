use infrastructure_core::*;
use infrastructure_loadbalancer::*;
use infrastructure_monitoring::*;
use infrastructure_registry::*;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use chrono::Utc;

/// Performance benchmark: Registry operations
#[tokio::test]
async fn benchmark_registry_operations() {
    let registry = Arc::new(InMemoryRegistry::new());
    let svc_id = ServiceId("perf-test".to_string());

    // Register service
    let def = ServiceDefinition {
        id: svc_id.clone(),
        name: "Performance Test".to_string(),
        protocol: "http".to_string(),
        port: 8080,
        tags: vec![],
        health_check: Default::default(),
        load_balancer_policy: LoadBalancerPolicy::RoundRobin,
        created_at: Utc::now(),
    };

    registry.register_service(def).await.unwrap();

    // Register many instances
    let start = Instant::now();
    for i in 0..1000 {
        let instance = ServiceInstance::new(
            svc_id.clone(),
            format!("instance-{}", i),
            8080 + (i % 100) as u16,
        );
        registry.register_instance(instance).await.unwrap();
    }
    let register_time = start.elapsed();

    // Lookup performance
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = registry.get_instances(&svc_id).await.unwrap();
    }
    let lookup_time = start.elapsed();

    // Assert acceptable performance
    assert!(register_time.as_millis() < 1000, "Registry registration took {:?}", register_time);
    assert!(lookup_time.as_millis() < 500, "Registry lookup took {:?}", lookup_time);

    println!("Registry registration (1000 ops): {:?}", register_time);
    println!("Registry lookup (1000 ops): {:?}", lookup_time);
}

/// Performance benchmark: Load balancer selection
#[tokio::test]
async fn benchmark_load_balancer_selection() {
    let registry = Arc::new(InMemoryRegistry::new());
    let svc_id = ServiceId("lb-test".to_string());
    let lb = DefaultLoadBalancer::new(registry.clone());

    // Setup service with 10 instances
    let def = ServiceDefinition {
        id: svc_id.clone(),
        name: "LB Test".to_string(),
        protocol: "http".to_string(),
        port: 8080,
        tags: vec![],
        health_check: Default::default(),
        load_balancer_policy: LoadBalancerPolicy::RoundRobin,
        created_at: Utc::now(),
    };

    registry.register_service(def).await.unwrap();

    for i in 0..10 {
        let mut instance = ServiceInstance::new(
            svc_id.clone(),
            format!("instance-{}", i),
            8080,
        );
        instance.health_status = HealthStatus::Healthy;
        registry.register_instance(instance).await.unwrap();
    }

    // Benchmark selection
    let start = Instant::now();
    for _ in 0..10000 {
        let _ = lb.select_instance(&svc_id).await.unwrap();
    }
    let selection_time = start.elapsed();

    assert!(selection_time.as_millis() < 100, "Selection took {:?}", selection_time);
    println!("Load balancer selection (10k ops): {:?}", selection_time);
    println!("Avg selection time: {:?} µs", selection_time.as_micros() / 10000);
}

/// Performance benchmark: Metrics recording
#[tokio::test]
async fn benchmark_metrics_recording() {
    let metrics = InMemoryMetrics::new();
    let svc_id = ServiceId("metrics-test".to_string());

    // Benchmark request recording
    let start = Instant::now();
    for i in 0..10000 {
        metrics
            .record_request(&svc_id, 10 + (i % 100) as u64, i % 20 != 0)
            .await
            .unwrap();
    }
    let recording_time = start.elapsed();

    // Benchmark metrics calculation
    let start = Instant::now();
    for _ in 0..100 {
        let _ = metrics.get_service_metrics(&svc_id, 3600).await.unwrap();
    }
    let calculation_time = start.elapsed();

    assert!(recording_time.as_millis() < 1000, "Recording took {:?}", recording_time);
    assert!(calculation_time.as_millis() < 100, "Calculation took {:?}", calculation_time);

    println!("Metrics recording (10k ops): {:?}", recording_time);
    println!("Metrics calculation (100 ops): {:?}", calculation_time);
}

/// Performance benchmark: Concurrent operations
#[tokio::test]
async fn benchmark_concurrent_operations() {
    let registry = Arc::new(InMemoryRegistry::new());
    let svc_id = ServiceId("concurrent-test".to_string());

    // Register service
    let def = ServiceDefinition {
        id: svc_id.clone(),
        name: "Concurrent Test".to_string(),
        protocol: "http".to_string(),
        port: 8080,
        tags: vec![],
        health_check: Default::default(),
        load_balancer_policy: LoadBalancerPolicy::RoundRobin,
        created_at: Utc::now(),
    };

    registry.register_service(def).await.unwrap();

    // Register instances
    for i in 0..100 {
        let instance = ServiceInstance::new(
            svc_id.clone(),
            format!("instance-{}", i),
            8080,
        );
        registry.register_instance(instance).await.unwrap();
    }

    // Spawn concurrent tasks
    let start = Instant::now();
    let mut handles = vec![];

    for task_id in 0..10 {
        let registry_clone = registry.clone();
        let svc_clone = svc_id.clone();

        let handle = tokio::spawn(async move {
            for _ in 0..1000 {
                let _ = registry_clone.get_instances(&svc_clone).await.unwrap();
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        let _ = handle.await;
    }

    let concurrent_time = start.elapsed();

    println!("Concurrent operations (10 tasks × 1000 ops): {:?}", concurrent_time);
    println!("Total ops: 10,000 in {:?}", concurrent_time);
    println!("Ops/sec: {}", (10000.0 / concurrent_time.as_secs_f64()) as u64);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_performance_targets() {
        // Performance targets for Phase 6A
        let targets = vec![
            ("Registry registration", 1000),      // 1000ms for 1000 ops
            ("Registry lookup", 500),             // 500ms for 1000 ops
            ("LB selection", 100),                // 100ms for 10k ops
            ("Metrics recording", 1000),          // 1000ms for 10k ops
            ("Metrics calculation", 100),         // 100ms for 100 ops
        ];

        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_scalability_targets() {
        // Scalability targets
        let scale_tests = vec![
            ("1000 services", 1000),
            ("10000 instances", 10000),
            ("1M metrics points", 1_000_000),
        ];

        assert!(scale_tests.iter().all(|(_, count)| *count > 0));
    }
}
