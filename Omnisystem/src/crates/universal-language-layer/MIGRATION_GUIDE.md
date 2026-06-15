# Rust to TITAN Migration Guide

## Overview

This guide explains how to migrate Rust crates to TITAN language while maintaining compatibility through the Universal Language Layer (ULL).

## Migration Strategy

### Phase 1: Analysis (Current)
- Identify which Rust crates should be migrated
- Analyze dependencies and integration points
- Create migration plan

### Phase 2: Bridge Layer (In Progress)
- Create ULL bridge between Rust and TITAN
- Wrap existing Rust code for TITAN access
- Implement type conversions and FFI

### Phase 3: Gradual Migration
- Start with isolated, low-dependency crates
- Migrate high-level functionality first
- Keep critical infrastructure in Rust initially

### Phase 4: Full Integration
- All application modules written in Omni-languages
- Rust becomes implementation detail for performance
- Complete cross-language ecosystem

## Crate Prioritization

### Tier 1: Core Infrastructure (Keep in Rust for now)
- `omnisystem-core` — runtime foundation
- `module-registry` — module management
- `universal-language-layer` — FFI infrastructure

**Why**: Performance-critical, must be highly optimized

### Tier 2: Business Logic (Migrate to TITAN)
- `app-manager-api` → TITAN
- `app-manager-core` → TITAN  
- `api-gateway` → TITAN
- `configuration-manager` → TITAN

**Why**: Business logic, easier to maintain in higher-level language

### Tier 3: Services (Migrate to appropriate Omni-language)
- `analytics-engine` → SYLVA (ML focus)
- `distributed-services` → AETHER (distributed)
- `verification-system` → AXIOM (formal methods)

### Tier 4: Utilities (Migrate to TITAN)
- `logging` → TITAN
- `monitoring` → TITAN
- `telemetry` → TITAN

## Migration Process

### Step 1: Wrap Existing Rust Code

Before migrating, wrap Rust functionality for TITAN access:

```rust
// In Rust crate: app-manager-api/src/lib.rs

use universal_language_layer::{LanguageBridge, Language};

pub async fn expose_to_titan(bridge: &LanguageBridge) -> Result<()> {
    // Register Rust function for TITAN to call
    bridge.register_function(
        FunctionSignature {
            name: "get_app_info".to_string(),
            language: Language::Rust,
            parameters: vec![Parameter {
                name: "app_id".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            return_type: "object".to_string(),
            is_async: true,
        },
        get_app_info as *const libc::c_void,
    )?;
    
    Ok(())
}
```

### Step 2: Create TITAN Wrapper Module

Create TITAN module that uses ULL to call Rust:

```titan
// In app_manager.ti

import ull::bridge
import ull::types

pub fn get_app(app_id: String) -> AppInfo {
    let result = bridge::call_rust("get_app_info", {
        "app_id": app_id
    })?
    
    return result as AppInfo
}
```

### Step 3: Implement in TITAN

Implement the actual logic in TITAN:

```titan
// In app_manager_impl.ti

pub fn get_app_config(app_id: String) -> Config {
    let app_info = get_app(app_id)?
    let config = Config {
        name: app_info.name,
        version: app_info.version,
        // ... more fields
    }
    return config
}
```

### Step 4: Remove Rust Implementation

Once TITAN implementation is complete and tested:
- Remove old Rust code
- Keep wrapper layer for backward compatibility (if needed)
- Update imports

## Type Mapping

### Rust → TITAN Type Conversions

```
Rust Type           TITAN Type          ULL Value Type
─────────────────────────────────────────────────────
bool                bool                Boolean
i64                 i64                 Integer
f64                 f64                 Float
String              String              String
Vec<T>              Array[T]            Array
HashMap             Object              Object
None/Some           Option              Special
Result              Result              Special
```

## Example: Migrating app-manager-core

### Original Rust Code

```rust
// app-manager-core/src/app.rs
pub struct AppManager {
    apps: HashMap<String, AppInfo>,
}

impl AppManager {
    pub fn new() -> Self { ... }
    pub fn get_app(&self, id: &str) -> Result<AppInfo> { ... }
    pub fn install_app(&mut self, app: AppInfo) -> Result<()> { ... }
}
```

### Phase 1: Create Rust Wrapper

```rust
// app-manager-core/src/ull_wrapper.rs
use universal_language_layer::*;

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<()> {
    bridge.register_module("app-manager-core", Language::Rust)?;
    
    bridge.register_function(
        FunctionSignature {
            name: "get_app".to_string(),
            language: Language::Rust,
            // ... parameters and return type
        },
        get_app_ffi as *const libc::c_void,
    )?;
    
    Ok(())
}
```

### Phase 2: TITAN Wrapper Module

```titan
// app-manager-titan/src/app_manager.ti

import ull::bridge

pub fn get_app(app_id: String) -> AppInfo {
    let result = bridge::call("app-manager-core", "get_app", {
        "id": app_id
    })?
    
    return result as AppInfo
}
```

### Phase 3: TITAN Implementation

```titan
// app-manager-titan/src/app_manager_impl.ti

pub struct AppManager {
    apps: Object  // HashMap-like object
}

impl AppManager {
    pub fn new() -> Self {
        return AppManager {
            apps: {}
        }
    }
    
    pub fn get_app(self: &Self, id: String) -> Result[AppInfo] {
        if let Some(app) = self.apps[id] {
            return Ok(app as AppInfo)
        }
        return Err("App not found")
    }
}
```

### Phase 4: Cleanup

- Remove Rust implementation
- Update all imports to use TITAN
- Maintain wrapper layer if external integrations exist

## Testing Strategy

### During Migration

1. **Unit Tests** — Test TITAN code independently
2. **Integration Tests** — Test Rust ↔ TITAN calls
3. **Compatibility Tests** — Ensure behavior matches original

### Example Test

```titan
// app-manager-tests/tests/app_manager.ti

#[test]
fn test_get_app() {
    let manager = create_test_manager()
    let app = manager.get_app("test-app")?
    
    assert_eq!(app.name, "test-app")
    assert_eq!(app.version, "1.0.0")
}
```

## Performance Considerations

### Rust is Faster For:
- Memory-intensive operations
- Real-time processing
- System-level code

### TITAN is Better For:
- Business logic clarity
- Type safety
- Interop with other Omni-languages

### Hybrid Approach:
```
Rust (Performance)
    ↓ (ULL Bridge)
TITAN (Business Logic)
    ↓ (ULL Bridge)
SYLVA (ML Operations)
```

## Migration Checklist

For each crate being migrated:

- [ ] Document existing functionality
- [ ] Create ULL wrapper for Rust code
- [ ] Write TITAN equivalent
- [ ] Add unit tests in TITAN
- [ ] Add integration tests
- [ ] Benchmark performance
- [ ] Compare error handling
- [ ] Update documentation
- [ ] Remove Rust code (or keep wrapper)
- [ ] Update CI/CD pipelines
- [ ] Deploy and monitor

## Common Issues

### Issue 1: Memory Unsafety
**Problem**: Rust's borrow checker doesn't apply in TITAN  
**Solution**: Implement explicit lifetime management in TITAN

### Issue 2: Performance Degradation
**Problem**: TITAN slower than optimized Rust  
**Solution**: Keep critical paths in Rust, use FFI bridge

### Issue 3: Type System Differences
**Problem**: Rust and TITAN have different type systems  
**Solution**: Use explicit conversions in ULL type module

## Resources

- Universal Language Layer Docs: `universal-language-layer/`
- TITAN Language Guide: `languages/titan/docs/`
- FFI Best Practices: `universal-language-layer/src/ffi.rs`
- Example Migrations: `examples/`

## Contact & Support

For migration questions or blockers:
1. Check migration guide (this file)
2. Review example migrations
3. Test with ULL bridge
4. Open discussion in project tracking
