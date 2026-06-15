// OMNISYSTEM CACHE FRAMEWORK
// Multi-tier caching with TTL, invalidation, and statistics

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// CACHE ENTRY
// ============================================================================

#[derive(Debug, Clone)]
pub enum CacheValue {
    String(String),
    Integer(i64),
    Float(f64),
    Json(String),
    Binary(Vec<u8>),
}

pub struct CacheEntry {
    pub value: CacheValue,
    pub created_at: Instant,
    pub accessed_at: Instant,
    pub ttl: Option<Duration>,
    pub hit_count: usize,
}

impl CacheEntry {
    pub fn new(value: CacheValue, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        CacheEntry {
            value,
            created_at: now,
            accessed_at: now,
            ttl,
            hit_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            self.created_at.elapsed() > ttl
        } else {
            false
        }
    }

    pub fn access(&mut self) {
        self.accessed_at = Instant::now();
        self.hit_count += 1;
    }
}

// ============================================================================
// IN-MEMORY CACHE
// ============================================================================

pub struct MemoryCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<Mutex<CacheStats>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_accessed: u64,
}

impl MemoryCache {
    pub fn new(max_size: usize) -> Self {
        MemoryCache {
            data: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                total_accessed: 0,
            })),
            max_size,
        }
    }

    pub fn get(&self, key: &str) -> Option<CacheValue> {
        let mut data = self.data.write().unwrap();

        if let Some(entry) = data.get_mut(key) {
            if entry.is_expired() {
                data.remove(key);
                let mut stats = self.stats.lock().unwrap();
                stats.misses += 1;
                return None;
            }

            entry.access();
            let mut stats = self.stats.lock().unwrap();
            stats.hits += 1;
            stats.total_accessed += 1;

            return Some(entry.value.clone());
        }

        let mut stats = self.stats.lock().unwrap();
        stats.misses += 1;
        None
    }

    pub fn set(&self, key: String, value: CacheValue, ttl: Option<Duration>) -> Result<(), String> {
        let mut data = self.data.write().unwrap();

        if data.len() >= self.max_size && !data.contains_key(&key) {
            // Evict oldest entry (simple LRU)
            if let Some((oldest_key, _)) = data.iter()
                .min_by_key(|(_, entry)| entry.accessed_at) {
                let oldest_key = oldest_key.clone();
                data.remove(&oldest_key);
                let mut stats = self.stats.lock().unwrap();
                stats.evictions += 1;
            }
        }

        data.insert(key, CacheEntry::new(value, ttl));
        println!("💾 Cached with TTL: {:?}", ttl);
        Ok(())
    }

    pub fn delete(&self, key: &str) -> bool {
        let mut data = self.data.write().unwrap();
        data.remove(key).is_some()
    }

    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
        println!("🗑️  Cache cleared");
    }

    pub fn size(&self) -> usize {
        self.data.read().unwrap().len()
    }

    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.lock().unwrap();
        let total = (stats.hits + stats.misses) as f64;
        if total > 0.0 {
            (stats.hits as f64) / total
        } else {
            0.0
        }
    }
}

// ============================================================================
// DISTRIBUTED CACHE (Multi-tier)
// ============================================================================

pub struct DistributedCache {
    l1_cache: Arc<MemoryCache>,
    l2_cache: Arc<MemoryCache>,
    l3_store: Arc<Mutex<HashMap<String, String>>>,
    config: CacheConfig,
}

pub struct CacheConfig {
    pub l1_size: usize,
    pub l2_size: usize,
    pub l1_ttl: Option<Duration>,
    pub l2_ttl: Option<Duration>,
    pub l3_ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            l1_size: 100,
            l2_size: 1000,
            l1_ttl: Some(Duration::from_secs(300)),
            l2_ttl: Some(Duration::from_secs(3600)),
            l3_ttl: Some(Duration::from_secs(86400)),
        }
    }
}

impl DistributedCache {
    pub fn new(config: CacheConfig) -> Self {
        DistributedCache {
            l1_cache: Arc::new(MemoryCache::new(config.l1_size)),
            l2_cache: Arc::new(MemoryCache::new(config.l2_size)),
            l3_store: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn get(&self, key: &str) -> Option<CacheValue> {
        // L1 cache
        if let Some(value) = self.l1_cache.get(key) {
            println!("⚡ L1 cache hit: {}", key);
            return Some(value);
        }

        // L2 cache
        if let Some(value) = self.l2_cache.get(key) {
            println!("📊 L2 cache hit: {}", key);
            // Promote to L1
            let _ = self.l1_cache.set(key.to_string(), value.clone(), self.config.l1_ttl);
            return Some(value);
        }

        // L3 store (simplified)
        let l3 = self.l3_store.lock().unwrap();
        if let Some(json_str) = l3.get(key) {
            println!("💾 L3 cache hit: {}", key);
            // Promote to L2
            let _ = self.l2_cache.set(key.to_string(),
                CacheValue::Json(json_str.clone()),
                self.config.l2_ttl);
            return Some(CacheValue::Json(json_str.clone()));
        }

        println!("❌ Cache miss: {}", key);
        None
    }

    pub fn set(&self, key: String, value: CacheValue) -> Result<(), String> {
        self.l1_cache.set(key.clone(), value.clone(), self.config.l1_ttl)?;
        println!("✅ Set in L1 cache: {}", key);
        Ok(())
    }

    pub fn invalidate(&self, key: &str) {
        self.l1_cache.delete(key);
        self.l2_cache.delete(key);
        let mut l3 = self.l3_store.lock().unwrap();
        l3.remove(key);
        println!("🔄 Invalidated across all tiers: {}", key);
    }

    pub fn invalidate_pattern(&self, pattern: &str) {
        // Simple pattern matching (prefix-based)
        let mut l3 = self.l3_store.lock().unwrap();
        l3.retain(|k, _| !k.starts_with(pattern));
        println!("🔄 Invalidated pattern: {}", pattern);
    }

    pub fn stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        stats.insert("L1".to_string(), self.l1_cache.stats());
        stats.insert("L2".to_string(), self.l2_cache.stats());
        stats
    }

    pub fn warm(&self, entries: Vec<(String, CacheValue)>) -> Result<(), String> {
        for (key, value) in entries {
            self.l1_cache.set(key, value, self.config.l1_ttl)?;
        }
        println!("🔥 Cache warmed with entries");
        Ok(())
    }
}

// ============================================================================
// CACHE INVALIDATION STRATEGY
// ============================================================================

pub enum InvalidationStrategy {
    TTL,
    LRU,
    LFU,
    FIFO,
    Manual,
}

pub struct CacheInvalidator {
    strategy: InvalidationStrategy,
    listeners: Arc<Mutex<Vec<Box<dyn Fn(&str) + Send + Sync>>>>,
}

impl CacheInvalidator {
    pub fn new(strategy: InvalidationStrategy) -> Self {
        CacheInvalidator {
            strategy,
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn on_invalidate<F>(&self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.listeners.lock().unwrap().push(Box::new(callback));
    }

    pub fn notify(&self, key: &str) {
        for listener in self.listeners.lock().unwrap().iter() {
            listener(key);
        }
    }
}

// ============================================================================
// CACHE STATISTICS & MONITORING
// ============================================================================

pub struct CacheMonitor {
    caches: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<CacheEvent>>>,
}

#[derive(Debug, Clone)]
pub struct CacheEvent {
    pub timestamp: Instant,
    pub event_type: String,
    pub key: String,
    pub details: String,
}

impl CacheMonitor {
    pub fn new() -> Self {
        CacheMonitor {
            caches: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log_event(&self, event: CacheEvent) {
        self.events.lock().unwrap().push(event);
    }

    pub fn get_events(&self, limit: usize) -> Vec<CacheEvent> {
        let events = self.events.lock().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    pub fn report(&self) {
        let events = self.events.lock().unwrap();
        println!("\n📊 CACHE MONITORING REPORT");
        println!("Total events: {}", events.len());
        println!("\nRecent events:");
        for event in events.iter().rev().take(5) {
            println!("  - {}: {} ({})", event.event_type, event.key, event.details);
        }
        println!();
    }
}

// ============================================================================
// EXAMPLE APPLICATION
// ============================================================================

pub fn example_cache_system() -> Result<(), Box<dyn std::error::Error>> {
    let config = CacheConfig::default();
    let cache = DistributedCache::new(config);

    // Set values
    cache.set("user:1".to_string(), CacheValue::Json(r#"{"id": 1, "name": "Alice"}"#.to_string()))?;
    cache.set("user:2".to_string(), CacheValue::String("Bob".to_string()))?;

    // Get values
    cache.get("user:1");
    cache.get("user:2");
    cache.get("user:3"); // Miss

    // Invalidate
    cache.invalidate("user:1");

    // Pattern invalidation
    cache.invalidate_pattern("session:");

    // Warm cache
    let warm_entries = vec![
        ("config:db".to_string(), CacheValue::String("localhost:5432".to_string())),
        ("config:cache".to_string(), CacheValue::String("redis:6379".to_string())),
    ];
    cache.warm(warm_entries)?;

    // Monitor
    let monitor = CacheMonitor::new();
    monitor.log_event(CacheEvent {
        timestamp: Instant::now(),
        event_type: "HIT".to_string(),
        key: "user:1".to_string(),
        details: "L1 hit".to_string(),
    });
    monitor.report();

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry::new(
            CacheValue::String("test".to_string()),
            Some(Duration::from_secs(60)),
        );
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_memory_cache() {
        let cache = MemoryCache::new(100);
        cache.set("key".to_string(), CacheValue::String("value".to_string()), None).unwrap();
        assert_eq!(cache.size(), 1);
        assert_eq!(cache.get("key"), Some(CacheValue::String("value".to_string())));
    }

    #[test]
    fn test_cache_stats() {
        let cache = MemoryCache::new(100);
        cache.set("k1".to_string(), CacheValue::String("v1".to_string()), None).unwrap();
        cache.get("k1");
        cache.get("k2");

        let stats = cache.stats();
        assert!(stats.hits > 0);
        assert!(stats.misses > 0);
    }

    #[test]
    fn test_distributed_cache() {
        let config = CacheConfig::default();
        let cache = DistributedCache::new(config);

        cache.set("key".to_string(), CacheValue::String("value".to_string())).unwrap();
        assert!(cache.get("key").is_some());
    }

    #[test]
    fn test_cache_invalidation() {
        let config = CacheConfig::default();
        let cache = DistributedCache::new(config);

        cache.set("user:1".to_string(), CacheValue::String("Alice".to_string())).unwrap();
        cache.invalidate("user:1");
        assert!(cache.get("user:1").is_none());
    }

    #[test]
    fn test_cache_monitor() {
        let monitor = CacheMonitor::new();
        monitor.log_event(CacheEvent {
            timestamp: Instant::now(),
            event_type: "HIT".to_string(),
            key: "test".to_string(),
            details: "test details".to_string(),
        });

        let events = monitor.get_events(10);
        assert_eq!(events.len(), 1);
    }
}
