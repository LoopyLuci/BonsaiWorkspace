use serde_json::Value;
/// Tenancy-aware LRU cache for tool results.
///
/// Cache key includes: tool name, canonical args hash, profile_id, workspace_path.
/// This prevents cross-profile or cross-workspace data leakage.
/// Tools with SideEffectProfile != None must never be cached (enforced by
/// the caller checking tool.cache_ttl_secs()).
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const DEFAULT_CAPACITY: usize = 256;

// ── Cache key ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    tool: String,
    args_hash: u64,
    profile_id: String,
    workspace_path: String, // "" when no workspace
}

impl CacheKey {
    fn new(tool: &str, args: &Value, profile_id: &str, workspace: Option<&str>) -> Self {
        Self {
            tool: tool.to_string(),
            args_hash: hash_value(args),
            profile_id: profile_id.to_string(),
            workspace_path: workspace.unwrap_or("").to_string(),
        }
    }
}

fn hash_value(v: &Value) -> u64 {
    // Canonical JSON (sorted keys) → FNV-1a hash
    let canonical = canonical_json(v);
    let mut h: u64 = 14695981039346656037;
    for b in canonical.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Sort object keys recursively for a stable hash regardless of insertion order.
fn canonical_json(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_default(),
                        canonical_json(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(arr) => {
            format!(
                "[{}]",
                arr.iter().map(canonical_json).collect::<Vec<_>>().join(",")
            )
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

// ── Cache entry ───────────────────────────────────────────────────────────────

struct Entry {
    value: Value,
    expires_at: Instant,
    hits: u32,
}

// ── ToolCache ─────────────────────────────────────────────────────────────────

pub struct ToolCache {
    inner: Mutex<Inner>,
    capacity: usize,
}

struct Inner {
    map: HashMap<CacheKey, Entry>,
    /// Insertion-order list for LRU eviction. Stores keys in eviction order.
    order: std::collections::VecDeque<CacheKey>,
}

impl ToolCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                map: HashMap::with_capacity(capacity),
                order: std::collections::VecDeque::with_capacity(capacity),
            }),
            capacity,
        }
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// No-op cache — used in contexts where caching is explicitly disabled
    /// (e.g. the parallel executor's inner calls for streaming tools).
    pub fn noop() -> Self {
        Self::new(0)
    }

    pub fn get(
        &self,
        tool: &str,
        args: &Value,
        profile_id: &str,
        workspace: Option<&str>,
    ) -> Option<Value> {
        if self.capacity == 0 {
            return None;
        }
        let key = CacheKey::new(tool, args, profile_id, workspace);
        let mut inner = self.inner.lock().unwrap();
        // Purge expired in the same lock to avoid double-lookup
        if let Some(entry) = inner.map.get_mut(&key) {
            if entry.expires_at > Instant::now() {
                entry.hits += 1;
                return Some(entry.value.clone());
            }
            // Expired — remove
            inner.map.remove(&key);
            inner.order.retain(|k| k != &key);
        }
        None
    }

    pub fn put(
        &self,
        tool: &str,
        args: &Value,
        profile_id: &str,
        workspace: Option<&str>,
        value: Value,
        ttl_secs: u64,
    ) {
        if self.capacity == 0 || ttl_secs == 0 {
            return;
        }
        let key = CacheKey::new(tool, args, profile_id, workspace);
        let mut inner = self.inner.lock().unwrap();

        // LRU eviction if at capacity
        while inner.map.len() >= self.capacity {
            if let Some(oldest) = inner.order.pop_front() {
                inner.map.remove(&oldest);
            } else {
                break;
            }
        }

        inner.order.push_back(key.clone());
        inner.map.insert(
            key,
            Entry {
                value,
                expires_at: Instant::now() + Duration::from_secs(ttl_secs),
                hits: 0,
            },
        );
    }

    /// Invalidate all entries for a given tool name (e.g. after a Write to the same path).
    pub fn invalidate_tool(&self, tool: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.map.retain(|k, _| k.tool != tool);
        inner.order.retain(|k| k.tool != tool);
    }

    /// Invalidate all entries for a workspace (e.g. on workspace close).
    pub fn invalidate_workspace(&self, workspace: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.map.retain(|k, _| k.workspace_path != workspace);
        inner.order.retain(|k| k.workspace_path != workspace);
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.lock().unwrap();
        let now = Instant::now();
        let live = inner.map.values().filter(|e| e.expires_at > now).count();
        let total_hits: u32 = inner.map.values().map(|e| e.hits).sum();
        CacheStats {
            entries: inner.map.len(),
            live,
            total_hits,
        }
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.map.clear();
        inner.order.clear();
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub entries: usize,
    pub live: usize,
    pub total_hits: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cache_isolation_by_profile_id() {
        let cache = ToolCache::new(8);
        let args = json!({"path": "README.md"});

        cache.put(
            "read_file",
            &args,
            "profile-a",
            Some("/workspace"),
            json!({"ok": "a"}),
            60,
        );

        assert_eq!(
            cache.get("read_file", &args, "profile-a", Some("/workspace")),
            Some(json!({"ok": "a"}))
        );
        assert_eq!(
            cache.get("read_file", &args, "profile-b", Some("/workspace")),
            None
        );
    }

    #[test]
    fn cache_isolation_by_workspace_path() {
        let cache = ToolCache::new(8);
        let args = json!({"q": "status"});

        cache.put(
            "get_system_stats",
            &args,
            "profile-a",
            Some("/workspace-a"),
            json!({"cpu": 1}),
            60,
        );

        assert_eq!(
            cache.get("get_system_stats", &args, "profile-a", Some("/workspace-a")),
            Some(json!({"cpu": 1}))
        );
        assert_eq!(
            cache.get("get_system_stats", &args, "profile-a", Some("/workspace-b")),
            None
        );
    }

    #[test]
    fn cache_treats_equivalent_json_object_args_as_same_key() {
        let cache = ToolCache::new(8);
        let args_a = json!({"a": 1, "b": 2});
        let args_b = json!({"b": 2, "a": 1});

        cache.put("demo", &args_a, "p", Some("/w"), json!({"hit": true}), 60);
        assert_eq!(
            cache.get("demo", &args_b, "p", Some("/w")),
            Some(json!({"hit": true}))
        );
    }

    #[test]
    fn cache_rejects_zero_ttl() {
        let cache = ToolCache::new(8);
        let args = json!({"x": 1});

        cache.put("demo", &args, "p", Some("/w"), json!({"v": 1}), 0);
        assert_eq!(cache.get("demo", &args, "p", Some("/w")), None);
    }
}
