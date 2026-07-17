//! CLI for exercising the storage-backend LSM-tree engine.

use storage_backend::StorageBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = StorageBackend::new();
    let entry = backend.put("key1", b"value1").await?;
    println!("Put entry: {}", entry.key);

    let fetched = backend.get("key1").await?;
    println!("Fetched: {} bytes", fetched.size_bytes);

    backend.put("key2", b"value2_longer").await?;
    let log = backend.log_write_ahead(1, "PUT key1,key2").await?;
    backend.mark_wal_synced(log.log_id).await?;
    println!("WAL entry {} synced: true", log.log_id);

    let snapshot = backend.flush_memtable().await?;
    println!("Flushed memtable snapshot: {} entries, {} bytes", snapshot.entry_count, snapshot.size_bytes);

    let sstable = backend.create_sstable(0, "key1", "key2").await?;
    println!("Created SSTable: {} entries, {} bytes", sstable.entry_count, sstable.size_bytes);

    let task = backend.start_compaction(0, vec![sstable.sstable_id]).await?;
    backend.complete_compaction(task.task_id).await?;
    println!("Compaction {} completed", task.task_id);

    println!("Total entries: {}", backend.entry_count());
    Ok(())
}
