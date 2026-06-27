# Omnisystem Installation & Setup Guide

## Requirements

### System Requirements
- **OS**: Linux, macOS, or Windows 10+
- **Processor**: x86-64 or ARM64
- **RAM**: 4GB minimum (8GB recommended)
- **Disk Space**: 500MB for installation

### Software Requirements
- **Rust**: 1.70+ (for native extensions)
- **Python**: 3.8+ (for SYLVA ML operations)
- **Node.js**: 16+ (for development tools)
- **Git**: 2.30+ (for version control)

## Installation Steps

### Quick Install (Recommended)

#### Linux/macOS
```bash
# Download and run installer
curl -fsSL https://omnisystem.io/install.sh | bash

# Verify installation
omnisystem --version
```

#### Windows
```powershell
# Download and run installer
Invoke-WebRequest -Uri "https://omnisystem.io/install.ps1" | Invoke-Expression

# Verify installation
omnisystem --version
```

### Manual Installation

#### 1. Download from GitHub
```bash
git clone https://github.com/omnisystem/omnisystem.git
cd omnisystem
```

#### 2. Build from Source (Linux/macOS)
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build Omnisystem
cargo build --release

# Install to system
cargo install --path .

# Verify
omnisystem --version
```

#### 3. Build from Source (Windows)
```powershell
# Install Rust from https://rustup.rs/

# Build
cargo build --release

# Install
cargo install --path .

# Verify
omnisystem --version
```

### Docker Installation

#### Using Docker Image
```bash
# Pull official image
docker pull omnisystem/omnisystem:latest

# Run container
docker run -it omnisystem/omnisystem:latest

# Or build from Dockerfile
docker build -t omnisystem .
docker run -it omnisystem
```

### Package Manager Installation

#### Homebrew (macOS)
```bash
brew tap omnisystem/omnisystem
brew install omnisystem
```

#### APT (Ubuntu/Debian)
```bash
sudo add-apt-repository ppa:omnisystem/omnisystem
sudo apt-get update
sudo apt-get install omnisystem
```

#### Chocolatey (Windows)
```powershell
choco install omnisystem
```

## Configuration

### Environment Variables

#### Linux/macOS
```bash
# Add to ~/.bashrc or ~/.zshrc
export OMNISYSTEM_HOME="$HOME/.omnisystem"
export PATH="$PATH:$OMNISYSTEM_HOME/bin"
export RUST_LOG=info
```

#### Windows
```powershell
# Set environment variables
[Environment]::SetEnvironmentVariable("OMNISYSTEM_HOME", "$env:USERPROFILE\.omnisystem", "User")
[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:OMNISYSTEM_HOME\bin", "User")
[Environment]::SetEnvironmentVariable("RUST_LOG", "info", "User")
```

### Configuration File

Create `~/.omnisystem/config.toml`:

```toml
[omnisystem]
version = "1.0.0"
log_level = "info"
data_dir = "~/.omnisystem/data"

[titan]
max_file_size_mb = 1000
compression = "gzip"

[sylva]
num_threads = 4
ml_backends = ["sklearn", "pytorch"]

[aether]
port = 8080
discovery_timeout_ms = 5000

[axiom]
smtlib_timeout_ms = 10000
```

## Verification

### Verify Installation
```bash
# Check version
omnisystem --version
# Output: Omnisystem 1.0.0

# Check components
omnisystem --check
# Output:
# ✓ TITAN: OK
# ✓ SYLVA: OK
# ✓ AETHER: OK
# ✓ AXIOM: OK

# Run diagnostics
omnisystem --diagnose
# Runs comprehensive system checks
```

### Run Tests
```bash
# Run test suite
omnisystem test

# Run specific module tests
omnisystem test titan
omnisystem test sylva
omnisystem test aether
omnisystem test axiom

# Run performance benchmarks
omnisystem bench --all
```

## Getting Started

### Hello World (TITAN)

Create `hello.ti`:
```
pub fn main() -> String {
    let greeting = "Hello, Omnisystem!";
    println(greeting);
    greeting
}
```

Run:
```bash
omnisystem run hello.ti
```

### Simple ML Pipeline (SYLVA)

Create `pipeline.ti`:
```
pub fn main() -> String {
    // Load data
    let df = dataframe_from_csv("data.csv");
    
    // Train model
    let features = dataframe_select_columns(df, "feature1,feature2");
    let labels = dataframe_select_columns(df, "target");
    let model = random_forest(features, labels, 50);
    
    // Evaluate
    let predictions = model_predict(model, features);
    let accuracy = accuracy(predictions, labels);
    
    println("Model accuracy: " + float_to_string(accuracy));
    "done"
}
```

Run:
```bash
omnisystem run pipeline.ti
```

## Troubleshooting

### Common Issues

#### "Command not found: omnisystem"
```bash
# Ensure installation path is in PATH
echo $PATH

# If not included, add to shell profile:
export PATH="/usr/local/bin:$PATH"  # Linux/macOS
# or set Windows PATH environment variable

# Verify installation:
which omnisystem  # Linux/macOS
where omnisystem  # Windows PowerShell
```

#### "No module found: SYLVA"
```bash
# Install ML dependencies
omnisystem install-deps sylva

# For manual install:
pip install sklearn pytorch pandas numpy

# Verify
omnisystem check sylva
```

#### "Port already in use" (AETHER services)
```bash
# Change default port in config.toml
[aether]
port = 8081  # Use different port

# Or kill existing process:
lsof -i :8080  # Find process
kill -9 <PID>  # Kill it
```

#### "Out of memory" errors
```bash
# Increase available memory
# Linux/macOS:
ulimit -v 8388608  # 8GB

# Windows: Adjust through System Settings
# Java/Python options in config.toml:
[sylva]
max_memory_mb = 4096
```

### Performance Issues

#### Slow startup
```bash
# Check system resources
omnisystem --diagnose

# Reduce initial load
omnisystem --lazy-load

# Optimize configuration
# Reduce thread count if CPU-bound:
[sylva]
num_threads = 2
```

#### High memory usage
```bash
# Enable aggressive garbage collection
export OMNISYSTEM_GC_THRESHOLD=100

# Reduce cache sizes in config.toml:
[aether]
cache_size_mb = 512

# Monitor memory
omnisystem --monitor
```

## Platform-Specific Notes

### Linux
- Recommended: Ubuntu 20.04 LTS or later, Debian 11+
- Build tools: `sudo apt-get install build-essential cargo`
- Extra dependencies: `sudo apt-get install libssl-dev libffi-dev`

### macOS
- Recommended: macOS 11.0 (Big Sur) or later
- Xcode Command Line Tools: `xcode-select --install`
- Homebrew: `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"`

### Windows
- Recommended: Windows 10 Pro or Windows Server 2019+
- Visual Studio Build Tools required for compilation
- Windows Terminal recommended for better CLI experience
- WSL2 supported for development

## Updates

### Automatic Updates
```bash
# Enable automatic updates (if installed via package manager)
omnisystem auto-update enable

# Check for updates
omnisystem update check

# Apply updates
omnisystem update
```

### Manual Updates
```bash
# Using Git
cd /path/to/omnisystem
git pull origin main
cargo build --release
cargo install --path .

# Or reinstall from package manager
# Homebrew: brew upgrade omnisystem
# APT: sudo apt-get upgrade omnisystem
```

## Uninstallation

### Homebrew (macOS)
```bash
brew uninstall omnisystem
```

### APT (Linux)
```bash
sudo apt-get remove omnisystem
sudo apt-get purge omnisystem  # Remove config files
```

### Chocolatey (Windows)
```powershell
choco uninstall omnisystem
```

### Manual Uninstallation
```bash
# Remove installation
rm -rf $OMNISYSTEM_HOME
cargo uninstall omnisystem

# Remove from PATH
# Edit ~/.bashrc, ~/.zshrc, or Windows environment variables
```

## Getting Help

### Documentation
- **Online Docs**: https://docs.omnisystem.io
- **API Reference**: https://docs.omnisystem.io/api
- **Tutorials**: https://docs.omnisystem.io/tutorials

### Community
- **GitHub Issues**: https://github.com/omnisystem/omnisystem/issues
- **Discussions**: https://github.com/omnisystem/omnisystem/discussions
- **Discord**: https://discord.gg/omnisystem

### Support
- **Email**: support@omnisystem.io
- **Commercial Support**: https://omnisystem.io/support

## Next Steps

1. **Read the Tutorials**: Start with [TUTORIALS.md](docs/TUTORIALS.md)
2. **Explore API Reference**: Check [API_REFERENCE.md](docs/API_REFERENCE.md)
3. **Run Examples**: Try the example files in `examples/`
4. **Join Community**: Connect with other users

## System Health Check

Run comprehensive system check:
```bash
omnisystem health-check

# Expected output:
# ✓ TITAN module: Operational
# ✓ SYLVA module: Operational
# ✓ AETHER module: Operational
# ✓ AXIOM module: Operational
# ✓ Disk space: 450MB available
# ✓ Memory: 6.2GB available
# ✓ Network: Connected
# ✓ All systems: Healthy
```

---

**Installation complete! You're ready to start using Omnisystem.**

