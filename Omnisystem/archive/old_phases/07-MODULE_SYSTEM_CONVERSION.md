# Module System Conversion Guide

**Complete Guide to Converting Legacy Code to Omnisystem Modules**

## Overview

The Omnisystem Module System provides a unified way to organize, depend, and compose code across all languages (TITAN, SYLVA, AETHER, AXIOM). This guide explains how to convert existing code (Rust crates, Conductor modules, Clojure code, etc.) into proper Omnisystem modules.

---

## Module System Architecture

### Module Structure

```
omnisystem_module_system.omni
├── Core Modules (11)
│   ├── TITAN Core
│   ├── SYLVA Core
│   ├── AETHER Core
│   └── AXIOM Core
├── Extension Modules (22)
│   ├── Phase 19: 6 modules
│   ├── Phase 20: 4 modules
│   ├── Phase 21: 4 modules
│   ├── Phase 22: 4 modules
│   └── Phase 23: 4 modules
└── Framework Modules (4)
    ├── Security Framework
    ├── Performance Framework
    ├── Testing Framework
    └── Observability Framework
```

### Module Declaration

```omni
module MyModule {
    name: "my-module",
    version: "2.0.0",
    base_language: "TITAN",
    language: "Titan",
    
    dependencies: [
        "titan-core",
        "other-module",
    ],
    
    exports: [
        "MyStruct",
        "MyTrait",
        "my_function",
    ],
    
    capabilities: [
        "capability1",
        "capability2",
    ],
    
    status: ModuleStatus::Active,
}
```

---

## Converting Rust Crates to Modules

### Step 1: Analyze Existing Crate

**Original Structure:**
```
aether-dns-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── cache.rs
│   ├── dnssec.rs
│   ├── error.rs
│   └── protocol.rs
└── tests/
    └── integration_tests.rs
```

### Step 2: Create Module Declaration

**File: modules/aether_dns_core.omni**
```omni
module AetherDNSCoreModule {
    name: "aether-dns-core",
    version: "2.0.24",
    base_language: "AETHER",
    language: "Aether",
    
    dependencies: [
        "aether-language",
        "axiom-cryptography",
    ],
    
    exports: [
        "DNSResolver",
        "DNSCache",
        "DNSSECValidator",
        "DNSProtocol",
        "DNSError",
    ],
    
    capabilities: [
        "dns-resolution",
        "dnssec-validation",
        "caching",
        "protocol-handling",
        "error-management",
    ],
    
    status: ModuleStatus::Active,
}
```

### Step 3: Convert Rust Code to Aether

**Original (Rust/lib.rs):**
```rust
pub struct DNSResolver {
    cache: Arc<Mutex<HashMap<String, DnsRecord>>>,
    timeout: Duration,
}

impl DNSResolver {
    pub fn new(timeout: Duration) -> Self {
        DNSResolver {
            cache: Arc::new(Mutex::new(HashMap::new())),
            timeout,
        }
    }

    pub fn resolve(&self, domain: &str) -> Result<IpAddr, DnsError> {
        // Implementation
        Ok(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
    }
}
```

**Converted (Aether):**
```aether
pub struct DNSResolver {
    cache: map<string, DNSRecord>,
    timeout_ms: i64,
    resolver_id: string,
}

pub struct DNSRecord {
    domain: string,
    ip_address: string,
    ttl: i64,
    timestamp: i64,
}

impl DNSResolver {
    pub fn new(timeout_ms: i64) -> Self {
        DNSResolver {
            cache: map::new(),
            timeout_ms,
            resolver_id: "resolver_1".to_string(),
        }
    }

    pub fn resolve(&mut self, domain: string) -> Result<string, string> {
        // Check cache first
        if let Some(record) = self.cache.get(&domain) {
            if is_record_valid(record) {
                return Ok(record.ip_address.clone());
            }
        }

        // Perform DNS resolution
        let ip = perform_dns_lookup(domain.clone())?;
        
        // Cache the result
        self.cache.insert(domain, DNSRecord {
            domain: domain.clone(),
            ip_address: ip.clone(),
            ttl: 3600,
            timestamp: current_time(),
        });

        Ok(ip)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        println!("✓ DNS cache cleared");
    }
}
```

### Step 4: Update Module Registry

**File: omnisystem_module_system.omni**
```omni
module AetherDNSCoreModule {
    name: "aether-dns-core",
    version: "2.0.24",
    base_module: "AETHER",
    language: "Aether",

    dependencies: ["aether-language", "axiom-cryptography"],

    exports: [
        "DNSResolver",
        "DNSRecord",
        "DNSCache",
        "DNSSECValidator",
    ],

    capabilities: [
        "dns-resolution",
        "dnssec-validation",
        "dns-caching",
        "dns-protocol",
        "error-handling",
    ],

    status: ModuleStatus::Active,
}
```

---

## Converting Conductor Crates

### Structure Analysis

**Conductor Crate:**
```
access-control-rbac/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── role.rs
│   ├── permission.rs
│   └── enforcement.rs
└── tests/
```

### Conversion to TITAN Module

**Module Declaration:**
```omni
module AccessControlRBACModule {
    name: "security-access-control-rbac",
    version: "2.0.24",
    base_module: "TITAN",
    language: "Titan",

    dependencies: ["titan-language", "security-framework"],

    exports: [
        "RBACManager",
        "Role",
        "Permission",
        "RoleAssignment",
    ],

    capabilities: [
        "role-management",
        "permission-assignment",
        "rbac-enforcement",
        "policy-evaluation",
        "audit-logging",
    ],

    status: ModuleStatus::Active,
}
```

**Code Conversion (Rust → Titan):**
```titan
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct RBACManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
    permissions: Arc<RwLock<HashMap<String, Permission>>>,
    assignments: Arc<RwLock<HashMap<String, RoleAssignment>>>,
}

#[derive(Clone)]
pub struct Role {
    role_id: String,
    name: String,
    permissions: Vec<String>,
    created_at: u64,
}

pub struct Permission {
    perm_id: String,
    name: String,
    description: String,
}

pub struct RoleAssignment {
    user_id: String,
    role_id: String,
    assigned_at: u64,
}

impl RBACManager {
    pub fn new() -> Self {
        RBACManager {
            roles: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_role(
        &self,
        role_id: String,
        name: String,
    ) -> Result<(), String> {
        let mut roles = self.roles.write()?;

        if roles.contains_key(&role_id) {
            return Err(format!("Role {} already exists", role_id));
        }

        roles.insert(
            role_id.clone(),
            Role {
                role_id,
                name,
                permissions: Vec::new(),
                created_at: current_timestamp(),
            },
        );

        println!("✓ Role created successfully");
        Ok(())
    }

    pub fn assign_role(
        &self,
        user_id: String,
        role_id: String,
    ) -> Result<(), String> {
        // Verify role exists
        {
            let roles = self.roles.read()?;
            if !roles.contains_key(&role_id) {
                return Err(format!("Role {} not found", role_id));
            }
        }

        // Create assignment
        let mut assignments = self.assignments.write()?;
        assignments.insert(
            format!("{}_{}", user_id, role_id),
            RoleAssignment {
                user_id: user_id.clone(),
                role_id: role_id.clone(),
                assigned_at: current_timestamp(),
            },
        );

        println!("✓ Role assigned to user");
        Ok(())
    }

    pub fn has_permission(&self, user_id: &str, perm: &str) -> bool {
        let assignments = self.assignments.read().unwrap();
        let roles = self.roles.read().unwrap();

        for (_, assignment) in assignments.iter() {
            if &assignment.user_id == user_id {
                if let Some(role) = roles.get(&assignment.role_id) {
                    if role.permissions.contains(&perm.to_string()) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

---

## Converting Clojure Code

### Clojure to SYLVA/TITAN

**Original Clojure:**
```clojure
(defn fibonacci [n]
  (if (<= n 1)
    n
    (+ (fibonacci (- n 1))
       (fibonacci (- n 2)))))

(defn map-values [f m]
  (reduce-kv (fn [result k v]
               (assoc result k (f v)))
             {}
             m))
```

**Converted to SYLVA:**
```sylva
pub fn fibonacci(n: i32) -> i64 {
    if n <= 1 {
        n as i64
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

pub fn map_values(map: map<string, i32>, transform: fn(i32) -> i32) -> map<string, i32> {
    let mut result = map::new();
    
    for (key, value) in &map {
        result.insert(key.clone(), transform(*value));
    }
    
    result
}
```

### Module Declaration

```omni
module FunctionalUtilsModule {
    name: "sylva-functional-utils",
    version: "2.0.24",
    base_module: "SYLVA",
    language: "Sylva",

    dependencies: ["sylva-language"],

    exports: [
        "fibonacci",
        "map_values",
        "FunctionComposer",
        "Pipeline",
    ],

    capabilities: [
        "functional-programming",
        "recursion",
        "map-reduce",
        "composition",
    ],

    status: ModuleStatus::Active,
}
```

---

## Migration Strategy

### Phase 1: Analysis (Week 1)
- [ ] List all existing crates/modules
- [ ] Map dependencies
- [ ] Identify language mappings
- [ ] Document public APIs

### Phase 2: Conversion (Week 2-3)
- [ ] Create module declarations
- [ ] Convert language-specific code
- [ ] Implement dependencies
- [ ] Write tests

### Phase 3: Integration (Week 4)
- [ ] Register in module system
- [ ] Run integration tests
- [ ] Performance validation
- [ ] Documentation

### Phase 4: Verification (Week 5)
- [ ] End-to-end testing
- [ ] Performance benchmarks
- [ ] Security audit
- [ ] Release

---

## Best Practices for Conversion

### 1. Preserve Functionality
- Test original behavior before conversion
- Maintain same API surface
- Keep error handling patterns

### 2. Language-Appropriate Code
- Use language idioms
- Leverage language features
- Follow conventions

### 3. Dependency Management
- Declare all dependencies explicitly
- Use module exports properly
- Avoid circular dependencies

### 4. Testing
```omni
module MyModule {
    // ... module definition
    
    #[test]
    fn test_basic_functionality() {
        let result = my_function();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_error_handling() {
        let result = my_function_with_error();
        assert!(result.is_err());
    }
}
```

### 5. Documentation
- Write doc comments
- Include examples
- Document dependencies
- List capabilities

---

## Automated Conversion Tools

### Script: Identify Crate Structure

```bash
#!/bin/bash
# identify_crates.sh

find . -name "Cargo.toml" -type f | while read cargo_file; do
    dir=$(dirname "$cargo_file")
    name=$(grep '^name' "$cargo_file" | cut -d'"' -f2)
    
    echo "Found crate: $name"
    echo "  Location: $dir"
    echo "  Files:"
    find "$dir/src" -type f -name "*.rs" | sed 's/^/    /'
done
```

### Script: Generate Module Declaration

```python
#!/usr/bin/env python3
# generate_module_decl.py

import sys
import toml
from pathlib import Path

def generate_module_declaration(cargo_toml_path):
    with open(cargo_toml_path) as f:
        config = toml.load(f)
    
    name = config['package']['name']
    version = config['package']['version']
    dependencies = list(config.get('dependencies', {}).keys())
    
    decl = f"""module {name.title().replace('-', '')}Module {{
    name: "{name}",
    version: "2.0.{version.split('.')[2]}",
    language: "Titan",
    
    dependencies: {dependencies},
    
    exports: [
        // Add exports here
    ],
    
    capabilities: [
        // Add capabilities here
    ],
    
    status: ModuleStatus::Active,
}}"""
    
    return decl

if __name__ == '__main__':
    cargo_path = sys.argv[1]
    print(generate_module_declaration(cargo_path))
```

---

## Verification Checklist

- [ ] All public APIs exported
- [ ] Dependencies declared
- [ ] Tests passing
- [ ] Documentation complete
- [ ] No circular dependencies
- [ ] Capabilities documented
- [ ] Error handling consistent
- [ ] Performance acceptable

---

## Troubleshooting

### Issue: Circular Dependencies
**Solution**: Refactor shared code into separate module

### Issue: Language Feature Mismatch
**Solution**: Use language-appropriate patterns

### Issue: Performance Regression
**Solution**: Profile and optimize hot paths

### Issue: Missing Dependencies
**Solution**: Add to module dependencies and exports

---

## Resources

- [Module System Architecture](./07-CORE_MODULES/README.md)
- [TITAN Language Guide](./03-LANGUAGES-TITAN_COMPLETE.md)
- [Aether Language Guide](./03-LANGUAGES/AETHER.md)
- [API Reference](./08-API_REFERENCE/README.md)

---

**Successfully Converting All Systems to Omnisystem Modules**

*Transform legacy code into modern, maintainable modules.*
