//! CLI for exercising the backup-manager crate.

use backup_manager::{BackupManager, BackupType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = BackupManager::new();

    let backup_id = manager.create_backup("primary-db", BackupType::Full).await?;
    println!("Created backup {}", backup_id);

    manager.complete_backup(backup_id, 128 * 1024 * 1024).await?;
    manager.verify_backup(backup_id).await?;
    println!("Backup completed and verified");

    let snapshot_id = manager.create_snapshot(backup_id).await?;
    println!("Created snapshot {}", snapshot_id);

    manager.create_schedule("primary-db", "daily", 30).await?;
    println!("Registered daily backup schedule with 30-day retention");

    println!("Total backups tracked: {}", manager.backup_count());
    Ok(())
}
