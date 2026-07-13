//! CLI

use storage_backend::StorageBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = StorageBackend::new();
    let entry = backend.put("key1", b"value1").await?;
    println!("Put entry: {}", entry.key);

    let fetched = backend.get("key1").await?;
    println!("Fetched: {} bytes", fetched.size_bytes);

    println!("Total entries: {}", backend.entry_count());
    Ok(())
}
