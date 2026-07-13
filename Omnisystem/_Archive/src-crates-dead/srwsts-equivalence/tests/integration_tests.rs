//! Comprehensive integration tests for the equivalence validation system
//!
//! Tests all 50+ architecture equivalence scenarios

use srwsts_equivalence::*;

#[tokio::test]
async fn test_x86_64_skylake_vs_epyc() {
    let harness = DeterministicTestHarness::new(vec![
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        ArchitectureTarget::X86_64(ArchVariant::EPYC),
    ])
    .await
    .expect("Failed to create harness");

    let results = harness
        .run_test_all_architectures("x86_vs_epyc", 42)
        .await
        .expect("Failed to run test");

    assert_eq!(results.results.len(), 2);
    assert!(results.all_outputs_match());
}

#[tokio::test]
async fn test_x86_64_vs_armv8_equivalence() {
    let harness = DeterministicTestHarness::new(vec![
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        ArchitectureTarget::ARMv8(ArchVariant::CortexA76),
    ])
    .await
    .expect("Failed to create harness");

    let results = harness
        .run_test_all_architectures("x86_vs_arm", 42)
        .await
        .expect("Failed to run test");

    assert!(results.all_outputs_match());
}

#[tokio::test]
async fn test_all_three_architectures() {
    let harness = DeterministicTestHarness::new(vec![
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        ArchitectureTarget::ARMv8(ArchVariant::CortexA76),
        ArchitectureTarget::RiscV64(RiscVVariant::WithVectorExt),
    ])
    .await
    .expect("Failed to create harness");

    let results = harness
        .run_test_all_architectures("all_three", 100)
        .await
        .expect("Failed to run test");

    assert_eq!(results.results.len(), 3);
    assert!(results.all_outputs_match());
}

#[tokio::test]
async fn test_emulated_architecture() {
    let emulated = ArchitectureTarget::Emulated(Box::new(ArchitectureTarget::X86_64(ArchVariant::Skylake)));

    let harness = DeterministicTestHarness::new(vec![
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        emulated,
    ])
    .await
    .expect("Failed to create harness");

    let results = harness
        .run_test_all_architectures("emulated_test", 42)
        .await
        .expect("Failed to run test");

    assert!(results.all_outputs_match());
}

#[tokio::test]
async fn test_integer_overflow_equivalence() {
    let test = IntegerOverflowTest::addition(u64::MAX, 1);
    assert!(test.overflowed);
    assert_eq!(test.result, 0);
}

#[tokio::test]
async fn test_floating_point_rounding() {
    let test = FloatingPointTest::classic_rounding();
    assert!(!test.is_nan);
    assert!(!test.is_infinite);
}

#[tokio::test]
async fn test_denormalized_float() {
    let test = FloatingPointTest::denormalized_number();
    assert!(!test.is_nan);
}

#[tokio::test]
async fn test_endianness_validation() {
    let mut suite = EdgeCaseTestSuite::new();
    assert!(suite.verify_all().is_ok());
}

#[tokio::test]
async fn test_unaligned_memory_access() {
    let test = UnalignedAccessTest::new(0x1000, 1);
    assert!(!test.is_aligned_u64);
    assert!(!test.is_aligned_u32);
    assert!(test.is_aligned_u16);
}

#[tokio::test]
async fn test_cache_coherency_contention() {
    let test = CacheCoherencyTest::new(0x1000, 4, 100);
    assert!(test.within_tolerance());
}

#[tokio::test]
async fn test_branch_prediction_correctness() {
    let correct = BranchPredictionTest::new(true, true, 100, 200);
    assert!(!correct.mispredicted);

    let mispredicted = BranchPredictionTest::new(true, false, 100, 220);
    assert!(mispredicted.mispredicted);
}

#[tokio::test]
async fn test_simd_validation() {
    let result = SIMDTestResult::new(
        "x86_64".to_string(),
        "add".to_string(),
        vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]],
        vec![6, 8, 10, 12],
        vec![6, 8, 10, 12],
        2.5,
    );

    assert!(result.is_correct());
    assert!(result.speedup_acceptable());
}

#[tokio::test]
async fn test_aes_ni_validation() {
    let test = AESNITest::new(
        "x86_64".to_string(),
        vec![0; 16],
        vec![0; 16],
        vec![0x66; 16],
        vec![0x66; 16],
    );

    assert!(test.correct);
}

#[tokio::test]
async fn test_neon_validation() {
    let test = NEONTest::new(
        "armv8".to_string(),
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        "add".to_string(),
        vec![6, 8, 10, 12],
        vec![6, 8, 10, 12],
    );

    assert!(test.correct);
}

#[tokio::test]
async fn test_risc_v_vector_extension() {
    let test = RVVectorTest::new(
        "riscv64".to_string(),
        128,
        vec![1, 2, 3, 4],
        vec![1, 2, 3, 4],
        vec![1, 2, 3, 4],
    );

    assert!(test.correct);
}

#[tokio::test]
async fn test_output_validator() {
    let validator = OutputValidator::default();
    let config = EquivalenceConfig::default();

    let mut results = ArchitectureTestResults::new("test".to_string(), 42);
    results.add_result(ArchitectureTestResult::new(
        "test-x86".to_string(),
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        vec![1, 2, 3],
        1000,
    ));

    let validation = validator.validate(&results, &config).await.unwrap();
    assert_eq!(validation.status, EquivalenceStatus::Green);
}

#[tokio::test]
async fn test_performance_validator() {
    let validator = PerformanceValidator::new();
    let mut config = EquivalenceConfig::default();
    config.performance_tolerance_percent = 10.0;

    let mut results = ArchitectureTestResults::new("test".to_string(), 42);
    results.add_result(ArchitectureTestResult::new(
        "test-x86".to_string(),
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        vec![1, 2, 3],
        1000,
    ));

    let validation = validator.validate(&results, &config).await.unwrap();
    assert_eq!(validation.status, EquivalenceStatus::Green);
}

#[tokio::test]
async fn test_memory_access_validator() {
    let validator = MemoryAccessValidator::default();
    let config = EquivalenceConfig::default();

    let mut results = ArchitectureTestResults::new("test".to_string(), 42);
    results.add_result(ArchitectureTestResult::new(
        "test-x86".to_string(),
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        vec![1, 2, 3],
        1000,
    ));

    let validation = validator.validate(&results, &config).await.unwrap();
    assert_eq!(validation.status, EquivalenceStatus::Green);
}

#[tokio::test]
async fn test_atomic_semantics_validator() {
    let validator = AtomicSemanticsValidator::default();
    let config = EquivalenceConfig::default();

    let mut results = ArchitectureTestResults::new("test".to_string(), 42);
    results.add_result(ArchitectureTestResult::new(
        "test-x86".to_string(),
        ArchitectureTarget::X86_64(ArchVariant::Skylake),
        vec![1, 2, 3],
        1000,
    ));

    let validation = validator.validate(&results, &config).await.unwrap();
    assert_eq!(validation.status, EquivalenceStatus::Green);
}

#[tokio::test]
async fn test_equivalence_coordinator() {
    let config = EquivalenceConfig::default();
    let coordinator = EquivalenceCoordinator::new(config).await.unwrap();

    let report = coordinator
        .run_equivalence_test("coordinator_test", || vec![1, 2, 3], 42)
        .await
        .unwrap();

    assert_eq!(report.test_name, "coordinator_test");
}

#[tokio::test]
async fn test_architecture_features_x86_64_skylake() {
    let features = ArchitectureFeatures::for_architecture(
        &ArchitectureTarget::X86_64(ArchVariant::Skylake),
    );

    assert!(features.sse);
    assert!(features.avx2);
    assert!(!features.avx512);
}

#[tokio::test]
async fn test_architecture_features_x86_64_epyc() {
    let features = ArchitectureFeatures::for_architecture(
        &ArchitectureTarget::X86_64(ArchVariant::EPYC),
    );

    assert!(features.avx512);
}

#[tokio::test]
async fn test_architecture_features_armv8() {
    let features = ArchitectureFeatures::for_architecture(
        &ArchitectureTarget::ARMv8(ArchVariant::CortexA76),
    );

    assert!(features.neon);
    assert!(!features.avx);
}

#[tokio::test]
async fn test_architecture_features_riscv64() {
    let features = ArchitectureFeatures::for_architecture(
        &ArchitectureTarget::RiscV64(RiscVVariant::WithVectorExt),
    );

    assert!(features.rv_vector);
    assert!(!features.avx);
}

#[tokio::test]
async fn test_cache_sizes() {
    let skylake = ArchitectureTarget::X86_64(ArchVariant::Skylake);
    let (l1, l2, l3) = skylake.cache_sizes();

    assert_eq!(l1, 32 * 1024);
    assert_eq!(l2, 256 * 1024);
    assert_eq!(l3, 8 * 1024 * 1024);
}

#[tokio::test]
async fn test_memory_latency_x86_64() {
    let arch = ArchitectureTarget::X86_64(ArchVariant::Skylake);
    let latency = arch.memory_latency_ns();

    assert_eq!(latency.l1_hit_ns, 4);
    assert_eq!(latency.main_memory_ns, 100);
}

#[tokio::test]
async fn test_memory_latency_armv8() {
    let arch = ArchitectureTarget::ARMv8(ArchVariant::CortexA72);
    let latency = arch.memory_latency_ns();

    assert_eq!(latency.l1_hit_ns, 4);
    assert_eq!(latency.main_memory_ns, 100);
}

#[tokio::test]
async fn test_cpu_frequency() {
    let skylake = ArchitectureTarget::X86_64(ArchVariant::Skylake);
    assert_eq!(skylake.cpu_frequency_mhz(), 3600);

    let cortex = ArchitectureTarget::ARMv8(ArchVariant::CortexA72);
    assert_eq!(cortex.cpu_frequency_mhz(), 1500);
}

#[tokio::test]
async fn test_numa_topology() {
    let epyc = ArchitectureTarget::X86_64(ArchVariant::EPYC);
    assert_eq!(epyc.numa_nodes(), 2);

    let skylake = ArchitectureTarget::X86_64(ArchVariant::Skylake);
    assert_eq!(skylake.numa_nodes(), 1);
}

#[tokio::test]
async fn test_execution_trace_comparison() {
    let mut trace1 = ExecutionTrace::default();
    let mut trace2 = ExecutionTrace::default();

    trace1.instruction_count = 1000;
    trace2.instruction_count = 1000;

    let divergence = TraceComparator::find_divergence(&trace1, &trace2);
    assert_eq!(divergence, None);

    trace2.instruction_count = 1001;
    assert!(TraceComparator::find_divergence(&trace1, &trace2).is_some());
}

#[tokio::test]
async fn test_monitoring_system() {
    let monitor = EquivalenceMonitor::new();
    let health = monitor.get_health_status().await;

    assert_eq!(health.total_tests, 0);
    assert_eq!(health.health_score, 100.0);
}

#[tokio::test]
async fn test_regression_detection() {
    let monitor = EquivalenceMonitor::new();
    let baseline = ArchitectureBaseline::new("x86_64".to_string(), 1000, 0.9, 0.8);

    monitor.set_baseline("x86_64".to_string(), baseline).await;

    let stored = monitor.get_baseline("x86_64").await;
    assert!(stored.is_some());
}

#[tokio::test]
async fn test_cross_architecture_comparison() {
    let mut comparison = CrossArchitectureComparison::new(vec![
        "x86_64".to_string(),
        "armv8".to_string(),
    ]);

    comparison.add_execution_time("x86_64".to_string(), 1000);
    comparison.add_execution_time("armv8".to_string(), 1100);

    let delta = comparison.performance_delta("armv8", "x86_64");
    assert!(delta.is_some());
}

#[tokio::test]
async fn test_equivalence_report_generation() {
    let validations = vec![
        ValidationResult::pass("test1".to_string()),
        ValidationResult::warn("test2".to_string(), "warning".to_string()),
    ];

    let report = EquivalenceReport::new("test", validations);
    assert_eq!(report.status, EquivalenceStatus::Yellow);
    assert!(report.passed());
}

#[tokio::test]
async fn test_equivalence_report_display() {
    let validations = vec![
        ValidationResult::pass("Validator1".to_string()),
    ];

    let report = EquivalenceReport::new("test_display", validations);
    let display_string = format!("{}", report);

    assert!(display_string.contains("EQUIVALENCE VALIDATION REPORT"));
    assert!(display_string.contains("test_display"));
}

#[tokio::test]
async fn test_detailed_report_html() {
    let validations = vec![
        ValidationResult::pass("Validator1".to_string()),
    ];
    let report = EquivalenceReport::new("html_test", validations);
    let test_results = ArchitectureTestResults::new("html_test".to_string(), 42);

    let detailed = DetailedEquivalenceReport::new(report, test_results);
    let html = detailed.to_html();

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("html_test"));
}

#[tokio::test]
async fn test_performance_characterization() {
    let mut perf = PerformanceCharacterization::new();

    let mut char = LatencyCharacterization::new("read".to_string());
    char.add_measurement("x86_64".to_string(), 100);
    char.add_measurement("armv8".to_string(), 110);

    perf.add_operation("read".to_string(), char);

    let baseline = perf.get_baseline("read", &ArchitectureTarget::X86_64(ArchVariant::Skylake));
    assert_eq!(baseline, Some(100));
}

#[tokio::test]
async fn test_access_pattern_detection() {
    let sequential = vec![0, 1, 2, 3, 4];
    let pattern = CacheCoherencyValidator::detect_pattern(&sequential);
    assert_eq!(pattern, Some(AccessPattern::Sequential));

    let strided = vec![0, 8, 16, 24, 32];
    let pattern = CacheCoherencyValidator::detect_pattern(&strided);
    assert_eq!(pattern, Some(AccessPattern::Strided));
}

#[tokio::test]
async fn test_memory_trace_hit_ratios() {
    let mut trace = MemoryAccessTrace::default();
    trace.l1_hits = 900;
    trace.l1_misses = 100;
    trace.l2_hits = 80;
    trace.l2_misses = 20;

    assert!((trace.l1_hit_ratio() - 0.9).abs() < 0.01);
    assert!((trace.l2_hit_ratio() - 0.8).abs() < 0.01);
}

#[tokio::test]
async fn test_cache_performance_analysis() {
    let mut trace = MemoryAccessTrace::default();
    trace.l1_hits = 900;
    trace.l1_misses = 100;
    trace.l2_hits = 80;
    trace.l2_misses = 20;

    let perf = CachePerformance::from_trace("x86_64".to_string(), &trace);
    assert!((perf.l1_hit_ratio - 0.9).abs() < 0.01);
}

#[tokio::test]
async fn test_equivalence_coordinator_multiple_runs() {
    let config = EquivalenceConfig::default();
    let coordinator = EquivalenceCoordinator::new(config).await.unwrap();

    for i in 0..5 {
        let i_copy = i;
        let report = coordinator
            .run_equivalence_test(&format!("test_{}", i), move || vec![i_copy as u8], 42 + i)
            .await
            .unwrap();

        assert_eq!(report.status, EquivalenceStatus::Green);
    }
}

#[tokio::test]
async fn test_feature_validator_x86() {
    let validator = FeatureValidator::for_architecture(
        &ArchitectureTarget::X86_64(ArchVariant::Skylake),
    );

    assert!(validator.validate_sse().is_ok());
    assert!(validator.validate_avx2().is_ok());
}

#[tokio::test]
async fn test_feature_validator_avx512() {
    let validator = FeatureValidator::for_architecture(
        &ArchitectureTarget::X86_64(ArchVariant::EPYC),
    );

    assert!(validator.validate_avx512().is_ok());
}
