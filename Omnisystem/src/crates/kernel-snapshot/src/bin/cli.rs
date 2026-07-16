//! Kernel-snapshot CLI: creates a vault, snapshots it, restores it from the
//! snapshot hash, then tears both vaults down -- exercising the full
//! create/snapshot/restore/destroy lifecycle.

use kernel_snapshot::{create_vault, destroy_vault, restore_vault, snapshot_vault, CapabilityTable};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut caps = CapabilityTable::new();
    caps.add_capability("net.connect".to_string());
    caps.add_capability("fs.read".to_string());

    let vault_id = create_vault("demo-binary", caps.clone())?;
    println!("Created vault {}", vault_id);

    let hash = snapshot_vault(vault_id)?;
    println!("Snapshotted vault {} as {}", vault_id, hash.to_hex());

    let restored_id = restore_vault(&hash, caps)?;
    println!("Restored snapshot as new vault {}", restored_id);

    destroy_vault(vault_id)?;
    destroy_vault(restored_id)?;
    println!("Destroyed both vaults");

    Ok(())
}
