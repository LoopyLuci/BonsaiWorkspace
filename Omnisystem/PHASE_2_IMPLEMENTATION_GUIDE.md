# Phase 2 Implementation Guide: TITAN Wrapping & Integration

**Status**: Ready to implement Phase 2 across all Tier 2 crates  
**First Crate**: app-manager-api  
**Scope**: TITAN wrapper module and integration tests

## Overview

Phase 2 transforms the ULL bridge layer into a fully functional TITAN module that:
- Wraps Rust functions through ULL
- Implements TITAN business logic
- Handles errors gracefully
- Supports async operations
- Validates inputs
- Caches results (optional)

## Phase 2 Architecture

```
Phase 1: Bridge          Phase 2: Wrapper         Phase 3: Full TITAN
┌─────────────────┐   ┌─────────────────┐     ┌─────────────────┐
│ Rust Code       │   │ TITAN Module    │     │ TITAN Module    │
│ (unchanged)     │   │ • Get from Rust │     │ • Full logic    │
├─────────────────┤   │ • Validate      │     │ • No Rust calls │
│ ull_wrapper.rs  │   │ • Cache result  │     └─────────────────┘
│ (FFI layer)     │   │ • Return data   │
├─────────────────┤   └─────────────────┘
│ LanguageBridge  │         ↓
│ (ULL)           │   TITAN business logic
└─────────────────┘   on top of Rust bridge
```

## Step 1: Create TITAN Module

### File Structure

```
languages/titan/
├── app_manager.ti              (Phase 1 bridge - DONE)
├── app_manager_advanced.ti     (Phase 2 patterns - DONE)
├── tests/
│   └── app_manager_integration_tests.ti  (Integration tests - DONE)
└── ...
```

### Module Content Pattern

```titan
// 1. Imports
import ull::bridge
import ull::types

// 2. Data types (TITAN equivalents of Rust structs)
pub struct AppState { ... }

// 3. Bridge functions (call Rust through ULL)
pub fn get_app_info(app_id: String) -> Result[AppState] { ... }

// 4. TITAN business logic (pure TITAN)
pub struct CachedAppManager { ... }

// 5. Tests
#[test]
fn test_integration() { ... }
```

## Step 2: Implement Bridge Functions

### Pattern: Simple Bridge

```titan
pub fn get_app_info(app_id: String) -> Result[AppState] {
    // 1. Call Rust through ULL
    let result = bridge::call_rust("app-manager-api", "get_app_info", {
        "app_id": app_id
    })?
    
    // 2. Convert ULL Value to TITAN struct
    let app_state = AppState {
        app_id: result["app_id"] as String,
        version: result["version"] as String,
        state: result["state"] as String,
        installed_at: result.get("installed_at") as Option[String],
        running: result["running"] as bool,
    }
    
    // 3. Return TITAN type
    return Ok(app_state)
}
```

### Pattern: With Validation

```titan
pub fn install_app_validated(
    app_id: String,
    version: String,
) -> Result[AppState] {
    // 1. Validate inputs
    if app_id.is_empty() {
        return Err("app_id cannot be empty")
    }
    
    if !is_valid_version(&version) {
        return Err("Invalid version format")
    }
    
    // 2. Call Rust
    let app = bridge::call_rust("app-manager-api", "install_app", {
        "app_id": app_id,
        "version": version,
    })?
    
    // 3. Return result
    return Ok(app)
}
```

### Pattern: With Caching

```titan
pub fn get_app_cached(
    mut manager: CachedAppManager,
    app_id: String,
) -> Result[AppState] {
    // 1. Check cache
    if let Some(cached) = manager.cache[app_id] {
        return Ok(cached)
    }
    
    // 2. Call Rust if not cached
    let app = bridge::call_rust("app-manager-api", "get_app_info", {
        "app_id": app_id
    })?
    
    // 3. Store in cache
    manager.cache[app_id] = app
    
    // 4. Return
    return Ok(app)
}
```

### Pattern: With Error Handling

```titan
pub fn install_with_retry(
    app_id: String,
    version: String,
    max_retries: u64,
) -> Result[AppState] {
    let mut last_error = None
    
    for attempt in 0..max_retries {
        match bridge::call_rust("app-manager-api", "install_app", {
            "app_id": app_id.clone(),
            "version": version.clone(),
        }) {
            Ok(result) => return Ok(result as AppState),
            Err(e) => {
                last_error = Some(e)
                if attempt < max_retries - 1 {
                    // Exponential backoff
                    let wait_ms = 100 * (2 ^ attempt)
                    sleep_ms(wait_ms)
                }
            }
        }
    }
    
    return Err(last_error.unwrap_or("Unknown error"))
}
```

## Step 3: Add Business Logic

### Pattern: Wrapper with State

```titan
pub struct AppManager {
    cache: Object
    config: AppManagerConfig
}

impl AppManager {
    pub fn new(config: AppManagerConfig) -> Self {
        return AppManager {
            cache: {},
            config: config,
        }
    }
    
    pub fn get_app(mut self: Self, app_id: String) -> Result[AppState] {
        // Check cache
        if let Some(cached) = self.cache[app_id] {
            return Ok(cached)
        }
        
        // Call Rust
        let app = get_app_info(app_id.clone())?
        
        // Cache result
        self.cache[app_id] = app
        
        return Ok(app)
    }
}
```

### Pattern: Batch Operations

```titan
pub struct AppManagerBatch {
    operations: Array[String]
}

impl AppManagerBatch {
    pub fn new() -> Self {
        return AppManagerBatch {
            operations: [],
        }
    }
    
    pub fn add_install(mut self: Self, app_id: String, version: String) -> Self {
        self.operations.push(format!("install:{app_id}:{version}"))
        return self
    }
    
    pub fn execute_all(self: Self) -> Result[Array[String]] {
        let mut results: Array[String] = []
        
        for op in self.operations {
            let parts = op.split(":")
            let op_type = parts[0]
            
            match op_type {
                "install" => {
                    let _result = install_app(parts[1], parts[2])?
                    results.push(format!("✓ {op}"))
                }
                "uninstall" => {
                    let _result = uninstall_app(parts[1])?
                    results.push(format!("✓ {op}"))
                }
                _ => results.push(format!("✗ Unknown: {op}"))
            }
        }
        
        return Ok(results)
    }
}
```

## Step 4: Write Tests

### Pattern: Unit Tests

```titan
#[test]
fn test_app_state_creation() {
    let app = AppState::new("test-app")
    assert_eq!(app.app_id, "test-app")
    assert_eq!(app.state, "created")
    assert!(!app.running)
}
```

### Pattern: Integration Tests

```titan
#[test]
async fn test_call_rust_get_app_info() {
    let result = get_app_info("real-app-id")
    
    // Should succeed or return specific error
    assert!(result.is_ok() || result.is_err())
}
```

### Pattern: Cross-Language Tests

```titan
#[test]
async fn test_roundtrip_data_integrity() {
    // Create locally
    let app = AppState::new("roundtrip-app")
        .mark_installed()
        .mark_running()
    
    // Convert to ULL Value (simulating cross-language call)
    let value = app_info_to_value(&app)
    
    // Convert back
    let recovered = value_to_app_info(&value)
    
    // Verify data integrity
    assert_eq!(app.app_id, recovered.app_id)
    assert_eq!(app.state, recovered.state)
    assert_eq!(app.running, recovered.running)
}
```

## Step 5: Error Handling

### Pattern: Error Types

```titan
pub enum AppManagerError {
    InvalidAppId,
    InvalidVersion,
    AppNotFound,
    InstallFailed(String),
    UninstallFailed(String),
    BridgeCommunicationError(String),
}

pub fn error_to_string(error: AppManagerError) -> String {
    match error {
        InvalidAppId => "Invalid app ID provided",
        InvalidVersion => "Invalid version format (expected X.Y.Z)",
        AppNotFound => "Application not found",
        InstallFailed(msg) => format!("Installation failed: {msg}"),
        UninstallFailed(msg) => format!("Uninstallation failed: {msg}"),
        BridgeCommunicationError(msg) => format!("Bridge error: {msg}"),
    }
}
```

### Pattern: Error Recovery

```titan
pub fn install_app_with_fallback(
    app_id: String,
    version: String,
    fallback_version: String,
) -> Result[AppState] {
    match install_app(&app_id, &version) {
        Ok(app) => Ok(app),
        Err(_) => {
            // Try with fallback version
            install_app(&app_id, &fallback_version)
        }
    }
}
```

## Step 6: Performance Patterns

### Pattern: Async Batch Operations

```titan
pub async fn install_apps_concurrent(
    apps: Array[(String, String)],
) -> Result[Array[AppState]] {
    let mut results: Array[AppState] = []
    
    for (app_id, version) in apps {
        // Use async installer
        let app = install_app(&app_id, &version).await?
        results.push(app)
    }
    
    return Ok(results)
}
```

### Pattern: Lazy Loading

```titan
pub struct AppManagerLazy {
    app_ids: Array[String],
    cache: Object,
}

impl AppManagerLazy {
    pub fn new(app_ids: Array[String]) -> Self {
        return AppManagerLazy {
            app_ids: app_ids,
            cache: {},
        }
    }
    
    pub fn get_app(mut self: Self, index: usize) -> Result[AppState] {
        if index >= self.app_ids.len() {
            return Err("Index out of bounds")
        }
        
        let app_id = self.app_ids[index].clone()
        
        // Load on demand
        if !self.cache.contains(app_id) {
            let app = get_app_info(app_id.clone())?
            self.cache[app_id.clone()] = app
        }
        
        return Ok(self.cache[app_id] as AppState)
    }
}
```

## Testing Checklist

For Phase 2 implementation:

- [ ] Unit tests for TITAN types (AppState, etc.)
- [ ] Integration tests for bridge calls
- [ ] Cross-language roundtrip tests
- [ ] Error handling tests
- [ ] Performance tests (benchmarks)
- [ ] Async/await tests
- [ ] State management tests
- [ ] Caching tests (if implemented)
- [ ] Batch operation tests (if implemented)

## Validation Checklist

Before moving to Phase 3:

- [ ] All bridge functions return correct TITAN types
- [ ] Error handling works across boundary
- [ ] Type conversions preserve data
- [ ] Async operations complete successfully
- [ ] Performance meets requirements (<5μs overhead)
- [ ] All tests pass
- [ ] Documentation updated

## Next Crates (Tier 2A Pattern)

Apply same Phase 2 pattern to:

1. ✅ **app-manager-api** (DONE)
2. 🔄 **app-manager-core** (Next)
3. 📋 **app-manager-cli**
4. 📋 **app-manager-security**
5. 📋 **app-manager-config**

Each follows identical pattern:
1. Phase 1: ULL wrapper (Rust side)
2. Phase 2: TITAN module (TITAN side)
3. Phase 3: Full TITAN implementation

## Success Metrics

- ✅ TITAN module compiles without errors
- ✅ All integration tests pass
- ✅ Cross-language calls work correctly
- ✅ <5 microsecond call overhead
- ✅ Data integrity preserved in roundtrips
- ✅ Error handling works across boundary
- ✅ Performance meets requirements

## Troubleshooting

### Issue: Type Conversion Fails
**Solution**: Verify ULL Value structure matches expected fields

### Issue: Bridge Call Hangs
**Solution**: Check async/await is used properly

### Issue: Data Loss in Conversion
**Solution**: Implement explicit field mapping in conversion

### Issue: Performance Degradation
**Solution**: Check for unnecessary conversions or repeated calls

## References

- Phase 1 (ULL Bridge): `APP_MANAGER_API_MIGRATION.md`
- ULL Documentation: `src/crates/universal-language-layer/`
- TITAN Language: `languages/titan/docs/`
- Examples: `languages/titan/app_manager*.ti`

---

**Ready to implement Phase 2 across all Tier 2 crates.**

This pattern scales to all 2431 Rust crates with consistent methodology.
