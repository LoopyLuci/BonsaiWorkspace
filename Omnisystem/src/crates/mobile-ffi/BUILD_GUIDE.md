# Build Guide - Bonsai Mobile FFI

Complete step-by-step guide to building the Android hardware-accelerated video decoder from source.

## Overview

This crate compiles to `libbonsai_mobile_ffi.so`, a native Android library providing JNI bindings to hardware-accelerated H.264/H.265 video decoding via MediaCodec.

**Build Target:** aarch64-linux-android (ARM64)
**Output:** `libbonsai_mobile_ffi.so` (~2-4MB)
**Compilation Time:** ~5-15 minutes (first build), ~30-60 seconds (incremental)

## System Requirements

### Hardware
- Minimum 4GB RAM (8GB+ recommended for first build)
- 2GB disk space for Android NDK + build artifacts
- Modern CPU (build speed: 5-15 min depending on CPU)

### Software
- **Rust:** 1.70+ with Cargo
- **Android NDK:** r25.1.8937393 (or compatible r25.x)
- **Android SDK:** API 21+
- **Build Tools:**
  - cargo-ndk (Rust to NDK cross-compiler)
  - clang (included in NDK)
  - Python 3.x (for NDK tools)

### Operating System
- Linux (recommended): Ubuntu 20.04+, Fedora 36+, Debian 11+
- macOS: 10.15+ (Intel or Apple Silicon)
- Windows: Windows 10/11 with WSL2 or native MSVC toolchain

## Pre-Build Checklist

Before starting, verify your system is ready:

```bash
# 1. Check Rust version
rustc --version  # Should be 1.70+
cargo --version

# 2. Check Android targets
rustup target list | grep android

# 3. Check cargo-ndk installation
cargo ndk --version

# 4. Verify NDK location
echo $ANDROID_NDK_HOME  # Should print NDK path
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/*/bin/aarch64-linux-android-clang
```

If any check fails, see the **Installation** section below.

## Installation

### Step 1: Install Rust (if not already installed)

```bash
# Download and install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify
rustc --version
```

### Step 2: Add Android Targets

```bash
# Add aarch64 target (main target for modern phones)
rustup target add aarch64-linux-android

# Optional: Add other architectures
rustup target add armv7-linux-android    # Older phones
rustup target add x86_64-linux-android   # Emulator
```

### Step 3: Install cargo-ndk

```bash
# Install cross-compilation tool
cargo install cargo-ndk

# Verify
cargo ndk --version
# Expected output: cargo-ndk 0.14.2 or higher
```

### Step 4: Install Android NDK

**Option A: Via Android Studio (easiest)**
1. Open Android Studio
2. Tools → SDK Manager
3. SDK Tools tab → Android NDK (Side by side)
4. Download version r25.1.8937393
5. NDK is installed at: `~/Android/sdk/ndk/25.1.8937393`

**Option B: Manual download**
```bash
# Download NDK from: https://developer.android.com/ndk/downloads
# Extract to preferred location
mkdir -p ~/android-ndk
cd ~/android-ndk
unzip android-ndk-r25.1.8937393-linux.zip
```

### Step 5: Set Environment Variables

**Linux/macOS (add to ~/.bashrc or ~/.zshrc):**
```bash
export ANDROID_NDK_HOME=$HOME/Android/sdk/ndk/25.1.8937393
# or:
export ANDROID_NDK_HOME=$HOME/android-ndk/android-ndk-r25.1.8937393
```

**Windows (PowerShell):**
```powershell
$env:ANDROID_NDK_HOME = "C:\Android\ndk\25.1.8937393"
[Environment]::SetEnvironmentVariable("ANDROID_NDK_HOME", "C:\Android\ndk\25.1.8937393", "User")
```

**Verify:**
```bash
echo $ANDROID_NDK_HOME
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/*/bin/clang
```

## Building

### Build for Android ARM64 (Primary Target)

```bash
cd crates/bonsai-mobile-ffi

# Option 1: Using environment variable
cargo ndk -t arm64-v8a build --release

# Option 2: Explicit NDK path
cargo ndk -t arm64-v8a --ndk $ANDROID_NDK_HOME build --release

# Option 3: From workspace root
cargo ndk -p bonsai-mobile-ffi -t arm64-v8a build --release
```

**Output:** `target/aarch64-linux-android/release/libbonsai_mobile_ffi.so`

### Verify Build Output

```bash
# Check file size
ls -lh target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# Expected: 2-4 MB

# Check architecture
file target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# Expected: ELF 64-bit LSB shared object, ARM aarch64

# Check symbols (optional, requires aarch64-linux-android-nm)
aarch64-linux-android-nm target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
```

### Optional: Build for Multiple Architectures

```bash
# All supported architectures
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 --ndk $ANDROID_NDK_HOME build --release

# Individual builds
cargo ndk -t arm64-v8a build --release    # aarch64 (64-bit ARM)
cargo ndk -t armeabi-v7a build --release  # armv7 (32-bit ARM, older phones)
cargo ndk -t x86_64 build --release       # x86_64 (emulator, some tablets)
```

## Running Tests

### Unit Tests

```bash
cd crates/bonsai-mobile-ffi

# Run all tests
cargo test

# Run specific test
cargo test test_decode_frame

# Run with output
cargo test -- --nocapture
```

### Benchmark Tests

```bash
# Run decode latency benchmarks
cargo bench --bench decode_latency

# Run throughput benchmarks
cargo bench --bench throughput

# Full benchmark suite
cargo bench
```

**Benchmark output:**
```
test result: ok. 0 measured
decode_1080p_frame              time:   [5.234 ms 5.456 ms 5.712 ms]
decode_720p_frame               time:   [2.134 ms 2.245 ms 2.389 ms]
get_output_frame                time:   [1.024 ms 1.089 ms 1.167 ms]
```

### Integration Tests

On Android device via Gradle (see INTEGRATION.md).

## Troubleshooting Build Issues

### "cargo-ndk not found"
```bash
cargo install cargo-ndk
export PATH="$HOME/.cargo/bin:$PATH"
```

### "target aarch64-linux-android not installed"
```bash
rustup target add aarch64-linux-android
rustup target list | grep android
```

### "Cannot find NDK"
```bash
export ANDROID_NDK_HOME=/path/to/android-ndk
# or
cargo ndk --ndk /path/to/ndk -t arm64-v8a build --release
```

### "Out of memory during compilation"
```bash
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
cargo clean
cargo ndk -t arm64-v8a build --release
```

### "Linker not found"
```bash
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/*/bin/aarch64-linux-android-clang
# If not found, reinstall NDK r25.1+
```

## Integration Into Android Project

After building, integrate into your Android app:

### 1. Copy Library to JNI Directory

```bash
mkdir -p app/src/main/jniLibs/arm64-v8a

cp crates/bonsai-mobile-ffi/target/aarch64-linux-android/release/libbonsai_mobile_ffi.so \
   app/src/main/jniLibs/arm64-v8a/
```

### 2. Copy Kotlin Wrapper

```bash
mkdir -p app/src/main/java/com/yourcompany/decoder

cp crates/bonsai-mobile-ffi/BrdfNativeBridge.kt \
   app/src/main/java/com/yourcompany/decoder/

# Update package name in the file
sed -i 's/com\.bonsai\.mobile\.decoder/com.yourcompany.decoder/' \
   app/src/main/java/com/yourcompany/decoder/BrdfNativeBridge.kt
```

### 3. Update Gradle Build Config

See INTEGRATION.md for detailed Gradle setup.

## Build Optimization

### Debug Build (faster for development)

```bash
cargo ndk -t arm64-v8a build  # No --release flag
# Output: target/aarch64-linux-android/debug/libbonsai_mobile_ffi.so
# Size: ~20-40 MB (includes debug symbols)
# Compile time: 1-3 minutes
```

### Release Build (optimized for production)

```bash
cargo ndk -t arm64-v8a build --release
# Output: target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# Size: ~2-4 MB (no debug symbols)
# Compile time: 5-15 minutes
# Performance: Optimized, ~20% faster than debug
```

### Incremental Builds

```bash
# After first build, incremental builds are much faster
# Only changed modules recompile
cargo ndk -t arm64-v8a build --release
# Time: 30-60 seconds (if changes are minimal)
```

## Build Artifacts

### Output Locations

```
target/aarch64-linux-android/
├── release/
│   ├── libbonsai_mobile_ffi.so         ← Production library
│   ├── deps/                           ← Dependencies
│   └── ...
└── debug/
    ├── libbonsai_mobile_ffi.so         ← Debug library with symbols
    └── ...
```

### Library Contents

The compiled `.so` includes:
- JNI bindings (FFI entry points)
- Decoder implementation
- Metrics collection logic
- Error handling
- All linked dependencies (zeroized, once_cell, etc.)

### Size Analysis

```bash
# Unoptimized release build
ls -lh target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# ~3-4 MB

# Further strip symbols (if needed for distribution)
aarch64-linux-android-strip target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# ~1.5-2 MB (more compact)
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build Bonsai Mobile FFI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Add Android targets
        run: rustup target add aarch64-linux-android
      
      - name: Install cargo-ndk
        run: cargo install cargo-ndk
      
      - name: Setup Android NDK
        uses: nttld/setup-ndk@v1
        with:
          ndk-version: r25
      
      - name: Build
        run: cargo ndk -t arm64-v8a build --release
      
      - name: Run tests
        run: cargo test
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: libbonsai_mobile_ffi.so
          path: target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
```

## Clean Rebuild

If something goes wrong, do a clean rebuild:

```bash
cd crates/bonsai-mobile-ffi

# Remove all build artifacts
cargo clean

# Rebuild from scratch
cargo ndk -t arm64-v8a build --release

# Verify output
ls -lh target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
file target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
```

## Performance Expectations

### Build Times (first build)

| Machine | Specs | Time |
|---------|-------|------|
| M1 MacBook Pro | 8-core CPU, 16GB RAM | 5-8 min |
| Ryzen 5 5600X | 6-core CPU, 16GB RAM | 8-12 min |
| Intel i5 | 4-core CPU, 8GB RAM | 15-20 min |
| GitHub Actions | 2-core VM, 4GB RAM | 20-30 min |

### Incremental Build Times

After first build, changes trigger incremental compilation:
- Small change: 30-60 seconds
- FFI layer change: 2-3 minutes
- Decoder logic change: 3-5 minutes
- Full library rebuild: 5-15 minutes

### Runtime Performance (on Redmi Note 12 Pro 5G)

| Operation | Latency | Throughput |
|-----------|---------|-----------|
| Decode frame (1080p) | 5-10ms | 120+ Mbps |
| Get metrics | <1ms | N/A |
| Release buffer | <1ms | N/A |
| Decoder init | 10-20ms | N/A |

## Advanced Options

### Custom Compiler Flags

```bash
# Set custom linker flags
RUSTFLAGS="-C link-arg=-fuse-ld=lld" \
  cargo ndk -t arm64-v8a build --release

# Enable LTO for smaller binary
RUSTFLAGS="-C lto=fat" \
  cargo ndk -t arm64-v8a build --release
```

### Verbose Build Output

```bash
# See all compiler commands
CARGO_LOG=debug cargo ndk -t arm64-v8a build --release 2>&1 | tee build.log
```

### Profile Build Performance

```bash
# Measure build time and bottlenecks
time cargo ndk -t arm64-v8a build --release

# Use cargo-build-time (if installed)
cargo install cargo-build-time
cargo build-time ndk -t arm64-v8a build --release
```

## Next Steps

1. **Verify the build:** See TROUBLESHOOTING.md if issues occur
2. **Integrate into Android app:** Follow INTEGRATION.md
3. **Run tests on device:** See INTEGRATION.md testing section
4. **Monitor performance:** Use included metrics collection

## Support

For build issues:
1. Check TROUBLESHOOTING.md
2. Verify environment variables: `echo $ANDROID_NDK_HOME`
3. Check cargo-ndk version: `cargo ndk --version`
4. Review Rust target status: `rustup target list`
5. File issue with full build log: `cargo ndk -t arm64-v8a build --release 2>&1 | tee build.log`
