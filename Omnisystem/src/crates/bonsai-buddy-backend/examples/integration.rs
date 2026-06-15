//! Complete integration example for Bonsai Buddy Backend
//!
//! Demonstrates:
//! - Initializing the backend system
//! - Service management (list, start, stop)
//! - Environment creation and snapshots
//! - Module installation
//! - Offline-first operation
//! - Cache and sync statistics

use bonsai_buddy_backend::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    bonsai_buddy_backend::init()?;

    println!("=== Bonsai Buddy Backend Integration Example ===\n");

    // Create command handlers
    let handlers = CommandHandlers::new("demo_user".to_string())?;
    println!("✓ Backend initialized for user: demo_user\n");

    // Example 1: Get system summary
    println!("--- System Summary ---");
    let summary = handlers.get_summary().await?;
    println!("{}\n", serde_json::to_string_pretty(&summary)?);

    // Example 2: Create environments
    println!("--- Creating Environments ---");
    let env1 = handlers.create_environment("development".to_string()).await?;
    println!("✓ Created environment: {} ({})", env1.name, env1.id);

    let env2 = handlers.create_environment("production".to_string()).await?;
    println!("✓ Created environment: {} ({})\n", env2.name, env2.id);

    // Example 3: List environments
    println!("--- Listing Environments ---");
    let envs = handlers.list_environments().await?;
    for env in envs {
        println!("  - {}: {} (running: {})", env.name, env.id, env.is_running);
    }
    println!();

    // Example 4: Create snapshot
    println!("--- Creating Environment Snapshot ---");
    let snapshot_hash = handlers
        .snapshot_environment(env1.id.clone(), "snapshot-v1.0".to_string())
        .await?;
    println!("✓ Snapshot created: {}\n", snapshot_hash);

    // Example 5: Offline mode - try to install module (should queue)
    println!("--- Offline Mode Test ---");
    handlers.set_online(false).await?;
    println!("✓ Set offline mode");

    match handlers
        .install_module("test-module".to_string(), "1.0.0".to_string())
        .await
    {
        Ok(_) => println!("  Module installed (unexpected in offline mode)"),
        Err(e) => println!("  Module installation queued (expected): {}", e),
    }

    // Check queue status
    let queue_stats = handlers.get_queue_stats().await?;
    println!("✓ Queue stats: {}\n", serde_json::to_string_pretty(&queue_stats)?);

    // Example 6: Go back online
    println!("--- Reconnection Simulation ---");
    handlers.set_online(true).await?;
    println!("✓ Back online (queue processing would occur)\n");

    // Example 7: Cache statistics
    println!("--- Cache Statistics ---");
    let cache_stats = handlers.get_cache_stats().await?;
    println!("{}\n", serde_json::to_string_pretty(&cache_stats)?);

    // Example 8: Sync engine statistics
    println!("--- Sync Engine Statistics ---");
    let sync_stats = handlers.get_sync_stats().await?;
    println!("{}\n", serde_json::to_string_pretty(&sync_stats)?);

    // Example 9: Debug snapshot
    println!("--- Full Debug Snapshot ---");
    let debug_snapshot = handlers.get_debug_snapshot().await?;
    println!("{}\n", serde_json::to_string_pretty(&debug_snapshot)?);

    // Example 10: Cleanup
    println!("--- Cleanup Operations ---");
    let expired_count = handlers.cleanup_cache().await?;
    println!("✓ Cleaned up {} expired cache entries", expired_count);

    println!("\n=== Integration Example Complete ===");
    Ok(())
}
