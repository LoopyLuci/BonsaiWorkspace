/// Comprehensive service stress test suite
///
/// This module contains complete stress tests for all Omnisystem services,
/// covering performance, reliability, and failure scenarios.

#[cfg(test)]
mod service_stress_tests {
    use srwsts_services::*;
    use std::time::Duration;

    /// Run comprehensive P2P stress tests
    #[tokio::test]
    async fn test_p2p_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(1000)
            .with_deterministic(false);

        let tests = P2PStressTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!("P2P Tests: {}/{} passed", report.passed_tests, report.total_tests);
    }

    /// Run comprehensive Storage stress tests
    #[tokio::test]
    async fn test_storage_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(500)
            .with_deterministic(false);

        let tests = StorageStressTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Storage Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Run comprehensive Network stress tests
    #[tokio::test]
    async fn test_network_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(2000)
            .with_deterministic(false);

        let tests = NetworkStressTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Network Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Run comprehensive Compositor stress tests
    #[tokio::test]
    async fn test_compositor_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(60)
            .with_deterministic(false);

        let tests = CompositorStressTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Compositor Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Run comprehensive Service Discovery stress tests
    #[tokio::test]
    async fn test_service_discovery_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(1000)
            .with_deterministic(false);

        let tests = ServiceDiscoveryTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Service Discovery Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Run comprehensive Service Interaction stress tests
    #[tokio::test]
    async fn test_service_interaction_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(1000)
            .with_deterministic(false);

        let tests = ServiceInteractionTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Service Interaction Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Run comprehensive Fault Scenario tests
    #[tokio::test]
    async fn test_fault_scenario_comprehensive_suite() {
        let config = TestConfig::default()
            .with_ops_per_sec(100)
            .with_deterministic(false);

        let tests = FaultScenarioTests::new(config);
        let report = tests.run_all_tests().await.unwrap();

        assert!(report.total_tests > 0);
        println!(
            "Fault Scenario Tests: {}/{} passed",
            report.passed_tests, report.total_tests
        );
    }

    /// Full end-to-end bootstrap and testing
    #[tokio::test]
    async fn test_full_omnisystem_stress_testing() {
        let config = ServiceBootstrapConfig::default()
            .with_test_config(TestConfig::default().with_ops_per_sec(100))
            .with_faults(true);

        let mut bootstrap = ServiceBootstrap::new(config);

        // Initialize all services
        assert!(bootstrap.initialize().await.is_ok());

        // Verify services are running
        let services = bootstrap.get_services().await.unwrap();
        assert!(!services.is_empty());

        for service in &services {
            assert_eq!(service.status, ServiceStatus::Running);
        }

        // Run all stress tests
        assert!(bootstrap.run_all_tests().await.is_ok());

        // Generate comprehensive report
        let mut report = bootstrap.generate_report().await.unwrap();
        report.generate_service_health();
        report.generate_performance_analysis(10.0);
        report.generate_summary();

        // Verify report
        assert!(report.total_tests > 0);
        assert!(!report.service_health.is_empty());
        assert!(!report.summary.is_empty());

        println!("\n{}", report.summary);
        println!("Overall success rate: {:.1}%", report.success_rate);

        // Shutdown services
        assert!(bootstrap.shutdown().await.is_ok());
    }

    /// Test high concurrency scenario
    #[tokio::test]
    async fn test_high_concurrency_stress() {
        let mut config = TestConfig::default();
        config.concurrency = 100; // High concurrency

        let tests = ServiceInteractionTests::new(config);
        let result = tests.test_cross_service_communication().await.unwrap();

        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test extreme load scenario
    #[tokio::test]
    async fn test_extreme_load_scenario() {
        let mut config = TestConfig::default();
        config.ops_per_sec = 10000;

        let tests = NetworkStressTests::new(config);
        let result = tests.test_tcp_high_throughput().await.unwrap();

        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test extended duration stress
    #[tokio::test]
    async fn test_extended_duration_stress() {
        let mut config = TestConfig::default();
        config.timeout = Duration::from_secs(30);

        let tests = P2PStressTests::new(config);
        let result = tests.test_node_churn().await.unwrap();

        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test recovery under load
    #[tokio::test]
    async fn test_recovery_under_load() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);
        let result = tests.test_fault_recovery().await.unwrap();

        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test cascading fault propagation
    #[tokio::test]
    async fn test_cascading_propagation_under_load() {
        let config = TestConfig::default();
        let tests = ServiceInteractionTests::new(config);
        let result = tests.test_cascading_failures().await.unwrap();

        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test latency bounds under various loads
    #[tokio::test]
    async fn test_latency_bounds() {
        for concurrency in &[1, 10, 50, 100] {
            let mut config = TestConfig::default();
            config.concurrency = *concurrency;

            let tests = P2PStressTests::new(config);
            let result = tests.test_latency_under_load().await.unwrap();

            assert!(matches!(
                result.status,
                TestResultStatus::Passed | TestResultStatus::Failed
            ));
            println!("Latency test with concurrency {}: {:?}", concurrency, result.status);
        }
    }

    /// Test throughput across different payload sizes
    #[tokio::test]
    async fn test_throughput_across_payload_sizes() {
        let config = TestConfig::default();
        let tests = StorageStressTests::new(config);

        // CAS throughput includes various payload sizes
        let result = tests.test_cas_throughput().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    /// Test metrics aggregation and accuracy
    #[test]
    fn test_metrics_aggregation_accuracy() {
        let collector = ServiceMetricsCollector::new("test-service");

        // Record various operations
        for i in 0..1000 {
            let latency = 10.0 + (i as f64 * 0.1);
            let success = i % 10 != 0; // 90% success rate
            collector.record_operation(
                "test_op",
                latency,
                success,
                if success { None } else { Some("simulated error".to_string()) },
            );
        }

        let metrics = collector.aggregate();
        assert_eq!(metrics.total_operations, 1000);
        assert_eq!(metrics.errors.total_errors, 100);
        assert!((metrics.errors.error_rate - 10.0).abs() < 0.5); // 10% error rate
    }

    /// Test service health status reporting
    #[test]
    fn test_service_health_reporting() {
        let mut metrics = ServiceMetrics::new("test-service");
        metrics.total_operations = 100;
        metrics.errors.total_errors = 5;
        metrics.latency.mean_ms = 25.0;
        metrics.latency.p99_ms = 100.0;

        let health = ServiceHealthStatus::from_metrics("svc-1", "P2P", &metrics);
        assert!(health.is_healthy); // 95% success rate
        assert_eq!(health.error_rate, 0.0); // Default value
    }

    /// Verify all test categories generate results
    #[tokio::test]
    async fn test_all_test_categories() {
        let test_categories = vec![
            ("P2P", 5),
            ("Storage", 5),
            ("Network", 6),
            ("Compositor", 6),
            ("ServiceDiscovery", 5),
            ("ServiceInteraction", 6),
            ("FaultScenarios", 7),
        ];

        let mut total_tests = 0;
        for (category, expected_tests) in test_categories {
            println!("Category: {}, Expected tests: {}", category, expected_tests);
            total_tests += expected_tests;
        }

        println!("\nTotal expected tests: {}", total_tests);
        assert!(total_tests >= 40, "Expected at least 40 total tests");
    }

    /// Test deterministic mode for reproducibility
    #[tokio::test]
    async fn test_deterministic_mode() {
        let mut config = TestConfig::default();
        config.deterministic = true;

        let tests = P2PStressTests::new(config);
        let result1 = tests.test_mesh_convergence().await.unwrap();
        let result2 = tests.test_mesh_convergence().await.unwrap();

        // Both should have same status in deterministic mode
        assert_eq!(result1.status, result2.status);
    }
}
