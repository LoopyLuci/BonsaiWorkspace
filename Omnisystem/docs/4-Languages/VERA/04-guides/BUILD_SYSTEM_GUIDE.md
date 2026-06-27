# Build System Guide - Complete Compilation Reference

**Comprehensive guide to building, compiling, and deploying Omnisystem projects**

---

## Build System Overview

The Omnisystem Build System provides:
- **Multi-Language Support** - TITAN, SYLVA, AETHER, AXIOM
- **Cross-Platform Compilation** - Windows, macOS, Linux, WASM, mobile
- **Incremental Compilation** - Fast rebuilds
- **Optimization Levels** - Debug through aggressive optimization
- **Parallel Building** - Multi-threaded compilation
- **Dependency Management** - Automatic dependency resolution
- **Build Caching** - Accelerate builds
- **Profiling & Analysis** - Timing and bottleneck detection

---

## Project Structure

### Standard Layout

```
project/
├── Cargo.toml              # Project manifest
├── omnisystem.toml         # Omnisystem configuration
├── src/
│   ├── main.ti            # Entry point
│   ├── lib.ti             # Library root
│   ├── bin/
│   │   └── tool.ti        # Binary target
│   └── modules/
│       ├── math.ti
│       ├── graphics.ti
│       └── network.ti
├── tests/
│   ├── integration_test.ti
│   └── common/
│       └── helpers.ti
├── benches/
│   └── performance.ti
├── examples/
│   ├── basic.ti
│   └── advanced.ti
├── docs/
│   └── README.md
├── resources/
│   ├── shaders/
│   ├── textures/
│   └── data/
└── target/
    ├── debug/
    ├── release/
    └── build/
```

### Cargo.toml Structure

```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2024"
description = "My awesome project"
authors = ["Your Name <email@example.com>"]
license = "MIT"
repository = "https://github.com/user/project"

[dependencies]
omnisystem = "0.1"
graphics = { version = "0.1", features = ["vulkan"] }
audio = { version = "0.1", optional = true }
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
criterion = "0.5"

[features]
default = ["graphics"]
graphics = []
audio = []
gpu-acceleration = []

[profile.dev]
opt-level = 0
debug = true
debug-assertions = true
overflow-checks = true
lto = false

[profile.release]
opt-level = 3
debug = false
debug-assertions = false
overflow-checks = false
lto = true
codegen-units = 1

[[bin]]
name = "myapp"
path = "src/main.ti"

[[example]]
name = "basic"
path = "examples/basic.ti"

[[bench]]
name = "performance"
harness = false
```

---

## Building Projects

### Command-Line Build

```bash
# Build debug version
omnisystem build

# Build release version (optimized)
omnisystem build --release

# Build for specific target
omnisystem build --target x86_64-windows-msvc
omnisystem build --target aarch64-apple-darwin

# Build with specific features
omnisystem build --features "graphics,audio"

# Build with custom flags
omnisystem build --opt-level 2 --jobs 4

# Verbose output
omnisystem build --verbose

# Check compilation without linking
omnisystem check

# Clean build artifacts
omnisystem clean
```

### Build Targets

```
Target Triple                   Platform
-----------                     --------
x86_64-pc-windows-msvc          Windows 64-bit (MSVC)
x86_64-pc-windows-gnu           Windows 64-bit (MinGW)
i686-pc-windows-msvc            Windows 32-bit (MSVC)
x86_64-apple-darwin             macOS Intel
aarch64-apple-darwin            macOS Apple Silicon
x86_64-unknown-linux-gnu        Linux x64
aarch64-unknown-linux-gnu       Linux ARM64
wasm32-unknown-unknown          WebAssembly
aarch64-linux-android           Android ARM64
armv7-linux-androideabi         Android ARM32
```

### Configuration Profiles

```toml
[profile.dev-fast]
inherits = "dev"
opt-level = 1
codegen-units = 256

[profile.release-with-debug]
inherits = "release"
debug = true
strip = false
```

---

## Optimization

### Optimization Levels

```
Level   Flag          Purpose
-----   ----          -------
0       -O0           No optimization (debug builds)
1       -O1           Basic optimizations
2       -O2           Recommended for production
3       -O3           Aggressive optimizations
s       -Os           Size optimization
z       -Oz           Minimal size

omnisystem build --opt-level 3
```

### Performance Optimization Flags

```toml
[profile.release]
opt-level = 3              # Maximum optimization
lto = true                 # Link-time optimization
codegen-units = 1          # Single codegen unit (slower build, faster code)
panic = "abort"            # Faster panics
strip = true               # Strip symbols
```

### Runtime Optimization

```toml
[profile.release]
# SIMD optimizations
target-cpu = "native"      # Optimize for host CPU

# Vectorization
target-feature = "+avx2"   # Enable AVX2

# GPU compilation
gpu-opt-level = 3
```

---

## Incremental Compilation

### Build Caching

```bash
# Enable incremental compilation
OMNISYSTEM_INCREMENTAL=1 omnisystem build

# Clear build cache
omnisystem clean

# Partial clean
omnisystem clean --target x86_64-windows-msvc
```

### Watch Mode

```bash
# Watch for changes and rebuild
omnisystem watch

# Watch and run
omnisystem watch --run

# Watch with custom command
omnisystem watch -- cargo test
```

---

## Testing

### Running Tests

```bash
# Run all tests
omnisystem test

# Run specific test
omnisystem test test_addition

# Run tests matching pattern
omnisystem test math::

# Run with backtrace on panic
RUST_BACKTRACE=1 omnisystem test

# Run ignored tests
omnisystem test -- --ignored

# Run single-threaded
omnisystem test -- --test-threads=1
```

### Test Macros

```titan
#[test]
fun test_basic() {
    assert_eq!(2 + 2, 4)
}

#[test]
#[should_panic(expected = "division by zero")]
fun test_panic() {
    let _ = 10 / 0
}

#[test]
#[ignore]
fun test_slow() {
    // Runs only with --ignored flag
}
```

---

## Benchmarking

### Benchmark Configuration

```toml
[[bench]]
name = "performance"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

### Benchmark Code

```titan
use criterion::{black_box, criterion_group, criterion_main, Criterion}

fun fibonacci(n: u32) -> u32 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fun bench_fib(c: &mut Criterion) {
    c.bench_function("fib 10", |b| b.iter(|| fibonacci(black_box(10))))
}

criterion_group!(benches, bench_fib)
criterion_main!(benches)
```

### Running Benchmarks

```bash
omnisystem bench

omnisystem bench --bench performance

omnisystem bench -- --verbose --sample-size 100
```

---

## Cross-Compilation

### Setup Toolchain

```bash
# Add target support
omnisystem target add x86_64-windows-msvc
omnisystem target add aarch64-apple-darwin
omnisystem target add wasm32-unknown-unknown

# List installed targets
omnisystem target list
```

### Building for Different Targets

```bash
# Build for all targets
omnisystem build --all-targets

# Cross-compile
omnisystem build --target aarch64-unknown-linux-gnu

# For WebAssembly
omnisystem build --target wasm32-unknown-unknown
omnisystem wasm-pack build --target web
```

---

## Dependency Management

### Adding Dependencies

```toml
[dependencies]
# Latest version
omnisystem = "*"

# Semantic versioning
serde = "1.0"
serde = "1.*"
serde = "~1.0"

# Exact version
tokio = "=1.0.0"

# From git
mystuff = { git = "https://github.com/user/mystuff" }
mystuff = { git = "...", branch = "dev" }
mystuff = { git = "...", tag = "v1.0" }

# From local path
mylib = { path = "../mylib" }

# Optional dependency
audio = { version = "0.1", optional = true }
gpu = { version = "0.2", optional = true }

[features]
default = ["audio"]
all-features = ["audio", "gpu"]
no-default-features = []
```

### Lock Files

```bash
# Generate lock file
omnisystem lock

# Update to latest compatible versions
omnisystem update

# Update specific dependency
omnisystem update serde
```

---

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build & Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v2
      - name: Install Omnisystem
        run: curl https://install.omnisystem.io | sh
      - name: Build
        run: omnisystem build --release
      - name: Test
        run: omnisystem test --release
      - name: Benchmark
        run: omnisystem bench --release
```

---

## Publishing

### Package Configuration

```toml
[package]
name = "my-package"
version = "0.1.0"
edition = "2024"
description = "My awesome package"
documentation = "https://docs.rs/my-package"
repository = "https://github.com/user/my-package"
homepage = "https://example.com"
keywords = ["graphics", "3d", "rendering"]
categories = ["graphics", "game-engines"]
license = "MIT"

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

### Publishing to Registry

```bash
# Login to registry
omnisystem login

# Check publishability
omnisystem package --allow-dirty

# Publish
omnisystem publish

# Publish with dry-run
omnisystem publish --dry-run
```

---

## Build Profiling

### Compilation Time Analysis

```bash
# Profile compilation time
omnisystem build --timings

# Detailed timing
omnisystem build --verbose --timings

# Analyze build graph
omnisystem metadata
```

### Binary Size Analysis

```bash
# Check binary size
ls -lh target/release/myapp

# Strip symbols
strip target/release/myapp

# Use cargo-bloat
omnisystem bloat --release
```

---

## Parallel Compilation

### Controlling Parallelism

```bash
# Use 4 parallel jobs
omnisystem build --jobs 4

# Use all available CPU cores
omnisystem build --jobs $(nproc)

# Sequential build (1 job)
omnisystem build --jobs 1
```

---

## Build Scripts

### build.rs Pre-build

```titan
// build.rs
fun main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate code
    println!("cargo:rustc-env=BUILD_TIME={}", 
        std::env::var("OMNISYSTEM_BUILD_TIME")?);
    
    // Link libraries
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=dylib=mylib");
    
    // Rerun on file change
    println!("cargo:rerun-if-changed=src/lib.rs");
    
    Ok(())
}
```

---

## Common Issues

### Issue: Slow Compilation

**Solution:**
```bash
# Use sccache for caching
export RUSTC_WRAPPER=sccache

# Reduce codegen units in dev
[profile.dev]
codegen-units = 256

# Separate compilation per feature
omnisystem build --no-default-features --features "one-feature"
```

### Issue: Out of Memory

**Solution:**
```bash
# Reduce parallel jobs
omnisystem build --jobs 1

# Reduce optimization level
omnisystem build --opt-level 1
```

### Issue: Linking Errors

**Solution:**
```bash
# Check library paths
omnisystem build --verbose

# Add library path
export LIBRARY_PATH="/usr/local/lib:$LIBRARY_PATH"

# Link to specific library version
```

---

## Best Practices

✅ **DO**
- Use semantic versioning
- Pin dependencies for releases
- Run tests before publishing
- Use build scripts for code generation
- Profile optimization impact

❌ **DON'T**
- Use unbounded version ranges in production
- Ignore compiler warnings
- Commit lock files in libraries
- Over-optimize prematurely
- Skip testing on target platforms

---

## Next Steps

- [PACKAGE_MANAGER.md](PACKAGE_MANAGER.md) - Dependency management
- [CI_CD_GUIDE.md](CI_CD_GUIDE.md) - Continuous integration
- [PERFORMANCE_PROFILING.md](PERFORMANCE_PROFILING.md) - Optimization tools

---

**Build System** - Flexible, fast, and powerful compilation!
