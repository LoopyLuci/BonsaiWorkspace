# Omnisystem - Getting Started Guide

**Repository**: Omnisystem  
**Team**: Omnisystem Team  
**Version**: 1.0.0  
**Last Updated**: 2026-06-12  
**Status**: ✅ Production Ready  
**Difficulty**: Intermediate  
**Estimated Setup Time**: 30-45 minutes  

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Project Structure](#project-structure)
5. [Common Tasks](#common-tasks)
6. [Troubleshooting](#troubleshooting)

---

## System Requirements

### Minimum Specifications

- **OS**: Linux, macOS, or Windows 10/11
- **RAM**: 8 GB (16 GB recommended)
- **Disk Space**: 50 GB free
- **CPU**: 4 cores (8+ recommended)
- **Network**: Broadband internet connection

### Required Software

- **Rust**: 1.70+ (stable)
- **Git**: 2.30+
- **Node.js**: 18+ (for frontend development)
- **Docker**: 20.10+ (optional, for containerization)
- **kubectl**: 1.27+ (optional, for Kubernetes deployment)

### Development Tools (Optional)

- **VS Code** with Rust Analyzer extension
- **JetBrains CLion** or **IntelliJ IDEA**
- **Git GUI clients** (GitHub Desktop, GitKraken, etc.)
- **Docker Desktop**
- **Postman** or **Insomnia** (API testing)

---

## Installation

### Step 1: Install Rust

```bash
# Download and install Rust (Linux/macOS)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# For Windows: Download from https://rustup.rs/

# Verify installation
rustc --version
cargo --version
```

### Step 2: Install System Dependencies

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  libssl-dev \
  libgit2-dev \
  pkg-config \
  git \
  curl
```

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Or install via Homebrew
brew install git libgit2 openssl pkg-config
```

#### Windows (PowerShell)
```powershell
# Install Visual C++ build tools
# Download from: https://visualstudio.microsoft.com/downloads/

# Or use Chocolatey
choco install visualstudio2022buildtools
```

### Step 3: Clone Repository

```bash
# Clone with HTTPS
git clone https://github.com/LoopyLuci/Omnisystem.git
cd Omnisystem

# Or clone with SSH (if you have SSH key set up)
git clone git@github.com:LoopyLuci/Omnisystem.git
cd Omnisystem
```

### Step 4: Configure Rust Toolchain

```bash
# Update Rust
rustup update stable

# Install required components
rustup component add rustfmt clippy

# Install useful tools
cargo install cargo-watch    # Auto-rebuild on file changes
cargo install cargo-edit     # Easy dependency management
cargo install cargo-outdated # Check for outdated dependencies
cargo install cargo-audit    # Security audit
```

### Step 5: Verify Installation

```bash
# Check Rust version
rustc --version

# Check Cargo version
cargo --version

# Build foundation crates (quick test)
cargo build -p omnisystem-ums

# Run tests
cargo test --lib --workspace --release
```

---

## Quick Start

### 1. Build the Workspace

```bash
# Full workspace build (first time, ~5-10 minutes)
cargo build --workspace --release

# Or use the build script
./scripts/build_all.sh release

# Build specific system
cargo build -p pathfinder-user-service --release
```

### 2. Run Tests

```bash
# Run all tests
cargo test --workspace --release

# Or use the test script
./scripts/test_all.sh unit

# Run specific test
cargo test -p pathfinder-user-service test_user_auth --release
```

### 3. Start Development

```bash
# Install development dependencies
rustup component add rust-analyzer

# Open in VS Code
code .

# Or start a development build with auto-reload
cargo watch -x "build --workspace"
```

### 4. Run a Service Locally

```bash
# Start PATHFINDER user service (requires database)
cargo run -p pathfinder-user-service --release

# Or start with environment variables
DATABASE_URL="postgresql://user:pass@localhost/pathfinder" \
  cargo run -p pathfinder-user-service --release
```

### 5. Check Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace -- -D warnings

# Check for security vulnerabilities
cargo audit

# Run all checks at once
cargo check --workspace && \
  cargo fmt --all -- --check && \
  cargo clippy --workspace -- -D warnings && \
  cargo test --workspace
```

---

## Project Structure

```
Omnisystem/
├── README.md (project overview)
├── Cargo.toml (workspace definition, 228+ crates)
├── Cargo.lock (dependency versions lock file)
│
├── crates/ (228+ functional crates)
│   ├── launcher-core/ (launcher kernel)
│   ├── launcher/ (launcher daemon)
│   ├── app-menu/ (application menu UI)
│   ├── pre-launcher/ (bootstrap launcher)
│   ├── advanced-launcher/ (advanced features)
│   ├── omnisystem-cicd/ (native CI/CD)
│   ├── omnisystem-kernel/ (core runtime)
│   ├── omnisystem-ffi/ (foreign function interface)
│   ├── network-firmware/ (Network/IoT support)
│   ├── ai-advisor/ (AI systems)
│   └── ... (200+ additional crates)
│
├── scripts/ (build & test automation)
│   ├── build_all.sh (build entire workspace)
│   ├── test_all.sh (run all tests)
│   └── ci.sh (CI/CD pipeline)
│
├── docker/ (container configuration)
│   ├── Dockerfile.dev (development image)
│   ├── Dockerfile.prod (production image)
│   └── docker-compose.yml (services orchestration)
│
├── kubernetes/ (K8s deployment)
│   ├── namespace.yaml
│   ├── deployments.yaml
│   ├── services.yaml
│   └── ingress.yaml
│
├── docs/ (documentation)
│   ├── ARCHITECTURE.md
│   ├── CONTRIBUTING.md
│   ├── API.md
│   └── ...
│
├── .github/workflows/ (GitHub Actions CI/CD)
│   ├── ci.yml
│   ├── build.yml
│   └── deploy.yml
│
└── WORKSPACE_BUILD_SYSTEM.md (this workspace guide)
```

### Key Directories Explained

| Directory | Purpose |
|-----------|---------|
| `Omnisystem/crates/omnisystem-*` | Foundation runtime and kernel |
| `Omnisystem/crates/pathfinder-*` | Learning platform services |
| `Omnisystem/crates/omnisearch-*` | Full-text search engine |
| `Omnisystem/crates/omnifile-*` | File management system |
| `Omnisystem/crates/ai-*` | AI/ML infrastructure |
| `scripts/` | Build automation and CI scripts |
| `docker/` | Docker images for services |
| `kubernetes/` | K8s manifests for production |
| `docs/` | Technical documentation |

---

## Common Tasks

### Building

```bash
# Build specific crate
cargo build -p omnisystem-kernel

# Build with all features
cargo build --workspace --all-features

# Build optimized release
cargo build --workspace --release

# Build and strip debug symbols
cargo build --workspace --release
strip target/release/binary_name
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p omnisystem-kernel

# Run with output
cargo test --workspace -- --nocapture

# Run benchmarks
cargo bench --workspace
```

### Code Quality

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Lint with clippy
cargo clippy --workspace -- -D warnings

# Security audit
cargo audit --fix

# Generate documentation
cargo doc --workspace --open
```

### Development Workflow

```bash
# Watch for changes and rebuild
cargo watch -x "build --workspace"

# Watch and run tests
cargo watch -x "test --lib"

# Interactive debugging (requires debugger)
rust-gdb target/debug/binary_name
```

### Dependency Management

```bash
# Add new dependency
cargo add serde --features derive

# Update dependencies
cargo update

# Check for outdated packages
cargo outdated

# Remove unused dependencies
cargo tree --workspace --duplicates
```

---

## Environment Configuration

### Database Setup (for PATHFINDER)

```bash
# Create PostgreSQL database
createdb pathfinder

# Run migrations
sqlx migrate run --database-url postgresql://localhost/pathfinder

# Or use Docker
docker run -e POSTGRES_PASSWORD=password \
  -p 5432:5432 postgres:15-alpine
```

### Environment Variables

Create `.env` file in workspace root:

```bash
# Database
DATABASE_URL=postgresql://user:password@localhost:5432/pathfinder
REDIS_URL=redis://localhost:6379

# API Configuration
API_PORT=8001
LOG_LEVEL=info

# Feature flags
ENABLE_LOGGING=true
ENABLE_METRICS=true

# Development
RUST_LOG=debug
RUST_BACKTRACE=1
```

---

## IDE Setup

### Visual Studio Code

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": [
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### IntelliJ IDEA

1. Install Rust plugin
2. Configure toolchain: Settings → Languages & Frameworks → Rust
3. Enable Clippy: Settings → Languages & Frameworks → Rust → Clippy
4. Configure debugger (LLDB on macOS, GDB on Linux)

---

## Performance Tips

### Faster Builds

```bash
# Use sccache for incremental caching
export RUSTC_WRAPPER=sccache

# Reduce codegen units for faster builds
export CARGO_BUILD_PIPELINED_COMPILATION=true

# Use mold linker (Linux)
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Faster Tests

```bash
# Run tests in parallel
cargo test --workspace -- --test-threads=$(nproc)

# Skip slow tests
cargo test --workspace --skip slow_integration_tests

# Run only unit tests
cargo test --lib --workspace
```

---

## Troubleshooting

### Build Issues

**"error: could not compile"**
```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Check Rust version
rustup update stable
```

**"out of memory"**
```bash
# Reduce parallel jobs
cargo build -j 2
```

**"linker not found"**
```bash
# Linux: Install build essentials
sudo apt-get install build-essential

# macOS: Install Xcode tools
xcode-select --install

# Windows: Install Visual C++ build tools
```

### Runtime Issues

**"connection refused" (database)**
```bash
# Check if database is running
ps aux | grep postgres

# Start PostgreSQL
brew services start postgresql  # macOS
sudo systemctl start postgresql # Linux
```

**"permission denied" (scripts)**
```bash
# Make scripts executable
chmod +x scripts/*.sh
./scripts/build_all.sh
```

### Development Issues

**"language server crashed"**
```bash
# Reinstall rust-analyzer
rustup component add rust-analyzer --force-non-host
```

**"tests hanging"**
```bash
# Run with timeout
cargo test --workspace -- --test-threads=1 --timeout 30
```

---

## Next Steps

1. **Read Architecture Guide**: `docs/ARCHITECTURE.md`
2. **Review Contributing**: `docs/CONTRIBUTING.md`
3. **Explore Services**: Start with PATHFINDER services in `Omnisystem/crates/pathfinder-*`
4. **Run Examples**: Check `docs/examples/` for sample code
5. **Join Community**: See `CONTRIBUTING.md` for communication channels

---

## Support

- **GitHub Repository**: https://github.com/LoopyLuci/Omnisystem
- **Issues**: https://github.com/LoopyLuci/Omnisystem/issues
- **Discussions**: https://github.com/LoopyLuci/Omnisystem/discussions
- **API Reference**: Run `cargo doc --open`

---

**Happy coding with Omnisystem!** 🚀
