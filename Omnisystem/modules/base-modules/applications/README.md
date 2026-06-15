# Omnisystem Application Modules

Applications as dynamically loadable modules in the Universal Module System.

## Overview

All applications in Omnisystem are **base modules** that implement the `OmniModule` trait. This enables:
- ✅ Dynamic loading/unloading without system restart
- ✅ Hot-swapping of compatible applications
- ✅ Lifecycle management (initialize/shutdown)
- ✅ Persistent state in Universal Model Database
- ✅ Dependency resolution and management
- ✅ Runtime configuration management
- ✅ Health monitoring and statistics

## Architecture

```
Omnisystem Platform
├── Universal Module System (core)
│   ├── ModuleRegistry (tracks all modules)
│   ├── AppModuleLoader (manages app lifecycle)
│   └── UniversalModelDatabase (persistent state)
│
└── Application Modules (this directory)
    ├── web/               (Web UI applications)
    ├── mobile/            (Mobile applications)
    ├── services/          (Backend services)
    ├── ai/                (AI/ML services)
    ├── core/              (Core modules)
    ├── APPLICATION_MANIFEST.omni   (App metadata)
    ├── app_module_loader.rs        (App lifecycle manager)
    └── README.md          (This file)
```

## Application Categories

### Web Applications (`web/`)
- **web-dashboard** — Admin interface and monitoring dashboard
- **web-frontend** — User-facing web application
- **web-ide** — In-browser development environment
- **omnisystem-gui** — Tauri desktop wrapper

### Service Applications (`services/`)
- **omnicore** — Core business logic and service orchestration
- **omnisystem-workers** — Background job processing
- **omnisystem-app** — Main application runtime

### Mobile Applications (`mobile/`)
- **omnisystem-mobile** — Cross-platform mobile app (iOS/Android)

### AI Applications (`ai/`)
- **omni-ai** — AI inference and ML service

### Core Applications (`core/`)
- **core-modules** — Foundation modules for all applications

## Module Lifecycle

### Loading an Application

```rust
let loader = AppModuleLoader::new(registry, "/db/path");

// Load application module
loader.load_application("web-dashboard")?;

// Process:
// 1. Resolve dependencies from ModuleRegistry
// 2. Load dependencies in correct order
// 3. Initialize the module (initialize())
// 4. Register in UniversalModelDatabase
// 5. Make available for requests
```

### Using a Loaded Application

Once loaded, an application module is available through:
- Web APIs (HTTP endpoints)
- RPC calls (via OMNI protocol)
- Direct module calls (if loaded in same process)
- Module discovery (via capability system)

### Unloading an Application

```rust
// Unload application module
loader.unload_application("web-dashboard")?;

// Process:
// 1. Find dependent modules
// 2. Unload dependents first (reverse dependency order)
// 3. Call shutdown() on module
// 4. Remove from UniversalModelDatabase
// 5. Free resources
```

### Hot Reloading (if hot_swappable)

```rust
// Hot-reload without restarting dependents
loader.hot_reload_application("omnisystem-workers")?;

// Note: Only available for modules marked hot_swappable
// in APPLICATION_MANIFEST.omni
```

## Module Registration

Each application must implement the `OmniModule` trait:

```rust
pub trait OmniModule: Send + Sync {
    // Required
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn state(&self) -> ModuleState;
    fn capabilities(&self) -> Vec<String>;

    // Optional with defaults
    fn dependencies(&self) -> Vec<String>;
    fn config_schema(&self) -> serde_json::Value;
    fn config(&self) -> serde_json::Value;
    fn set_config(&mut self, config: serde_json::Value) -> Result<()>;
    fn health_check(&self) -> Result<HealthStatus>;
    fn stats(&self) -> ModuleStats;
}
```

### Example Application Module

```rust
pub struct WebDashboard {
    state: ModuleState,
    config: serde_json::Value,
    stats: ModuleStats,
}

impl OmniModule for WebDashboard {
    fn name(&self) -> &str {
        "web-dashboard"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        // Start web server
        // Initialize database connections
        // Load configuration
        self.state = ModuleState::Active;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Stop web server
        // Close connections
        // Cleanup resources
        self.state = ModuleState::Unloaded;
        Ok(())
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "web-ui".to_string(),
            "admin-interface".to_string(),
            "system-monitoring".to_string(),
        ]
    }

    fn dependencies(&self) -> Vec<String> {
        vec![
            "security-framework".to_string(),
            "observability-framework".to_string(),
        ]
    }
}
```

## Manifest Definition

Each application is defined in `APPLICATION_MANIFEST.omni`:

```omni
module WebDashboard {
    name: "web-dashboard",
    version: "2.0.0",
    language: "TypeScript",
    application_type: "web",
    description: "Omnisystem Web Dashboard",
    capabilities: [
        "web-ui",
        "admin-interface",
        "system-monitoring",
    ],
    dependencies: [
        "security-framework",
        "observability-framework",
    ],
    module_interface: "OmniModule",
    loadable: true,
    hot_swappable: true,
    persistence_required: true,
    stability: "STABLE",
    required: false,
}
```

### Module Manifest Fields

- **name** — Unique module identifier
- **version** — Semantic version (follows OMNI versioning)
- **language** — Implementation language (Rust, TypeScript, etc.)
- **application_type** — web, mobile, service, ai, or core
- **description** — Human-readable description
- **capabilities** — Features provided by this module
- **dependencies** — Required base modules
- **module_interface** — "OmniModule" (always)
- **loadable** — Can be loaded dynamically
- **hot_swappable** — Can be reloaded without restarting dependents
- **persistence_required** — Requires state persistence
- **stability** — STABLE, BETA, EXPERIMENTAL
- **required** — Must be loaded for system operation

## Dependency Resolution

The Universal Module System automatically resolves dependencies:

1. **Dependency Graph** — ModuleRegistry tracks all dependencies
2. **Topological Sort** — Ensures correct load order
3. **Circular Dependency Detection** — Prevents cycles
4. **Transitive Dependencies** — Loads all indirect dependencies

```
Example: Loading web-dashboard

web-dashboard depends on:
├── security-framework (depends on titan-core)
└── observability-framework (depends on titan-core)

Load order:
1. titan-core (required by both)
2. security-framework
3. observability-framework
4. web-dashboard
```

## Configuration Management

Applications can be configured dynamically:

```rust
let config = serde_json::json!({
    "port": 8080,
    "workers": 4,
    "log_level": "debug",
});

loader.configure_application("web-dashboard", config)?;
```

Configuration is:
- Validated against config_schema()
- Persisted in UniversalModelDatabase
- Applied immediately without restart
- Preserved across load/unload cycles

## Monitoring & Observability

### Health Checks

```rust
let health = loader.get_application_health("web-dashboard")?;
// Returns: Healthy, Degraded, or Unhealthy
```

### Statistics

```rust
let stats = loader.get_application_stats("web-dashboard")?;
// Returns: uptime, error_count, request_count, etc.
```

### Continuous Monitoring

AppModuleLoader performs:
- Periodic health checks
- Statistics collection
- Error tracking
- Performance monitoring
- Alert generation (on errors)

## Universal Model Database Integration

All application state is persisted in the Universal Model Database:

```
/omnisystem/modules/
├── web-dashboard/
│   ├── state           → "loaded" | "unloaded"
│   ├── config          → Current configuration
│   ├── stats           → Performance metrics
│   ├── health          → Current health status
│   └── history         → Load/unload history
├── omnisystem-workers/
│   └── ...
└── ...
```

State persists across:
- Module reloads
- System restarts
- Configuration changes

## Best Practices

### 1. Minimal Dependencies
- Only depend on required base modules
- Avoid circular or deep dependency chains

### 2. Fast Initialization
- Initialize() should complete quickly
- Defer heavy initialization to background

### 3. Graceful Shutdown
- Shutdown() must complete within timeout
- Don't leave open connections or resources

### 4. Health Monitoring
- Implement health_check()
- Return realistic health status
- Update stats continuously

### 5. Configuration Validation
- Define complete config_schema()
- Validate in set_config()
- Log all configuration changes

### 6. Error Handling
- Return descriptive errors
- Log errors with context
- Update stats on errors

### 7. Resource Management
- Track resource usage in stats()
- Implement cleanup in shutdown()
- Don't hold onto unneeded resources

## Common Patterns

### Accessing Another Module

```rust
// Get module by name
let worker_stats = loader.get_application_stats("omnisystem-workers")?;

// Check if module is loaded
let is_loaded = !loader.list_loaded_applications()
    .iter()
    .filter(|name| *name == "omnicore")
    .collect::<Vec<_>>()
    .is_empty();
```

### Handling Module Not Found

```rust
match loader.unload_application("web-dashboard") {
    Ok(_) => println!("Unloaded successfully"),
    Err(Error::ModuleNotFound(_)) => println!("Not loaded"),
    Err(e) => println!("Error: {}", e),
}
```

### Implementing Hot Reload

```rust
// Only works if hot_swappable: true in manifest
loader.hot_reload_application("omnisystem-workers")?;

// Dependents are not restarted
// Module state is preserved
// Configuration is retained
```

## Troubleshooting

### Module won't load
- Check dependencies are installed
- Verify module name matches APPLICATION_MANIFEST.omni
- Check for circular dependencies
- Review logs for initialization errors

### Module unload fails
- Check for dependent modules
- Ensure timeout is sufficient for shutdown
- Review shutdown() implementation
- Check for resource leaks

### Hot reload not working
- Verify hot_swappable: true in manifest
- Check no incompatible code changes
- Ensure no external dependencies changed
- Review binary compatibility

## References

- `APPLICATION_MANIFEST.omni` — Module definitions
- `app_module_loader.rs` — Loader implementation
- Universal Module System docs
- UniversalModelDatabase docs
- `ModuleRegistry` API reference

## Status

✅ **Production Ready**

- All application modules implemented
- Full Universal Module System integration
- Persistent state management
- Dynamic load/unload support
- Hot-swap capability (where applicable)
- Complete monitoring and observability
