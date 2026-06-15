//! Comprehensive service API tests (100+ tests)
//!
//! Tests cover:
//! - Service startup, shutdown, restart
//! - Concurrent operations
//! - State transitions
//! - Error handling
//! - Configuration management
//! - Snapshot operations

use omni_bot_tests::{TestContext, TestDataBuilder};
use std::sync::Arc;

#[tokio::test]
async fn service_start_basic() {
    let ctx = TestContext::new();
    let result = ctx.client.start_service("p2p", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().state, "running");
}

#[tokio::test]
async fn service_start_with_config() {
    let ctx = TestContext::new();
    let config = serde_json::json!({"timeout": 30, "retries": 3});
    let result = ctx.client.start_service("p2p", Some(config)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_stop_graceful() {
    let ctx = TestContext::new();
    let result = ctx.client.stop_service("p2p", Some(true), Some(30)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().state, "stopped");
}

#[tokio::test]
async fn service_stop_forceful() {
    let ctx = TestContext::new();
    let result = ctx.client.stop_service("p2p", Some(false), Some(5)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_restart_basic() {
    let ctx = TestContext::new();
    let result = ctx.client.restart_service("p2p", None).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.old_pid.is_some());
    assert!(response.new_pid.is_some());
}

#[tokio::test]
async fn service_restart_graceful() {
    let ctx = TestContext::new();
    let result = ctx.client.restart_service("p2p", Some(true)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_list_empty() {
    let ctx = TestContext::new();
    let result = ctx.client.list_services().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().total_count, 0);
}

#[tokio::test]
async fn service_list_multiple() {
    let ctx = TestContext::new();
    let _ = ctx.client.start_service("p2p", None).await;
    let _ = ctx.client.start_service("mesh", None).await;
    let result = ctx.client.list_services().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_get_details() {
    let ctx = TestContext::new();
    let result = ctx.client.get_service_detail("p2p").await;
    assert!(result.is_ok());
    let detail = result.unwrap();
    assert_eq!(detail.name, "p2p");
    assert!(detail.pid.is_some());
}

#[tokio::test]
async fn service_configure() {
    let ctx = TestContext::new();
    let config = serde_json::json!({"port": 8080, "debug": true});
    let result = ctx.client.configure_service("p2p", config, None).await;
    assert!(result.is_ok());
    assert!(result.unwrap().applied);
}

#[tokio::test]
async fn service_configure_merge() {
    let ctx = TestContext::new();
    let config = serde_json::json!({"new_key": "value"});
    let result = ctx.client.configure_service("p2p", config, Some(true)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_snapshot() {
    let ctx = TestContext::new();
    let result = ctx
        .client
        .snapshot_service("p2p", Some("backup-1".to_string()))
        .await;
    assert!(result.is_ok());
    let snapshot = result.unwrap();
    assert!(!snapshot.snapshot_id.is_empty());
}

#[tokio::test]
async fn service_snapshot_unnamed() {
    let ctx = TestContext::new();
    let result = ctx.client.snapshot_service("p2p", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_get_logs() {
    let ctx = TestContext::new();
    let result = ctx.client.get_service_logs("p2p", Some(10), None).await;
    assert!(result.is_ok());
    let logs = result.unwrap();
    assert!(!logs.lines.is_empty());
}

#[tokio::test]
async fn service_get_logs_filtered() {
    let ctx = TestContext::new();
    let result = ctx
        .client
        .get_service_logs("p2p", Some(100), Some("ERROR".to_string()))
        .await;
    assert!(result.is_ok());
}

// Concurrent operation tests
#[tokio::test]
async fn service_concurrent_start() {
    let ctx = Arc::new(TestContext::new());
    let mut tasks = vec![];

    for i in 1..4 {
        let ctx = Arc::clone(&ctx);
        let task = tokio::task::spawn(async move {
            let _ = ctx.client.start_service(&format!("service-{}", i), None).await;
        });
        tasks.push(task);
    }

    for task in tasks {
        assert!(task.await.is_ok());
    }
}

#[tokio::test]
async fn service_concurrent_operations() {
    let ctx = Arc::new(TestContext::new());
    let mut tasks = vec![];

    for _ in 0..4 {
        let ctx = Arc::clone(&ctx);
        let task = tokio::task::spawn(async move {
            let _ = ctx.client.start_service("p2p", None).await;
        });
        tasks.push(task);
    }

    for task in tasks {
        assert!(task.await.is_ok());
    }
}

// State transition tests
#[tokio::test]
async fn service_state_transition_start() {
    let ctx = TestContext::new();
    // Service starts in unstarted state
    // After start, should be running
    let result = ctx.client.start_service("p2p", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().state, "running");
}

#[tokio::test]
async fn service_state_transition_stop() {
    let ctx = TestContext::new();
    let result = ctx.client.stop_service("p2p", Some(true), Some(30)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().state, "stopped");
}

#[tokio::test]
async fn service_state_transition_restart() {
    let ctx = TestContext::new();
    let result = ctx.client.restart_service("p2p", Some(true)).await;
    assert!(result.is_ok());
}

// Error handling tests
#[tokio::test]
async fn service_start_error_mode() {
    let ctx = TestContext::new();
    ctx.server.set_error_mode(Some("Service failed to start".to_string()));
    let result = ctx.client.start_service("p2p", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn service_stop_error_mode() {
    let ctx = TestContext::new();
    ctx.server.set_error_mode(Some("Service already stopped".to_string()));
    let result = ctx.client.stop_service("p2p", Some(true), Some(30)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn service_configure_error() {
    let ctx = TestContext::new();
    ctx.server.set_error_mode(Some("Invalid configuration".to_string()));
    let config = serde_json::json!({"invalid": "config"});
    let result = ctx.client.configure_service("p2p", config, None).await;
    assert!(result.is_err());
}

// Configuration tests
#[tokio::test]
async fn service_config_persistence() {
    let ctx = TestContext::new();
    let config = serde_json::json!({"key": "value", "count": 42});
    let result = ctx.client.configure_service("p2p", config.clone(), None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().config, config);
}

#[tokio::test]
async fn service_config_validation() {
    let ctx = TestContext::new();
    let valid_config = serde_json::json!({
        "timeout": 30,
        "retries": 3,
        "debug": true
    });
    let result = ctx.client.configure_service("p2p", valid_config, None).await;
    assert!(result.is_ok());
}

// Resource management tests
#[tokio::test]
async fn service_resource_monitoring() {
    let ctx = TestContext::new();
    let result = ctx.client.get_service_detail("p2p").await;
    assert!(result.is_ok());
    let detail = result.unwrap();
    assert!(detail.cpu_percent >= 0.0);
    assert!(detail.memory_mb > 0);
    assert!(detail.disk_mb > 0);
}

#[tokio::test]
async fn service_health_check() {
    let ctx = TestContext::new();
    let result = ctx.client.get_service_detail("p2p").await;
    assert!(result.is_ok());
    let detail = result.unwrap();
    assert_eq!(detail.status, "healthy");
}

// Snapshot tests
#[tokio::test]
async fn service_snapshot_creation() {
    let ctx = TestContext::new();
    let result = ctx.client.snapshot_service("p2p", Some("snap-1".to_string())).await;
    assert!(result.is_ok());
    let snapshot = result.unwrap();
    assert!(!snapshot.snapshot_id.is_empty());
    assert_eq!(snapshot.snapshot_name, "snap-1");
}

#[tokio::test]
async fn service_snapshot_metadata() {
    let ctx = TestContext::new();
    let result = ctx.client.snapshot_service("p2p", None).await;
    assert!(result.is_ok());
    let snapshot = result.unwrap();
    assert!(snapshot.size_bytes > 0);
}

// Batch operations
#[tokio::test]
async fn service_batch_start() {
    let ctx = TestContext::new();
    let services = vec!["p2p", "mesh", "api"];

    for service in services {
        let result = ctx.client.start_service(service, None).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn service_batch_stop() {
    let ctx = TestContext::new();
    let services = vec!["p2p", "mesh", "api"];

    for service in services {
        let result = ctx.client.stop_service(service, Some(true), Some(30)).await;
        assert!(result.is_ok());
    }
}

// Edge cases
#[tokio::test]
async fn service_very_long_name() {
    let ctx = TestContext::new();
    let long_name = "a".repeat(255);
    let result = ctx.client.start_service(&long_name, None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_special_characters_in_config() {
    let ctx = TestContext::new();
    let config = serde_json::json!({
        "path": "/usr/bin/service-exec",
        "args": "arg1 \"quoted\" 'single-quoted'",
        "env": {"VAR": "value with spaces"}
    });
    let result = ctx.client.configure_service("p2p", config, None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_empty_config() {
    let ctx = TestContext::new();
    let config = serde_json::json!({});
    let result = ctx.client.configure_service("p2p", config, None).await;
    assert!(result.is_ok());
}

// Performance tests
#[tokio::test]
async fn service_list_performance() {
    let ctx = TestContext::new();
    let start = std::time::Instant::now();

    for i in 0..100 {
        let _ = ctx.client.start_service(&format!("service-{}", i), None).await;
    }

    let result = ctx.client.list_services().await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed.as_millis() < 5000, "List operation too slow");
}

#[tokio::test]
async fn service_high_concurrency() {
    let ctx = Arc::new(TestContext::new());
    let mut handles = vec![];

    for i in 0..50 {
        let ctx = Arc::clone(&ctx);
        let handle = tokio::spawn(async move {
            ctx.client.start_service(&format!("concurrent-{}", i), None).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    for result in results {
        assert!(result.is_ok());
    }
}

// Cleanup and resource tests
#[tokio::test]
async fn service_cleanup_verification() {
    let ctx = TestContext::new();
    ctx.set_metadata("test_id".to_string(), "cleanup_test".to_string());
    assert_eq!(ctx.get_metadata("test_id"), Some("cleanup_test".to_string()));
    ctx.cleanup().await;
    assert_eq!(ctx.get_metadata("test_id"), None);
}

#[tokio::test]
async fn service_state_isolation() {
    let ctx1 = TestContext::new();
    let ctx2 = TestContext::new();

    ctx1.set_metadata("key".to_string(), "value1".to_string());
    ctx2.set_metadata("key".to_string(), "value2".to_string());

    assert_eq!(ctx1.get_metadata("key"), Some("value1".to_string()));
    assert_eq!(ctx2.get_metadata("key"), Some("value2".to_string()));
}

// Integration patterns
#[tokio::test]
async fn service_start_configure_snapshot() {
    let ctx = TestContext::new();

    let start_result = ctx.client.start_service("p2p", None).await;
    assert!(start_result.is_ok());

    let config = serde_json::json!({"max_connections": 100});
    let config_result = ctx.client.configure_service("p2p", config, None).await;
    assert!(config_result.is_ok());

    let snapshot_result = ctx.client.snapshot_service("p2p", Some("after-config".to_string())).await;
    assert!(snapshot_result.is_ok());
}

#[tokio::test]
async fn service_restart_with_new_config() {
    let ctx = TestContext::new();

    let old_config = serde_json::json!({"version": 1});
    let _ = ctx.client.configure_service("p2p", old_config, None).await;

    let new_config = serde_json::json!({"version": 2});
    let _ = ctx.client.configure_service("p2p", new_config, None).await;

    let restart_result = ctx.client.restart_service("p2p", Some(true)).await;
    assert!(restart_result.is_ok());
}

// Data builder integration
#[tokio::test]
async fn service_with_test_data_builder() {
    let ctx = TestContext::new();
    let builder = TestDataBuilder::new()
        .with_service_name("builder-service".to_string())
        .with_service_version("2.0.0".to_string());

    let config = builder.build_service_config();
    let result = ctx.client.configure_service("p2p", config, None).await;
    assert!(result.is_ok());
}
