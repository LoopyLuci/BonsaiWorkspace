# MASTER RESTRUCTURING PLAN

**Date**: June 14, 2026  
**Status**: Ready for Execution  
**Scope**: Reorganize 95 Omni* modules + 12 core modules + compiler infrastructure

---

## DISCOVERY SUMMARY

### Current Reality
- **47,200 lines** of working Titan code (535 files)
- **545 compiled binaries** (proof of functionality)
- **95 Omnisystem service modules** currently in `/titan/omni*/`
- **29 compiler passes** (including GPU, LLVM, Axiom integration)
- **12 partial modules** in `/modules/` (need completion)
- **Core infrastructure**: Module system, capability system, data manager in `/modules/omnisystem-core/`

### Why Current Structure Fails
1. Code is **scattered across `/titan/` and `/modules/`**
2. **No single source of truth** for module configuration
3. **No hot-reloading** infrastructure
4. **No manifest system** (omnisystem.toml)
5. **Dependency tracking incomplete**
6. **Cannot be consumed as modules** - must be monolithic

### What Success Looks Like
```
/modules/
├── core/                          # 7 core framework modules
│   ├── omnisystem-core/          (Module system, capability system)
│   ├── omnisystem-runtime/       (AsyncRuntime)
│   ├── omnisystem-time/          (Time handling)
│   ├── omnisystem-id/            (ID generation)
│   ├── omnisystem-serialization/ (JSON codec)
│   ├── omnisystem-observability/ (Tracing, metrics)
│   └── omnisystem-collections/   (Data structures)
│
├── languages/                     # 4 language implementations
│   ├── titan-compiler/           (Self-hosted Titan)
│   ├── aether-runtime/           (Actor system)
│   ├── sylva-interpreter/        (Interactive scripting)
│   └── axiom-kernel/             (Proof checker)
│
├── services/                      # 30+ core services
│   ├── compiler-service/
│   ├── marketplace-service/
│   ├── package-manager/
│   ├── monitor-service/
│   ├── caching-service/
│   ├── filesystem-service/
│   ├── network-service/
│   └── [25 others from titan/omni*]
│
└── applications/                  # User-facing apps
    ├── gui/
    ├── cli/
    ├── web-dashboard/
    └── [plugins, notebooks, etc.]
```

---

## STRUCTURED MIGRATION PLAN

### PHASE A: Quick Wins (Days 1-2)

**Task A1**: Move mature language implementations
```
move /aether/titan_mature → /modules/languages/titan-compiler/src/
move /aether/aether_mature → /modules/languages/aether-runtime/src/
move /aether/sylva_mature → /modules/languages/sylva-interpreter/src/
move /aether/axiom_mature → /modules/languages/axiom-kernel/src/
```

**Task A2**: Move all 95 omni* modules to proper structure
```bash
for dir in /titan/omni*; do
  name=$(basename $dir)
  category=$(categorize $name)  # infer from prefix
  mv $dir /modules/$category/$name/
done
```

**Task A3**: Add omnisystem.toml manifest to each module
```toml
[module]
name = "omnisystem-compiler"
version = "1.0.0"
description = "Multi-language universal compiler"

[module.capabilities]
"compiler:titan" = { enabled = true }
"compiler:c" = { enabled = true }
"compiler:rust" = { enabled = true }
# ... etc
```

### PHASE B: Consolidation (Days 3-4)

**Task B1**: Merge partial implementations
- `/modules/omnisystem-cli` + `/titan/omnicli/` → unified
- `/modules/omnisystem-marketplace` + `/titan/omnimarketplace/` → unified
- etc.

**Task B2**: Update cross-module dependencies
```bash
# Find all imports in .ti files
grep -r "use.*omni" /modules/
grep -r "import.*omni" /modules/

# Update paths: /titan/omniX → relative paths
sed -i 's|/titan/omni|../|g' /modules/**/*.ti
```

**Task B3**: Build module dependency graph
```bash
# Generate DAG of all dependencies
python3 scripts/generate_dependency_graph.py > DEPENDENCY_GRAPH.md
```

### PHASE C: Module System Integration (Days 5-7)

**Task C1**: Create omnisystem.toml for each of 95 modules

Template:
```toml
[module]
name = "omnisystem-{service}"
version = "1.0.0"
description = "..."
author = "Omnisystem Team"

[module.dependencies]
# List all modules this depends on
"omnisystem-core" = ">=1.0"
"omnisystem-runtime" = ">=1.0"

[module.capabilities]
# Define capabilities this provides
"{service}:feature1" = { enabled = true }
"{service}:feature2" = { enabled = false }

[module.data]
system_data = "modules/{service}"
user_data = "modules/{service}"
device_data = "modules/{service}"
temp_data = "modules/{service}"

[module.health]
check_interval_ms = 60000
timeout_ms = 5000

[omnios_mode]
# Full configuration for OmniOS

[bonsai_mode]
# Lightweight configuration
```

**Task C2**: Implement module registry scanner
```titan
pub fn scan_modules(base_path: &str) -> Vec<ModuleManifest> {
    let mut modules = Vec::new();
    for dir in walk_directory(base_path) {
        if file_exists(dir + "/omnisystem.toml") {
            let manifest = parse_toml(dir + "/omnisystem.toml");
            modules.push(manifest);
        }
    }
    modules.sort_by_dependencies();
    return modules;
}
```

**Task C3**: Test module loading
```
1. Load all 100+ modules
2. Verify dependency ordering
3. Check for circular dependencies
4. Validate capability declarations
5. Verify data paths exist
```

### PHASE D: Compiler Integration (Days 8-9)

**Task D1**: Create unified build system
```bash
# Old: cargo build (scattered crates)
# New: omnisystem build (module-aware)

# Build system strategy:
# 1. Scan /modules for omnisystem.toml
# 2. Resolve dependencies
# 3. Compile in order
# 4. Link modules
# 5. Verify hot-reload capability
```

**Task D2**: Integrate 29 compiler passes
- Group compiler passes into logical stages
- Create pass ordering DAG
- Add configuration for each pass
- Document pass contracts

**Task D3**: Build module loader
```aether
actor ModuleLoader {
    var loaded_modules: Map<String, Module>;
    
    handle LoadModule(path: String) -> Result<ModuleRef> {
        let manifest = load_manifest(path);
        let deps = resolve_dependencies(manifest.dependencies);
        let loaded = compile_and_load(manifest, deps);
        loaded_modules.insert(manifest.name, loaded);
        return Ok(loaded);
    }
    
    handle UnloadModule(name: String) {
        loaded_modules.remove(name);
    }
    
    handle ReloadModule(name: String, new_path: String) {
        let old = loaded_modules.get(name);
        let new = LoadModule(new_path).await;
        migrate_state(old.state, new.state);
        loaded_modules.insert(name, new);
    }
}
```

### PHASE E: Verification & Testing (Days 10-11)

**Task E1**: Build comprehensive test suite
- Unit tests for each module
- Integration tests (module interactions)
- Performance benchmarks
- Hot-reload tests

**Task E2**: Verify all 545 binaries still work
```bash
for exe in $(find /modules -name "*.exe"); do
    $exe || echo "FAILED: $exe"
done
```

**Task E3**: Validate module boundaries
- Dependency isolation
- No forbidden cross-module access
- Capability enforcement

### PHASE F: Documentation & Launch (Days 12-14)

**Task F1**: Create module development guide
```markdown
# Module Development Guide

Every module has:
- omnisystem.toml (manifest)
- src/*.ti (Titan source)
- tests/ (module tests)
- docs/ (documentation)

To create a module:
1. mkdir modules/category/my-module
2. Create omnisystem.toml with capabilities
3. Create src/lib.ti with module implementation
4. Add tests in tests/
5. Run: omnisystem verify modules/category/my-module
```

**Task F2**: Create architecture documentation
```
Module System Architecture
  ├── Registry (who's loaded)
  ├── Loader (dynamic loading)
  ├── Capabilities (what's enabled)
  ├── Dependencies (ordering)
  ├── Data Manager (storage isolation)
  ├── Health Monitor (liveness)
  └── Hot Reloader (zero-downtime updates)
```

**Task F3**: Publish module catalog
- All 100+ modules documented
- Capability index
- Dependency graph visualization
- Quick-start guides

---

## EXECUTION TIMELINE

| Phase | Days | Tasks | Deliverable |
|-------|------|-------|-------------|
| A | 1-2 | A1-A3 | 95 modules moved, manifests created |
| B | 3-4 | B1-B3 | Consolidated structure, dep graph |
| C | 5-7 | C1-C3 | All modules have omnisystem.toml, registry works |
| D | 8-9 | D1-D3 | Unified build system, module loader |
| E | 10-11 | E1-E3 | All tests passing, binaries verified |
| F | 12-14 | F1-F3 | Documentation, launch |

**Total**: 14 days (2 weeks)

---

## CRITICAL SUCCESS FACTORS

1. **Preserve binary compatibility**: All 545 binaries must still work
2. **No breaking changes**: Existing Titan code continues to compile
3. **Gradual migration**: Move modules incrementally, test after each
4. **Clear dependencies**: Use omnisystem.toml to enforce boundaries
5. **Hot-reload ready**: Architecture must support dynamic loading

---

## RISK MITIGATION

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Circular dependencies | Medium | High | Pre-scan before migration |
| Broken imports | High | High | Automated import updater |
| Missing capabilities | Medium | Medium | Comprehensive audit first |
| Build system complexity | Medium | High | Start with simple scanner |
| State migration issues | Low | High | Implement carefully in Aether |

---

## NEXT STEP

**Approve this plan to begin PHASE A immediately.**

Once approved:
1. Create `/modules/` subdirectories (core, languages, services, applications)
2. Begin moving modules systematically
3. Track progress against this timeline
4. Report daily status

**Estimated completion**: June 28, 2026 (2 weeks from today)

Then proceed to Phase 2 (Universal Module System hardening) and Phase 3 (full integration).
