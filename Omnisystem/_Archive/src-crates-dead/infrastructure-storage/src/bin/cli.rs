//! CLI

use infrastructure_storage::{BucketName, InMemoryObjectStorage, ObjectKey, ObjectStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryObjectStorage::new();
    let bucket = BucketName("backups".to_string());
    storage.create_bucket(bucket.clone()).await?;

    let key = ObjectKey("hello.txt".to_string());
    let meta = storage.put_object(&bucket, key, b"hello world".to_vec()).await?;
    println!("Stored object: {} ({} bytes)", meta.key.0, meta.size);

    Ok(())
}
