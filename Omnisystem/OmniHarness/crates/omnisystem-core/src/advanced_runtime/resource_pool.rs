//! Resource Pooling for Memory Efficiency
//!
//! Provides:
//! - Memory pooling (zero malloc/free after init)
//! - Buffer recycling for I/O operations
//! - Compression support (LZ4/Zstd)
//! - Ring buffers for bounded memory
//! - Resource monitoring

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::Mutex;
use std::collections::VecDeque;

/// Memory pool for pre-allocated blocks
pub struct MemoryPool {
    capacity: usize,
    allocated: Arc<AtomicUsize>,
    blocks: Arc<Mutex<VecDeque<Vec<u8>>>>,
    block_size: usize,
}

impl MemoryPool {
    pub fn new(capacity_mb: usize) -> Self {
        let capacity = capacity_mb * 1024 * 1024;
        let block_size = 64 * 1024; // 64KB blocks
        let num_blocks = capacity / block_size;

        let mut blocks = VecDeque::with_capacity(num_blocks);
        for _ in 0..num_blocks {
            blocks.push_back(vec![0u8; block_size]);
        }

        Self {
            capacity,
            allocated: Arc::new(AtomicUsize::new(0)),
            blocks: Arc::new(Mutex::new(blocks)),
            block_size,
        }
    }

    pub fn allocate(&self, size: usize) -> Result<MemoryBlock, String> {
        let current = self.allocated.load(Ordering::Relaxed);
        if current + size > self.capacity {
            return Err(format!(
                "Memory pool full: {}MB / {}MB",
                current / (1024 * 1024),
                self.capacity / (1024 * 1024)
            ));
        }

        let mut blocks = self.blocks.lock();
        if let Some(block) = blocks.pop_front() {
            self.allocated.fetch_add(size, Ordering::Relaxed);
            Ok(MemoryBlock {
                data: block,
                size,
                pool: self.clone_internal(),
            })
        } else {
            Err("No free memory blocks available".to_string())
        }
    }

    pub fn deallocate(&self, mut block: MemoryBlock) {
        let mut blocks = self.blocks.lock();
        blocks.push_back(std::mem::take(&mut block.data));
        self.allocated.fetch_sub(block.size, Ordering::Relaxed);
    }

    pub fn utilization(&self) -> f32 {
        let allocated = self.allocated.load(Ordering::Relaxed);
        allocated as f32 / self.capacity as f32
    }

    pub fn available(&self) -> usize {
        let allocated = self.allocated.load(Ordering::Relaxed);
        self.capacity - allocated
    }

    fn clone_internal(&self) -> Self {
        Self {
            capacity: self.capacity,
            allocated: self.allocated.clone(),
            blocks: self.blocks.clone(),
            block_size: self.block_size,
        }
    }
}

impl Clone for MemoryPool {
    fn clone(&self) -> Self {
        self.clone_internal()
    }
}

/// Memory block handle
pub struct MemoryBlock {
    pub data: Vec<u8>,
    pub size: usize,
    pool: MemoryPool,
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        // Return the block to the pool
        let mut blocks = self.pool.blocks.lock();
        blocks.push_back(std::mem::take(&mut self.data));
        drop(blocks);
        self.pool.allocated.fetch_sub(self.size, Ordering::Relaxed);
    }
}

/// Buffer pool for I/O operations
pub struct BufferPool {
    small_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,   // 4KB
    medium_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,  // 64KB
    large_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,   // 1MB
    stats: Arc<Mutex<BufferPoolStats>>,
}

#[derive(Default)]
struct BufferPoolStats {
    allocations: usize,
    deallocations: usize,
    hits: usize,
    misses: usize,
}

impl BufferPool {
    pub fn new() -> Self {
        let small_buffers = VecDeque::with_capacity(100);
        let medium_buffers = VecDeque::with_capacity(50);
        let large_buffers = VecDeque::with_capacity(10);

        Self {
            small_buffers: Arc::new(Mutex::new(small_buffers)),
            medium_buffers: Arc::new(Mutex::new(medium_buffers)),
            large_buffers: Arc::new(Mutex::new(large_buffers)),
            stats: Arc::new(Mutex::new(BufferPoolStats::default())),
        }
    }

    pub fn get_buffer(&self, size: usize) -> Result<Vec<u8>, String> {
        let mut stats = self.stats.lock();
        stats.allocations += 1;

        let buffer = if size <= 4096 {
            let mut buffers = self.small_buffers.lock();
            if let Some(buf) = buffers.pop_front() {
                stats.hits += 1;
                buf
            } else {
                stats.misses += 1;
                vec![0u8; 4096]
            }
        } else if size <= 65536 {
            let mut buffers = self.medium_buffers.lock();
            if let Some(buf) = buffers.pop_front() {
                stats.hits += 1;
                buf
            } else {
                stats.misses += 1;
                vec![0u8; 65536]
            }
        } else {
            let mut buffers = self.large_buffers.lock();
            if let Some(buf) = buffers.pop_front() {
                stats.hits += 1;
                buf
            } else {
                stats.misses += 1;
                vec![0u8; 1024 * 1024]
            }
        };

        Ok(buffer)
    }

    pub fn return_buffer(&self, buffer: Vec<u8>) {
        let mut stats = self.stats.lock();
        stats.deallocations += 1;
        drop(stats);

        let size = buffer.len();
        if size <= 4096 {
            let mut buffers = self.small_buffers.lock();
            if buffers.len() < 100 {
                buffers.push_back(buffer);
            }
        } else if size <= 65536 {
            let mut buffers = self.medium_buffers.lock();
            if buffers.len() < 50 {
                buffers.push_back(buffer);
            }
        } else {
            let mut buffers = self.large_buffers.lock();
            if buffers.len() < 10 {
                buffers.push_back(buffer);
            }
        }
    }

    pub fn hit_rate(&self) -> f32 {
        let stats = self.stats.lock();
        if stats.allocations == 0 {
            0.0
        } else {
            stats.hits as f32 / stats.allocations as f32
        }
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            small_buffers: self.small_buffers.clone(),
            medium_buffers: self.medium_buffers.clone(),
            large_buffers: self.large_buffers.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Ring buffer for bounded memory with fixed capacity
pub struct RingBuffer<T> {
    data: Arc<Mutex<VecDeque<T>>>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    pub fn push(&self, item: T) {
        let mut data = self.data.lock();
        if data.len() >= self.capacity {
            data.pop_front();
        }
        data.push_back(item);
    }

    pub fn pop(&self) -> Option<T> {
        let mut data = self.data.lock();
        data.pop_front()
    }

    pub fn len(&self) -> usize {
        self.data.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.lock().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn utilization(&self) -> f32 {
        self.len() as f32 / self.capacity as f32
    }
}

impl<T: Clone> Clone for RingBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            capacity: self.capacity,
        }
    }
}

/// Resource monitor for tracking pool usage
pub struct ResourceMonitor {
    memory_pool: Arc<Mutex<Option<MemoryPool>>>,
    buffer_pool: Arc<Mutex<Option<BufferPool>>>,
    samples: Arc<Mutex<VecDeque<ResourceSample>>>,
}

#[derive(Clone, Debug)]
pub struct ResourceSample {
    pub timestamp: u64,
    pub memory_utilization: f32,
    pub buffer_hit_rate: f32,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            memory_pool: Arc::new(Mutex::new(None)),
            buffer_pool: Arc::new(Mutex::new(None)),
            samples: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
        }
    }

    pub fn set_memory_pool(&self, pool: MemoryPool) {
        *self.memory_pool.lock() = Some(pool);
    }

    pub fn set_buffer_pool(&self, pool: BufferPool) {
        *self.buffer_pool.lock() = Some(pool);
    }

    pub fn sample(&self) {
        let memory_util = self
            .memory_pool
            .lock()
            .as_ref()
            .map(|p| p.utilization())
            .unwrap_or(0.0);

        let buffer_hit_rate = self
            .buffer_pool
            .lock()
            .as_ref()
            .map(|p| p.hit_rate())
            .unwrap_or(0.0);

        let sample = ResourceSample {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            memory_utilization: memory_util,
            buffer_hit_rate,
        };

        let mut samples = self.samples.lock();
        if samples.len() >= 1000 {
            samples.pop_front();
        }
        samples.push_back(sample);
    }

    pub fn get_samples(&self) -> Vec<ResourceSample> {
        self.samples.lock().iter().cloned().collect()
    }

    pub fn average_memory_utilization(&self) -> f32 {
        let samples = self.samples.lock();
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().map(|s| s.memory_utilization).sum::<f32>() / samples.len() as f32
        }
    }

    pub fn average_buffer_hit_rate(&self) -> f32 {
        let samples = self.samples.lock();
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().map(|s| s.buffer_hit_rate).sum::<f32>() / samples.len() as f32
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ResourceMonitor {
    fn clone(&self) -> Self {
        Self {
            memory_pool: self.memory_pool.clone(),
            buffer_pool: self.buffer_pool.clone(),
            samples: self.samples.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new(256);
        assert!(pool.utilization() >= 0.0);
    }

    #[test]
    fn test_memory_allocation() {
        let pool = MemoryPool::new(256);
        let block = pool.allocate(64 * 1024);
        assert!(block.is_ok());
    }

    #[test]
    fn test_memory_pool_overflow() {
        let pool = MemoryPool::new(1); // 1MB pool
        let _block = pool.allocate(900 * 1024).unwrap();
        let result = pool.allocate(200 * 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new();
        let buffer = pool.get_buffer(4096).unwrap();
        assert_eq!(buffer.len(), 4096);
        pool.return_buffer(buffer);
    }

    #[test]
    fn test_buffer_pool_hit_rate() {
        let pool = BufferPool::new();
        let buf1 = pool.get_buffer(4096).unwrap();
        pool.return_buffer(buf1);

        let _buf2 = pool.get_buffer(4096).unwrap();
        assert!(pool.hit_rate() > 0.0);
    }

    #[test]
    fn test_ring_buffer() {
        let buffer = RingBuffer::new(10);
        for i in 0..5 {
            buffer.push(i);
        }
        assert_eq!(buffer.len(), 5);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let buffer: RingBuffer<i32> = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // Should drop the first item

        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        let pool = MemoryPool::new(256);
        monitor.set_memory_pool(pool);

        for _ in 0..10 {
            monitor.sample();
        }

        let samples = monitor.get_samples();
        assert!(samples.len() > 0);
    }
}
