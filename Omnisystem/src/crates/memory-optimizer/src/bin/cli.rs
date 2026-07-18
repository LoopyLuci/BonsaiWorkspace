//! CLI that exercises the garbage collector and memory pool manager.

use memory_optimizer::{GarbageCollector, MemoryPoolManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gc = GarbageCollector::new(1024 * 1024);
    gc.allocate(1, 4096).await?;
    gc.allocate(2, 8192).await?;
    println!("Blocks allocated: {}", gc.block_count());

    let freed = gc.collect().await?;
    println!("Collected {freed} bytes, blocks remaining: {}", gc.block_count());
    println!("Stats: {:?}", gc.get_statistics().await?);

    let pool_manager = MemoryPoolManager::new();
    pool_manager.create_pool(1, 4096, 8).await?;
    let block_size = pool_manager.allocate_from_pool(1).await?;
    println!("Allocated block of size {block_size} from pool");
    println!("Pool status: {:?}", pool_manager.get_pool_status(1).await?);

    Ok(())
}
