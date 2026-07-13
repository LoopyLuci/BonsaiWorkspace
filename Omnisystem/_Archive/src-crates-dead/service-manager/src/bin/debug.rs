//! Debug/demo binary for SLM

use service_manager::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    println!("=== Service Lifecycle Manager Debug ===\n");

    // Initialize components
    let kernel = KernelAdapter::new();
    let lifecycle = LifecycleManager::new(kernel);
    let registry = ServiceRegistry::new();

    // Register some sample services
    let fax_manifest = ServiceManifest {
        name: "fax".to_string(),
        version: "2.0.0".to_string(),
        binary_hash: "blake3:fax_binary_hash".to_string(),
        capabilities_required: vec!["USB".to_string(), "NET:outbound".to_string()],
        quota: ResourceQuota {
            memory_mb: 512,
            cpu_cores: 2.0,
            cpu_percent_max: 80,
            iops_limit: 1000,
            max_snapshots: 5,
            max_snapshot_size_mb: 256,
        },
        idle_timeout_secs: 300,
        archive_after_hours: 24,
        heartbeat_interval_secs: 10,
        heartbeat_timeout_secs: 5,
        signature: "bls_signature_here".to_string(),
    };

    let scanner_manifest = ServiceManifest {
        name: "scanner".to_string(),
        version: "1.5.0".to_string(),
        binary_hash: "blake3:scanner_binary_hash".to_string(),
        capabilities_required: vec!["USB".to_string()],
        quota: ResourceQuota::default(),
        idle_timeout_secs: 180,
        archive_after_hours: 12,
        heartbeat_interval_secs: 10,
        heartbeat_timeout_secs: 5,
        signature: "bls_signature_here".to_string(),
    };

    registry.register_service(fax_manifest.clone())?;
    registry.register_service(scanner_manifest.clone())?;

    println!("✓ Registered {} services\n", registry.list_services().len());

    // Demo 1: Spawn a service
    println!("=== Demo 1: Spawn FAX Service ===");
    let mut fax_instance = lifecycle.spawn_service(fax_manifest.clone()).await?;
    println!("✓ Spawned FAX service");
    println!("  - Instance ID: {}", fax_instance.instance_id);
    println!("  - Vault ID: {:?}", fax_instance.vault_id);
    println!("  - State: {:?}\n", fax_instance.state);

    // Demo 2: Access service (touch)
    println!("=== Demo 2: Touch Service ===");
    lifecycle.touch_service(&mut fax_instance);
    println!("✓ Touched service");
    println!("  - Last access: {}", fax_instance.last_access_timestamp);
    println!("  - Consecutive failures: {}\n", fax_instance.consecutive_failures);

    // Demo 3: Pause and snapshot
    println!("=== Demo 3: Pause & Snapshot ===");
    lifecycle.pause_and_snapshot(&mut fax_instance).await?;
    println!("✓ Paused and snapshotted service");
    println!(
        "  - State: {:?}",
        fax_instance.state
    );
    println!(
        "  - Snapshot: {}",
        fax_instance.latest_snapshot.as_ref().unwrap().hash
    );
    println!(
        "  - Snapshot size: {} bytes\n",
        fax_instance.latest_snapshot.as_ref().unwrap().size_bytes
    );

    // Demo 4: Restore from snapshot
    println!("=== Demo 4: Restore from Snapshot ===");
    lifecycle.restore_from_snapshot(&mut fax_instance).await?;
    println!("✓ Restored from snapshot");
    println!("  - State: {:?}", fax_instance.state);
    println!("  - New vault ID: {:?}\n", fax_instance.vault_id);

    // Demo 5: Multiple snapshots with archival
    println!("=== Demo 5: Snapshot Rotation ===");
    for i in 0..7 {
        lifecycle.pause_and_snapshot(&mut fax_instance).await?;
        println!("  Snapshot {}: created", i + 1);
        lifecycle.restore_from_snapshot(&mut fax_instance).await?;
    }
    lifecycle.archive_old_snapshots(&mut fax_instance).await?;
    println!(
        "✓ Created 7 snapshots, kept {}/5 (others archived)\n",
        fax_instance.snapshots.len()
    );

    // Demo 6: Service registry queries
    println!("=== Demo 6: Service Registry ===");
    println!("✓ Registered services:");
    for service in registry.list_services() {
        let manifest = registry.get_service(&service)?;
        println!(
            "  - {}: {} (memory: {}MB, timeout: {}s)",
            manifest.name, manifest.version, manifest.quota.memory_mb, manifest.idle_timeout_secs
        );
    }

    println!("\n=== All Demos Complete ===");
    println!("✓ SLM functioning correctly");
    println!("✓ Ready for Phase 3 (UMS Integration) implementation");

    Ok(())
}
