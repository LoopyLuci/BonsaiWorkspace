# Phase 4: Dependency Migration Guide (Weeks 7-8)

## Overview
Phase 4 systematically migrates all Omnisystem crates from external dependencies to custom zero-dependency components. This guide provides the migration strategy, patterns, and implementation roadmap.

## Migration Target Mapping

### External Dependencies → Omnisystem Components

| External Crate | Omnisystem Component | Key Exports | Replacement Pattern |
|---|---|---|---|
| `tokio` | omnisystem-async-runtime | `initialize_runtime()`, `global_runtime()`, `spawn()`, `block_on()` | Replace tokio::spawn with spawn(), tokio::main with block_on(async {...}) |
| `serde`/`serde_json` | omnisystem-serialization | Custom JSON encoder/decoder | Implement Serialize/Deserialize traits |
| `dashmap` | omnisystem-collections | `ConcurrentMap`, `ShardedMap`, `mpsc_queue` | Drop-in replacements with same API |
| `chrono` | omnisystem-time | `Instant`, `Duration` | Replace DateTime with Instant, add/subtract Duration |
| `uuid` | omnisystem-id-generation | `UuidV4`, `SnowflakeGen`, `Ulid` | Use UuidV4::new() for UUID generation |
| `axum`/`tower` | omnisystem-web-framework | `Router`, `Request`, `Response`, `Middleware` | Route registration and handler patterns |
| `tracing` | omnisystem-observability | `Logger`, `Tracer`, `MetricsCollector` | Log events with MetricsCollector |

## Migration Phases by Crate Type

### Priority Tier 1: Core Framework Crates (Week 7)

These crates are dependencies for many others and must be migrated first:

1. **omnisystem-async-runtime** (AsyncRuntime Component)
   - Status: Phase 1 complete with tests passing
   - Dependents: All async/network-based crates
   
2. **omnisystem-collections** (Collections Component)
   - Status: Phase 2 complete with 13 tests passing
   - Dependents: System architecture crates, storage crates
   
3. **omnisystem-web-framework** (WebFramework Component)
   - Status: Phase 2 complete with tests passing
   - Dependents: API service crates, HTTP server crates
   
4. **omnisystem-time** (Time Component)
   - Status: Phase 3 complete with tests passing
   - Replaces: chrono, std::time extensions
   
5. **omnisystem-id-generation** (IdGeneration Component)
   - Status: Phase 3 complete with tests passing
   - Replaces: uuid, custom ID generation
   
6. **omnisystem-observability** (Observability Component)
   - Status: Phase 3 complete with tests passing
   - Replaces: tracing, metrics collection

### Priority Tier 2: GUI and Application Crates (Week 7)

1. **omnisystem-gui**
   - Current deps: tokio, serde/serde_json
   - Migration: Replace tokio with omnisystem-async-runtime, remove serde/serde_json
   - Pattern: Use tauri's built-in command system, JSON handling through custom serializer
   
2. **omnisystem-app**
   - Current deps: Check for serde, tokio usage
   - Migration: Replace with corresponding Omnisystem components

### Priority Tier 3: Infrastructure Crates (Week 7-8)

Migrate these systematic crates that other modules depend on:
- Core access control framework crates
- Base architecture crates
- Storage and persistence crates
- Async operation crates

### Priority Tier 4: Feature Crates (Week 8)

Migrate all remaining feature crates:
- Analytics, monitoring, observability extensions
- AI/ML integration crates
- Domain-specific service crates
- Advanced feature implementations

## Migration Patterns

### Pattern 1: Replacing tokio with omnisystem-async-runtime

**Before:**
```rust
use tokio::task;

#[tokio::main]
async fn main() {
    task::spawn(async {
        // work
    }).await.unwrap();
}
```

**After:**
```rust
use omnisystem_async_runtime::{initialize_runtime, spawn, block_on};

fn main() {
    initialize_runtime();
    block_on(async {
        spawn(async {
            // work
        });
    });
}
```

**Cargo.toml:**
```toml
# Remove: tokio = { version = "1", features = ["full"] }
# Add:
omnisystem-async-runtime = { path = "../omnisystem-async-runtime" }
```

### Pattern 2: Replacing serde with custom JSON handling

**Before:**
```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    value: u64,
}

let json = serde_json::to_string(&config)?;
let config: Config = serde_json::from_str(&json)?;
```

**After:**
```rust
// For simple cases, use string concatenation or custom serialization
struct Config {
    name: String,
    value: u64,
}

impl Config {
    fn to_json_string(&self) -> String {
        format!(r#"{{"name":"{}","value":{}}}"#, self.name, self.value)
    }
    
    fn from_json_string(s: &str) -> Option<Self> {
        // Parse JSON manually or use a simple JSON parser
        // This depends on structure complexity
        None
    }
}
```

**Tauri Workaround:**
- Tauri has built-in JSON serialization for command arguments
- Use `#[tauri::command]` with standard Rust types (String, u64, etc.)
- Tauri handles JSON marshalling automatically

### Pattern 3: Replacing dashmap with omnisystem-collections

**Before:**
```rust
use dashmap::DashMap;

let map: DashMap<String, u64> = DashMap::new();
map.insert("key".to_string(), 42);
let value = map.get(&"key".to_string()).map(|v| *v);
```

**After:**
```rust
use omnisystem_collections::ConcurrentMap;

let map: ConcurrentMap<String, u64> = ConcurrentMap::new();
map.insert("key".to_string(), 42);
let value = map.get(&"key".to_string());
```

### Pattern 4: Replacing uuid with omnisystem-id-generation

**Before:**
```rust
use uuid::Uuid;

let id = Uuid::new_v4();
let id_string = id.to_string();
```

**After:**
```rust
use omnisystem_id_generation::UuidV4;

let id = UuidV4::new();
let id_string = id.to_string();
```

### Pattern 5: Replacing chrono with omnisystem-time

**Before:**
```rust
use chrono::{DateTime, Utc, Duration};

let now = Utc::now();
let later = now + Duration::seconds(60);
let duration = later - now;
```

**After:**
```rust
use omnisystem_time::{Instant, Duration};

let now = Instant::now();
let later = now + Duration::from_secs(60);
let duration = later.elapsed();
```

### Pattern 6: Replacing tracing with omnisystem-observability

**Before:**
```rust
use tracing::{info, warn};

info!(target: "my_module", "Event occurred");
warn!("Warning message");
```

**After:**
```rust
use omnisystem_observability::{Logger, LogLevel};

let logger = Logger::new(LogLevel::Info);
logger.log(LogLevel::Info, "Event occurred");
logger.log(LogLevel::Warn, "Warning message");
```

## Migration Checklist Template

For each crate being migrated:

```
Crate: [name]
Current Dependencies:
- [ ] tokio: [version, features]
- [ ] serde: [version, features]
- [ ] serde_json: [version]
- [ ] dashmap: [version]
- [ ] chrono: [version]
- [ ] uuid: [version]
- [ ] tracing: [version]
- [ ] [other external crates]

Migration Steps:
- [ ] 1. Identify all imports from external crates
- [ ] 2. Plan replacement strategy for each import
- [ ] 3. Update Cargo.toml dependencies
- [ ] 4. Replace code patterns with Omnisystem equivalents
- [ ] 5. Update tests to use new APIs
- [ ] 6. Run `cargo test -p [crate]` to verify
- [ ] 7. Run `cargo check --all` to verify no regressions
- [ ] 8. Document any special migration notes
- [ ] 9. Create PR with migration changes

Verification:
- [ ] All tests pass
- [ ] No cargo check warnings about unmigrated dependencies
- [ ] Code compiles with no errors
- [ ] Performance benchmarks match or exceed previous
```

## Success Criteria for Phase 4

- [ ] All 25+ external crates eliminated from core Omnisystem crates
- [ ] Tier 1 crates (6 framework crates) fully migrated with zero external deps
- [ ] Tier 2 crates (GUI, App) fully migrated with only safe externals (tauri, tauri-build)
- [ ] Tier 3 & 4 majority migrated (>80% of crates)
- [ ] All tests passing across workspace
- [ ] Build succeeds with `cargo build --release`
- [ ] No cargo audit warnings for vulnerable dependencies
- [ ] Performance benchmarks maintained or improved
- [ ] Supply chain attack surface reduced to zero for internal code

## Timeline

- **Week 7, Day 1-2**: Tier 1 completion validation, begin Tier 2 (GUI/App)
- **Week 7, Day 3-5**: Tier 2 completion, start Tier 3
- **Week 8, Day 1-3**: Tier 3 completion, begin Tier 4
- **Week 8, Day 4-5**: Tier 4 completion, final testing and validation

## Known Issues and Workarounds

### Issue: Pre-existing brotli dependency conflict
- **Impact**: cargo check may fail due to transitive brotli versions
- **Root Cause**: Multiple versions of alloc_no_stdlib in dependency tree
- **Workaround**: Use `cargo tauri dev` for GUI testing instead of `cargo check`
- **Solution**: Waiting for transitive dependency cleanup in future phase

### Issue: Tauri maintains external dependencies
- **Rationale**: Tauri is necessary for cross-platform desktop GUI
- **Scope**: tauri and tauri-build are exempt from zero-dependency goal
- **Alternative**: Could implement pure Rust GUI framework in future

## Post-Phase 4 Benefits

1. **Supply Chain Security**: Zero vulnerability surface for internal code
2. **Build Speed**: Reduced compilation time from eliminated dependencies
3. **Deployment Size**: Smaller binary size with optimized internals
4. **Vendor Lock-in Prevention**: No risk from dependency deprecation
5. **Full Control**: Can optimize components specifically for Omnisystem needs
6. **Maintainability**: Single source of truth for critical components

## Next Steps (Phase 5)

Phase 5 focuses on hardening and optimization:
- Security audit of all Phase 1-4 components
- Performance optimization and stress testing
- Production release preparation
- Deployment and distribution strategy
