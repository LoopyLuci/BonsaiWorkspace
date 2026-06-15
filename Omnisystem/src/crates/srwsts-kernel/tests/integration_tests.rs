//! Integration Tests for SRWSTS Kernel
//!
//! Comprehensive test suite validating all kernel stress testing components
//! working together in realistic scenarios.

use srwsts_kernel::{
    bootstrap::{BootstrapConfig, KernelBootstrap},
    scheduler::{SchedulerConfig, SchedulerTest},
    memory::{MemoryConfig, MemoryTest},
    ipc::{IPCConfig, IPCTest},
    drivers::{DriverConfig, DriverTest},
    invariants::{InvariantConfig, InvariantTest},
    snapshots::{SnapshotConfig, SnapshotTest},
    faults::{FaultConfig, FaultScenario},
    reporting::{ResultReport, ReportGenerator, TestSuiteResult, TestCaseResult, TestStatus},
    KernelTestContext,
};
use srwsts_core::SrwstsConfig;

#[tokio::test]
async fn test_kernel_bootstrap() {
    let config = BootstrapConfig::default();
    let bootstrap = KernelBootstrap::new(config);

    let result = bootstrap.boot().await;
    assert!(result.is_ok());

    let state = bootstrap.get_state().await;
    assert!(state.is_successful());
    assert!(!state.subsystems_ready.is_empty());
    assert!(state.subsystems_ready.contains(&"scheduler".to_string()));
    assert!(state.subsystems_ready.contains(&"memory-management".to_string()));
}

#[tokio::test]
async fn test_scheduler_stress() {
    let config = SchedulerConfig {
        num_tasks: 500,
        task_duration_ms: 50,
        priority_levels: 8,
        enable_preemption: true,
        ..Default::default()
    };

    let test = SchedulerTest::new(config);
    let result = test.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_memory_stress() {
    let config = MemoryConfig {
        total_allocation_bytes: 500 * 1024 * 1024, // 500 MB
        concurrent_allocations: 20,
        enable_numa: true,
        enable_huge_pages: true,
        enable_swap: true,
        ..Default::default()
    };

    let test = MemoryTest::new(config);
    let result = test.run_all().await;
    assert!(result.is_ok());

    let stats = test.get_stats().await;
    assert!(stats.total_allocations > 0);
}

#[tokio::test]
async fn test_ipc_stress() {
    let config = IPCConfig {
        num_senders: 20,
        num_receivers: 20,
        messages_per_sender: 500,
        message_size_bytes: 256,
        channel_buffer_size: 500,
        ..Default::default()
    };

    let test = IPCTest::new(config);
    let result = test.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_driver_stress() {
    let config = DriverConfig {
        storage_iops_target: 50_000,
        parallel_io_ops: 50,
        network_packet_size: 1500,
        ..Default::default()
    };

    let test = DriverTest::new(config);
    let result = test.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_invariants() {
    let config = InvariantConfig {
        num_tasks: 50,
        test_duration_secs: 5,
        detect_deadlocks: true,
        detect_corruption: true,
        check_logical_invariants: true,
    };

    let test = InvariantTest::new(config);
    let result = test.run_all().await;
    assert!(result.is_ok());

    let results = result.unwrap();
    assert!(results.passed_checks > 0);
}

#[tokio::test]
async fn test_snapshots() {
    let config = SnapshotConfig {
        snapshot_size_bytes: 50 * 1024 * 1024, // 50 MB
        num_snapshots: 5,
        enable_integrity_checks: true,
        restore_under_load: true,
        concurrent_restores: 2,
    };

    let test = SnapshotTest::new(config);
    let result = test.test_snapshot_lifecycle().await;
    assert!(result.is_ok());

    let stats = test.get_stats().await;
    assert_eq!(stats.successful_snapshots, 5);
}

#[tokio::test]
async fn test_fault_scenarios() {
    let config = FaultConfig {
        memory_pressure_percent: 25,
        enable_clock_skew: true,
        enable_hw_failures: true,
        failure_rate_percent: 2,
        enable_throttling: true,
        throttle_level_percent: 20,
    };

    let scenario = FaultScenario::new(config);
    let result = scenario.run_all().await;
    assert!(result.is_ok());

    let results = result.unwrap();
    assert_eq!(results.total_faults, results.faults_injected);
}

#[tokio::test]
async fn test_full_kernel_test_context() {
    let config = SrwstsConfig {
        max_concurrent: 100,
        test_timeout: std::time::Duration::from_secs(60),
        enable_faults: true,
        enable_collection: true,
        deterministic: false,
    };

    let context = KernelTestContext::new(config);
    assert_eq!(context.config.max_concurrent, 100);
}

#[tokio::test]
async fn test_reporting() {
    let mut report = ResultReport::new("test-run-001", "UOSC-0.1.0");

    let mut suite = TestSuiteResult::new("SchedulerTests");
    suite.add_test(TestCaseResult::new("test_priority", TestStatus::Pass).with_duration(50.5));
    suite.add_test(TestCaseResult::new("test_context_switch", TestStatus::Pass).with_duration(45.2));

    let mut suite2 = TestSuiteResult::new("MemoryTests");
    suite2.add_test(TestCaseResult::new("test_allocation", TestStatus::Pass).with_duration(120.0));

    report.add_suite(suite);
    report.add_suite(suite2);
    report.calculate_pass_rate();

    assert_eq!(report.total_tests, 3);
    assert_eq!(report.total_passed, 3);
    assert_eq!(report.overall_pass_rate, 100.0);
    assert!(report.is_passed());

    // Verify text output
    let text = report.to_text();
    assert!(text.contains("UOSC Kernel Stress Test Report"));
    assert!(text.contains("test-run-001"));

    // Verify JSON output
    let json_result = ReportGenerator::to_json(&report);
    assert!(json_result.is_ok());
}

#[tokio::test]
async fn test_end_to_end_stress_suite() {
    // Bootstrap kernel
    let bootstrap_config = BootstrapConfig::default();
    let bootstrap = KernelBootstrap::new(bootstrap_config);
    let boot_result = bootstrap.boot().await;
    assert!(boot_result.is_ok());

    let boot_state = bootstrap.get_state().await;
    assert!(boot_state.is_successful());

    // Create report
    let mut report = ResultReport::new("full-suite-001", "UOSC-0.1.0");

    // Scheduler test
    let sched_config = SchedulerConfig {
        num_tasks: 100,
        task_duration_ms: 20,
        ..Default::default()
    };
    let sched_test = SchedulerTest::new(sched_config);
    let sched_result = sched_test.run().await;

    let mut sched_suite = TestSuiteResult::new("SchedulerTests");
    sched_suite.add_test(
        TestCaseResult::new(
            "scheduler_stress",
            if sched_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(100.0),
    );
    report.add_suite(sched_suite);

    // Memory test
    let mem_config = MemoryConfig {
        total_allocation_bytes: 200 * 1024 * 1024,
        concurrent_allocations: 10,
        ..Default::default()
    };
    let mem_test = MemoryTest::new(mem_config);
    let mem_result = mem_test.test_allocation_stress().await;

    let mut mem_suite = TestSuiteResult::new("MemoryTests");
    mem_suite.add_test(
        TestCaseResult::new(
            "memory_stress",
            if mem_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(200.0),
    );
    report.add_suite(mem_suite);

    // IPC test
    let ipc_config = IPCConfig {
        num_senders: 10,
        num_receivers: 10,
        messages_per_sender: 100,
        ..Default::default()
    };
    let ipc_test = IPCTest::new(ipc_config);
    let ipc_result = ipc_test.run().await;

    let mut ipc_suite = TestSuiteResult::new("IPCTests");
    ipc_suite.add_test(
        TestCaseResult::new(
            "ipc_stress",
            if ipc_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(150.0),
    );
    report.add_suite(ipc_suite);

    // Driver test
    let driver_config = DriverConfig {
        storage_iops_target: 10_000,
        parallel_io_ops: 10,
        ..Default::default()
    };
    let driver_test = DriverTest::new(driver_config);
    let driver_result = driver_test.run().await;

    let mut driver_suite = TestSuiteResult::new("DriverTests");
    driver_suite.add_test(
        TestCaseResult::new(
            "driver_stress",
            if driver_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(180.0),
    );
    report.add_suite(driver_suite);

    // Invariant test
    let inv_config = InvariantConfig {
        num_tasks: 20,
        ..Default::default()
    };
    let inv_test = InvariantTest::new(inv_config);
    let inv_result = inv_test.run_all().await;

    let mut inv_suite = TestSuiteResult::new("InvariantTests");
    inv_suite.add_test(
        TestCaseResult::new(
            "invariants_stress",
            if inv_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(120.0),
    );
    report.add_suite(inv_suite);

    // Snapshot test
    let snap_config = SnapshotConfig {
        snapshot_size_bytes: 20 * 1024 * 1024,
        num_snapshots: 3,
        ..Default::default()
    };
    let snap_test = SnapshotTest::new(snap_config);
    let snap_result = snap_test.test_snapshot_lifecycle().await;

    let mut snap_suite = TestSuiteResult::new("SnapshotTests");
    snap_suite.add_test(
        TestCaseResult::new(
            "snapshot_lifecycle",
            if snap_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(200.0),
    );
    report.add_suite(snap_suite);

    // Fault injection test
    let fault_config = FaultConfig {
        memory_pressure_percent: 10,
        failure_rate_percent: 1,
        ..Default::default()
    };
    let fault_scenario = FaultScenario::new(fault_config);
    let fault_result = fault_scenario.run_all().await;

    let mut fault_suite = TestSuiteResult::new("FaultScenarios");
    fault_suite.add_test(
        TestCaseResult::new(
            "fault_injection",
            if fault_result.is_ok() {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
        )
        .with_duration(300.0),
    );
    report.add_suite(fault_suite);

    // Finalize report
    report.duration_ms = 1250.0;
    report.calculate_pass_rate();

    // Verify report
    assert_eq!(report.total_tests, 8);
    assert!(report.overall_pass_rate >= 50.0);

    // Verify text output
    let text = report.to_text();
    assert!(text.contains("UOSC Kernel Stress Test Report"));
    assert!(text.contains("SchedulerTests"));
    assert!(text.contains("MemoryTests"));

    // Verify HTML output
    let html = report.to_html();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("UOSC Kernel Stress Test Report"));
}

#[test]
fn test_concurrent_bootstrap_scenarios() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let mut handles = vec![];

        for i in 0..5 {
            let handle = tokio::spawn(async move {
                let config = BootstrapConfig {
                    kernel_version: format!("UOSC-0.1.0-{}", i),
                    num_cpus: 8 + i,
                    ..Default::default()
                };

                let bootstrap = KernelBootstrap::new(config);
                let result = bootstrap.boot().await;
                assert!(result.is_ok());

                let state = bootstrap.get_state().await;
                assert!(state.is_successful());

                i
            });

            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok());
        }
    });
}
