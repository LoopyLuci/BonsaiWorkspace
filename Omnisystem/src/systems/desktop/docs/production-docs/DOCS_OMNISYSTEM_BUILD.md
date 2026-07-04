# Omnisystem Build Guide

**Complete build instructions for all platforms and targets.**

---

## Quick Reference

| Target | Command | Time | Output |
|--------|---------|------|--------|
| **Development (QEMU)** | `make qemu` | 10 min | Bootable QEMU image |
| **Full ISO** | `make image` | 15 min | omnisystem.iso (bootable) |
| **QCOW2 disk** | `make disk` | 10 min | omnisystem.qcow2 (for VMs) |
| **Container** | `make container` | 5 min | Docker image |
| **Bare-metal** | `make iso && dd` | 15 min | Bootable USB stick |
| **Library OS** | `make library-os` | 5 min | libomnisystem.a + binaries |

## Prerequisites

### Minimal (for development)

- **OS**: Linux (Ubuntu 22.04+), macOS (12+), or Windows (WSL2)
- **RAM**: 8GB minimum
- **Disk**: 50GB free space
- **Rust**: 1.75+ (install via `rustup.rs`)
- **Build tools**: GCC/Clang, Make, Git

### Full (for release builds)

- Everything above, plus:
- **QEMU**: 4.0+ (for testing)
- **Axiom**: Proof checker (optional, for verification)
- **Cargo-watch**: For incremental builds
- **Time**: ~30 minutes for first full build

### Platform-Specific

**Linux (Ubuntu/Debian)**:
```bash
sudo apt update
sudo apt install -y \
  build-essential gcc g++ make \
  git curl wget \
  qemu-system-x86 libvirt-daemon \
  rustup
```

**macOS (Homebrew)**:
```bash
brew install rust llvm cmake qemu
```

**Windows (WSL2 + Rust)**:
```bash
wsl --install
# Inside WSL:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install -y build-essential qemu-system-x86
```

## Setup

### 1. Clone Repository

```bash
git clone https://github.com/your-org/omnisystem.git
cd omnisystem

# Clone or download UOSC kernel (as submodule)
git submodule update --init --recursive
# OR if UOSC is separate:
git clone https://github.com/your-org/uosc.git kernel
```

### 2. Install Dependencies

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
make deps

# This will:
# - Install Rust toolchains
# - Download UOSC kernel (if needed)
# - Set up build environment
```

### 3. Verify Setup

```bash
make check
# This will verify:
# - Rust version
# - Build tools available
# - UOSC kernel present
# - Disk space available
# - Output: "✓ Setup verified"
```

## Building

### Development Build (QEMU)

Fast, unoptimized build for testing:

```bash
# Build all components for QEMU
make qemu

# Or step-by-step:
make kernel              # Build UOSC kernel
make languages           # Build Titan, Sylva, Aether, Axiom
make services            # Build all services
make apps               # Build user applications
make image-qemu         # Create bootable image

# Output: Ready to run in QEMU (see below)
```

**Time**: ~10 minutes (on 8-core CPU with SSD)

### Release Build (Optimized)

Fully optimized, slower build:

```bash
# Full release build
make release            # equivalent to: make all RELEASE=1

# Or with verification:
make release VERIFY=1   # also verify all Axiom proofs

# Output:
# - omnisystem.iso (bootable CD)
# - omnisystem.qcow2 (QEMU/KVM disk image)
# - omnisystem-container.tar (Docker image)
```

**Time**: ~20 minutes (first time), ~5 minutes (incremental)

### Incremental Build

Rebuild only changed components:

```bash
# Automatically rebuild on file changes
make watch

# Or manually build specific component:
make build-service SERVICE=ai-shim      # Just AI service
make build-lang LANG=sylva              # Just Sylva compiler
make build-app APP=workspace            # Just Workspace app
```

### Cross-Compilation

Build for different architecture:

```bash
# Build for ARM64 (on x86-64 host)
make all ARCH=aarch64

# Build for RISC-V
make all ARCH=riscv64

# Build for 32-bit x86
make all ARCH=i686
```

## Build Configuration

Edit `build.toml` to customize:

```toml
[build]
# Optimization level: 0 (dev), 1, 2, 3 (release)
optimization = 3

# Enable formal verification (requires axiom binary)
verify = false

# Number of parallel build jobs
jobs = 8

# Enable debug symbols
debug = true

# Strip binaries (release only)
strip = true

[services]
# Which services to include
include = [
    "transfer-daemon",
    "ums",
    "ai-shim",
    "service-manager",
    "filesystem",
    "network-stack",
]

[languages]
# Which languages to include
titan = true
sylva = true
aether = true
axiom = true

[apps]
# Which apps to include
workspace = true
buddy = false  # Mobile-only
```

## Testing During Build

### Unit Tests

```bash
# Run all unit tests
make test-unit

# Test specific service
make test SERVICE=ai-shim

# Test specific language
make test LANG=sylva

# Verbose output
make test VERBOSE=1
```

### Integration Tests

```bash
# Boot Omnisystem + run integration tests
make test-integration

# Run specific test
make test-integration TEST=boot
make test-integration TEST=ai-fallback
```

### Performance Benchmarks

```bash
# Run all benchmarks
make bench

# Specific benchmark
make bench BENCH=ipc-latency
make bench BENCH=ai-inference
```

## Running the Build

### QEMU (Development)

```bash
# Boot in QEMU (graphical)
make qemu
# Or with arguments:
make qemu QEMU_ARGS="-m 8192 -cpu host -enable-kvm"

# SSH into running QEMU instance
make qemu-ssh

# Halt QEMU
make qemu-halt
```

### KVM (Linux)

```bash
# Boot using KVM (faster than QEMU)
qemu-system-x86_64 \
  -drive file=omnisystem.qcow2 \
  -m 4096 \
  -cpu host \
  -enable-kvm \
  -nographic

# For GUI:
# Remove `-nographic` flag
```

### Hyper-V (Windows)

```powershell
# Create VM
New-VM -Name Omnisystem `
  -MemoryStartupBytes 4GB `
  -SwitchName Default `
  -VHDPath "omnisystem.vhdx"

# Start VM
Start-VM -Name Omnisystem
```

### Bare-Metal (USB)

```bash
# Create bootable USB stick
sudo dd if=omnisystem.iso of=/dev/sdX bs=4M conv=fsync

# Or use Etcher (GUI):
# - Download balena-etcher
# - Select omnisystem.iso
# - Select USB stick
# - Write

# Boot from USB
# (insert USB, reboot, select as boot device)
```

### Docker Container

```bash
# Load container image
docker load < omnisystem-container.tar

# Run container
docker run -it omnisystem:latest /bin/sylva

# Run with GPU support
docker run -it --gpus all omnisystem:latest

# Run with volume mount
docker run -it -v ~/mydata:/data omnisystem:latest
```

## Troubleshooting Build Issues

### "Out of disk space"

```bash
# Clean build artifacts
make clean              # Remove build/ directory

# Or just remove old artifacts
rm -rf build/           # Frees ~30GB
```

### "Rust version too old"

```bash
# Update Rust
rustup update

# Check version
rustc --version         # should be 1.75+
```

### "UOSC kernel not found"

```bash
# Re-clone submodules
git submodule update --init --recursive

# Or manually:
git clone https://github.com/your-org/uosc.git kernel
```

### "Build fails with linker error"

```bash
# Clean rebuild
make clean
make release

# Or update linker
sudo apt install lld    # Linux
brew install lld        # macOS
```

### "Tests fail in CI but pass locally"

This usually means non-determinism. Check:

```bash
# Run tests multiple times
for i in {1..5}; do make test || break; done

# Run with different seed
make test SEED=12345
```

## Custom Build Targets

### Build just the kernel (UOSC)

```bash
make kernel
# Output: kernel.elf (bootable)
```

### Build just the languages

```bash
# Titan only
make build-lang LANG=titan

# All languages
make languages

# With Axiom proofs verified
make languages VERIFY=1
```

### Build just the services

```bash
# All services
make services

# Specific service
make build-service SERVICE=transfer-daemon

# Multiple services
make build-service SERVICES="ai-shim,ums,transfer-daemon"
```

### Build just the apps

```bash
# Just Bonsai Workspace
make build-app APP=workspace

# Just Buddy (mobile app)
make build-app APP=buddy

# All apps
make apps
```

### Library OS mode (embedded)

```bash
# Build Omnisystem as a static library
make library-os
# Output: libomnisystem.a

# Link with C program
gcc my_program.c -L. -lomnisystem -o my_program
```

## Incremental Builds

Omnisystem uses **incremental compilation** to speed up rebuilds:

```bash
# First build: ~10 minutes
make release

# After changing one file: ~30 seconds
# Only changed service/language rebuilt
touch services/ai-shim/src/main.ti
make release
```

To enable persistent incremental cache:

```bash
# Use sccache (shared compilation cache)
make release CACHE=sccache

# Sccache config:
export RUSTC_WRAPPER=sccache
export SCCACHE_CACHE_SIZE=10G
```

## Parallel Build

By default, builds use all available CPU cores:

```bash
# Limit to 4 cores
make release JOBS=4

# Use 1 core only (debugging)
make release JOBS=1
```

## Formal Verification During Build

```bash
# Include verification in build
make release VERIFY=1

# This will:
# - Build all components
# - Check all Axiom proofs
# - Verify AI Shim safety theorem
# - Verify UMS integrity theorem
# - Output: proof verification status

# Just verify without rebuilding
make verify
```

## CI/CD Integration

GitHub Actions workflows are included:

```yaml
# .github/workflows/build.yml runs on every push
# - Builds on Linux, macOS, Windows
# - Runs unit + integration tests
# - Uploads build artifacts
# - On release: builds installers for all platforms
```

## Build Artifacts

After successful build, outputs are in:

```
build/
├── kernel.elf               # UOSC kernel (bootable)
├── libomnisystem.a          # Static library (library OS mode)
├── omnisystem.iso           # ISO image (bare-metal)
├── omnisystem.qcow2         # QCOW2 disk (QEMU/KVM)
├── omnisystem.vhdx          # VHD disk (Hyper-V)
├── omnisystem-container.tar # Docker image
├── bonsai-installer.exe     # Windows installer
├── bonsai-installer.dmg     # macOS installer
├── bonsai-installer.deb     # Linux installer
├── omnisystem.jar           # Java artifact (if applicable)
└── logs/                    # Build logs
    ├── build.log            # Build output
    ├── test.log             # Test output
    └── verify.log           # Proof verification output
```

## Performance Tuning

### Build Speed

```bash
# Use mold linker (faster than GNU ld)
make release LINKER=mold

# Use LTO (link-time optimization) - slower build, faster binary
make release LTO=thin  # thin LTO
make release LTO=full  # full LTO (very slow)

# Parallel jobs matching CPU
make release JOBS=$(($(nproc)-1))
```

### Binary Size

```bash
# Strip debug symbols
make release STRIP=1    # reduces ~30% size

# Use UPX compression (optional)
make release UPX=1      # further reduces size
```

## Next Steps

After successful build:

1. **Test**: `make test` to run full test suite
2. **Boot**: `make qemu` to try in QEMU
3. **Deploy**: See [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment
4. **Contribute**: See [CONTRIBUTING.md](CONTRIBUTING.md) to contribute changes

---

**Build Guide Version**: 1.0.0  
**Last Updated**: 2026-06-08

