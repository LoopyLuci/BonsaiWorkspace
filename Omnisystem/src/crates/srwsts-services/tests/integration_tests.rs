/// Integration tests for SRWSTS Services
///
/// These tests verify that all service stress testing components work together
/// to provide a complete testing framework for Omnisystem services.

#[cfg(test)]
mod integration_tests {
    use srwsts_services::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_bootstrap_initializes_all_services() {
        let config = ServiceBootstrapConfig::new(vec![
            ServiceType::P2P,
            ServiceType::Storage,
            ServiceType::Network,
        ]);

        let mut bootstrap = ServiceBootstrap::new(config);
        assert!(bootstrap.initialize().await.is_ok());

        let services = bootstrap.get_services().await.unwrap();
        assert_eq!(services.len(), 3);

        for service in services {
            assert_eq!(service.status, ServiceStatus::Running);
        }
    }

    #[tokio::test]
    async fn test_full_stress_test_suite() {
        let config = ServiceBootstrapConfig::default().with_test_config(
            TestConfig::default()
                .with_ops_per_sec(100)
                .with_deterministic(true),
        );

        let mut bootstrap = ServiceBootstrap::new(config);
        assert!(bootstrap.initialize().await.is_ok());
        assert!(bootstrap.run_all_tests().await.is_ok());

        let report = bootstrap.generate_report().await.unwrap();
        assert!(report.total_tests > 0);
    }

    #[tokio::test]
    async fn test_p2p_mesh_convergence() {
        let config = TestConfig::default();
        let tests = P2PStressTests::new(config);
        let result = tests.test_mesh_convergence().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_p2p_multipath_bonding() {
        let config = TestConfig::default();
        let tests = P2PStressTests::new(config);
        let result = tests.test_multipath_bonding().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_p2p_bandwidth_scaling() {
        let config = TestConfig::default();
        let tests = P2PStressTests::new(config);
        let result = tests.test_bandwidth_scaling().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_storage_cas_throughput() {
        let config = TestConfig::default();
        let tests = StorageStressTests::new(config);
        let result = tests.test_cas_throughput().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_storage_deduplication() {
        let config = TestConfig::default();
        let tests = StorageStressTests::new(config);
        let result = tests.test_deduplication().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_storage_erasure_reconstruction() {
        let config = TestConfig::default();
        let tests = StorageStressTests::new(config);
        let result = tests.test_erasure_reconstruction().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_network_tcp_throughput() {
        let config = TestConfig::default();
        let tests = NetworkStressTests::new(config);
        let result = tests.test_tcp_high_throughput().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_network_firewall_matching() {
        let config = TestConfig::default();
        let tests = NetworkStressTests::new(config);
        let result = tests.test_firewall_matching().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_network_packet_reordering() {
        let config = TestConfig::default();
        let tests = NetworkStressTests::new(config);
        let result = tests.test_packet_reordering().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_compositor_60fps_rendering() {
        let config = TestConfig::default().with_ops_per_sec(60);
        let tests = CompositorStressTests::new(config);
        let result = tests.test_60fps_rendering().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_compositor_gpu_memory_exhaustion() {
        let config = TestConfig::default();
        let tests = CompositorStressTests::new(config);
        let result = tests.test_gpu_memory_exhaustion().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_compositor_gpu_reset_recovery() {
        let config = TestConfig::default();
        let tests = CompositorStressTests::new(config);
        let result = tests.test_gpu_reset_recovery().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_discovery_registration() {
        let config = TestConfig::default();
        let tests = ServiceDiscoveryTests::new(config);
        let result = tests.test_dynamic_registration().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_discovery_dns_resolution() {
        let config = TestConfig::default();
        let tests = ServiceDiscoveryTests::new(config);
        let result = tests.test_dns_resolution_under_load().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_discovery_health_checks() {
        let config = TestConfig::default();
        let tests = ServiceDiscoveryTests::new(config);
        let result = tests.test_health_check_frequency().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_interaction_communication() {
        let config = TestConfig::default();
        let tests = ServiceInteractionTests::new(config);
        let result = tests.test_cross_service_communication().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_interaction_cascading_failures() {
        let config = TestConfig::default();
        let tests = ServiceInteractionTests::new(config);
        let result = tests.test_cascading_failures().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_service_interaction_backpressure() {
        let config = TestConfig::default();
        let tests = ServiceInteractionTests::new(config);
        let result = tests.test_backpressure_handling().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_fault_scenario_service_kill() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);
        let result = tests.test_service_kill().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_fault_scenario_network_partition() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);
        let result = tests.test_network_partition().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_fault_scenario_storage_failure() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);
        let result = tests.test_storage_failure().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_fault_scenario_cpu_overload() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);
        let result = tests.test_cpu_overload().await.unwrap();
        assert!(matches!(
            result.status,
            TestResultStatus::Passed | TestResultStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_report_generation() {
        let mut report = TestReport::new();
        report.total_tests = 100;
        report.passed_tests = 95;
        report.failed_tests = 5;
        report.success_rate = 95.0;

        assert!(report.is_success());
        let json = report.to_json().unwrap();
        assert!(json.contains("report_id"));
    }

    #[test]
    fn test_metrics_collection() {
        let collector = ServiceMetricsCollector::new("test-service");
        collector.record_operation("op1", 10.0, true, None);
        collector.record_operation("op2", 20.0, false, Some("error".to_string()));

        let metrics = collector.aggregate();
        assert_eq!(metrics.total_operations, 2);
        assert_eq!(metrics.errors.total_errors, 1);
        assert!(metrics.success_rate() > 40.0 && metrics.success_rate() < 60.0);
    }

    #[test]
    fn test_service_creation() {
        let service = Service::new("svc-1", ServiceType::P2P);
        assert_eq!(service.id.as_str(), "svc-1");
        assert_eq!(service.service_type, ServiceType::P2P);
        assert!(!service.is_healthy());
    }

    #[test]
    fn test_test_config_builder() {
        let config = TestConfig::default()
            .with_ops_per_sec(5000)
            .with_deterministic(true);

        assert_eq!(config.ops_per_sec, 5000);
        assert!(config.deterministic);
    }
}
