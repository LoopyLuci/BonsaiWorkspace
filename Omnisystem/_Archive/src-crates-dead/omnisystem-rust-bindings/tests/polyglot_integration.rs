//! Integration test demonstrating polyglot communication between Rust and Go

use omnisystem_rust_bindings::prelude::*;
use omnisystem_rust_bindings::polyglot::{PolyglotRuntime, Language};

#[tokio::test]
async fn test_polyglot_runtime_initialization() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");
    let stats = runtime.get_stats();

    // Verify kernel is initialized
    assert!(stats.total_memory_bytes > 0);
    assert_eq!(stats.process_count, 0); // No processes created yet
}

#[tokio::test]
async fn test_language_registration() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");

    // Create polyglot runtime
    let polyglot = PolyglotRuntime::new(
        runtime.ffi_registry().clone(),
        runtime.module_loader().clone(),
    );

    // Register Rust
    let result = polyglot.register_language(Language::Rust).await;
    assert!(result.is_ok());
    assert!(polyglot.is_language_loaded(Language::Rust));

    // Register Go
    let result = polyglot.register_language(Language::Go).await;
    assert!(result.is_ok());
    assert!(polyglot.is_language_loaded(Language::Go));

    // Verify both are loaded
    let languages = polyglot.list_loaded_languages();
    assert!(languages.contains(&"rust".to_string()));
    assert!(languages.contains(&"go".to_string()));
}

#[tokio::test]
async fn test_async_task_execution() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");

    // Spawn async tasks
    let task1 = runtime.spawn(async { 42 });
    let task2 = runtime.spawn(async { "hello" });
    let task3 = runtime.spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        100
    });

    // Wait for results
    let r1 = task1.await.expect("Task 1 failed");
    let r2 = task2.await.expect("Task 2 failed");
    let r3 = task3.await.expect("Task 3 failed");

    assert_eq!(r1, 42);
    assert_eq!(r2, "hello");
    assert_eq!(r3, 100);
}

#[tokio::test]
async fn test_ffi_bridge_communication() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");

    // Create FFI bridge
    let ffi_bridge = FFIBridge::new(runtime.ffi_registry().clone());

    // Register modules
    ffi_bridge.register_module("rust_service", (1, 0, 0));
    ffi_bridge.register_module("go_service", (1, 0, 0));

    // Verify registration
    let modules = ffi_bridge.list_modules();
    assert!(modules.contains(&"rust_service".to_string()));
    assert!(modules.contains(&"go_service".to_string()));
}

#[tokio::test]
async fn test_multiprocess_execution() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");

    let kernel = runtime.kernel();
    let process_mgr = kernel.process();

    // Create processes
    let p1 = process_mgr
        .create_process(None)
        .expect("Failed to create process 1");
    let p2 = process_mgr
        .create_process(None)
        .expect("Failed to create process 2");

    // Verify processes
    assert_ne!(p1.id, p2.id);
    assert_eq!(process_mgr.process_count(), 2);
}

#[tokio::test]
async fn test_memory_allocation() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");

    let kernel = runtime.kernel();
    let memory_mgr = kernel.memory();

    let initial_stats = memory_mgr.get_stats();
    let initial_allocated = initial_stats.allocated_memory_bytes;

    // Allocate pages
    let pages = memory_mgr
        .allocate_pages(10)
        .expect("Failed to allocate pages");

    assert_eq!(pages.len(), 10);

    let after_stats = memory_mgr.get_stats();
    assert!(after_stats.allocated_memory_bytes > initial_allocated);
}

#[test]
fn test_polyglot_language_enum() {
    let rust = Language::Rust;
    assert_eq!(rust.as_str(), "rust");
    assert_eq!(rust.version(), (1, 0, 0));

    let go = Language::Go;
    assert_eq!(go.as_str(), "go");

    let python = Language::Python;
    assert_eq!(python.as_str(), "python");
}

#[tokio::test]
async fn test_all_languages_registration() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");
    let polyglot = PolyglotRuntime::new(
        runtime.ffi_registry().clone(),
        runtime.module_loader().clone(),
    );

    // Register all supported languages
    for lang in PolyglotRuntime::supported_languages() {
        let result = polyglot.register_language(lang).await;
        assert!(result.is_ok(), "Failed to register language: {}", lang.as_str());
    }

    // Verify all languages loaded
    assert_eq!(polyglot.loaded_language_count(), 10);

    let languages = polyglot.list_loaded_languages();
    assert!(languages.len() == 10);
}

#[tokio::test]
async fn test_system_health_check() {
    let runtime = OmnisystemRuntime::new().await.expect("Failed to create runtime");
    let stats = runtime.get_stats();

    // System should be healthy
    assert!(stats.total_memory_bytes > 0);
    assert!(stats.free_memory_bytes > 0);
    assert!(stats.loaded_modules >= 0);
    assert!(stats.process_count >= 0);
}
