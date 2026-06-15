# Pre-Launcher - Automatic Dependency Installation

**Repository**: Omnisystem  
**Team**: Omnisystem Team  
**Version**: 1.0.0  
**Last Updated**: 2026-06-12  
**Status**: ✅ Production Ready  

---

## Overview

The Pre-Launcher is a zero-configuration bootstrap system that automatically handles all dependency installation and environment setup. Users don't need to manually install anything—the Pre-Launcher handles it all.

## 🚀 What It Does

The Pre-Launcher automatically:

1. ✅ **Detects System** — Identifies OS, architecture, and installed tools
2. ✅ **Checks Dependencies** — Verifies all required tools are installed
3. ✅ **Installs Missing** — Automatically downloads and installs missing dependencies
4. ✅ **Configures Environment** — Sets up environment variables and paths
5. ✅ **Verifies Setup** — Confirms everything is ready to use
6. ✅ **Reports Status** — Shows what was installed and current versions

## 📋 Managed Dependencies

The Pre-Launcher automatically handles:

### Core Requirements
- **Rust 1.70+** — Compiler and runtime
- **Cargo** — Rust package manager (installed with Rust)
- **Node.js 18+** — JavaScript runtime
- **npm 9+** — Node package manager (installed with Node.js)
- **Git 2.30+** — Version control

### Environment Setup
- Rust toolchain components (rustfmt, clippy, rust-analyzer)
- Build cache configuration (sccache)
- Parallel job settings
- Development environment variables
- Tauri configuration

### Per-Platform Installation

**Windows:**
- Uses Chocolatey (if available) or direct installers
- Configures PowerShell execution policy
- Sets up Visual C++ build tools

**macOS:**
- Uses Homebrew for package management
- Installs developer tools
- Configures code signing (if needed)

**Linux:**
- Uses apt-get (Ubuntu/Debian)
- Uses dnf (Fedora/RHEL)
- Installs build essentials

## 💻 Usage

### Automatic Installation (Recommended)

Just run the pre-launcher:

```bash
# Windows
.\target\release\pre-launcher.exe

# macOS/Linux
./target/release/pre-launcher
```

That's it! It will:
1. Check what's missing
2. Install everything automatically
3. Configure the environment
4. Report success

### Check Only (No Installation)

If you just want to verify your system:

```bash
# Windows
.\target\release\pre-launcher.exe --check

# macOS/Linux
./target/release/pre-launcher --check
```

This will show:
- Current system information
- Installed tools and versions
- Any missing dependencies
- Recommendations

### Verbose Output

For detailed debugging information:

```bash
.\target\release\pre-launcher.exe --verbose
```

## 📊 Output Example

```
╔══════════════════════════════════════════════╗
║   OMNISYSTEM PRE-LAUNCHER BOOTSTRAP         ║
╚══════════════════════════════════════════════╝

🔍 Detecting system information...

📋 System Information:

  OS: Windows
  Architecture: x64
  Rust: rustc 1.70.0
  Cargo: cargo 1.70.0
  Node.js: 18.16.1
  npm: 9.6.7
  Git: 2.40.1

📦 Checking dependencies...
✅ All dependencies satisfied

📊 Dependency Summary:
  ✓ Rust Compiler v1.70.0
  ✓ Cargo v1.70.0
  ✓ Node.js v18.16.1
  ✓ npm v9.6.7
  ✓ Git v2.40.1

⚙️  Setting up environment variables...
✓ Environment configured successfully

🔐 Verifying installation...
✓ Rust Compiler is installed
✓ Cargo (Rust Package Manager) is installed
✓ Node.js is installed
✓ npm (Node Package Manager) is installed
✓ Git is installed
✓ All dependencies verified

╔══════════════════════════════════════════════╗
║   ✅ BOOTSTRAP COMPLETE                      ║
╚══════════════════════════════════════════════╝

📊 Summary:
  Dependencies Installed: 0
  Services Started: 3
  Duration: 250ms

✅ Pre-launcher bootstrap successful!
System: Windows x64 (rustc 1.70.0)
Startup Time: 250ms
Services Started: session-manager, app-registry, launch-coordinator

🚀 You're ready to use Omnisystem!
```

## 🔧 Building the Pre-Launcher

### Build Release Binary

```bash
# Build optimized binary
cd Omnisystem/crates/pre-launcher
cargo build --release --bin pre-launcher

# Output: target/release/pre-launcher.exe (Windows)
# Output: target/release/pre-launcher (macOS/Linux)
```

### Run Tests

```bash
cargo test
```

## 📦 Integration Points

The Pre-Launcher integrates with:

1. **Launcher System** — Prepares environment before launcher starts
2. **Build System** — Ensures cargo can build everything
3. **Development Tools** — Configures IDE integration
4. **CI/CD Pipeline** — Sets up build agents

## 🔐 Safety Features

- ✅ Verifies all installations after setup
- ✅ Provides detailed error messages
- ✅ Preserves existing configurations
- ✅ Supports offline mode (check only)
- ✅ Can be run multiple times safely

## 🌍 Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Windows 10/11 | ✅ Full | Chocolatey or direct install |
| macOS 10.13+ | ✅ Full | Homebrew required |
| Linux (Ubuntu/Debian) | ✅ Full | apt-get required |
| Linux (Fedora/RHEL) | ✅ Full | dnf required |

## 🚨 Troubleshooting

### "Chocolatey not found" (Windows)

Install Chocolatey:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
iwr https://community.chocolatey.org/install.ps1 -UseBasicParsing | iex
```

Then run pre-launcher again.

### "Homebrew not found" (macOS)

Install Homebrew:
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then run pre-launcher again.

### "sudo required" (Linux)

The pre-launcher may need sudo for apt-get:
```bash
sudo ./target/release/pre-launcher
```

### Installation Still Fails

Check logs in verbose mode:
```bash
./pre-launcher.exe --verbose
```

Common causes:
- No internet connection
- Disk space full
- User permissions
- Conflicting software

## 📚 Modules

### `dependencies.rs`
- `DependencyManager` — Manages dependency detection and installation
- `DependencyStatus` — Tracks if tools are installed/missing/outdated
- Detector functions for each tool
- Installer functions for each platform

### `environment.rs`
- `EnvironmentSetup` — Configures all environment variables
- `EnvironmentInfo` — Detects system information
- Rust environment configuration
- Node.js environment configuration
- Build cache setup

### `bootstrap.rs`
- `Bootstrap::run()` — Full bootstrap with automatic installation
- `Bootstrap::check()` — Check-only mode
- Status reporting

### `bin/pre-launcher.rs`
- Executable entry point
- Command-line argument parsing
- Result reporting to user

## 🔄 Update Process

The Pre-Launcher can be run multiple times:

```bash
# First run: Full installation
.\pre-launcher.exe

# Later runs: Verification
.\pre-launcher.exe
# Runs quickly, shows current status
```

This makes it safe to re-run as part of:
- Docker build processes
- CI/CD pipelines
- Development setups
- Fresh installations

## 🎯 Design Goals

1. **Zero Configuration** — Works out of the box
2. **Automatic** — No user prompts or manual steps
3. **Smart** — Only installs what's missing
4. **Fast** — Minimal overhead
5. **Cross-Platform** — Same workflow on all OS
6. **Safe** — Can be run multiple times
7. **Transparent** — Shows what it's doing
8. **Reliable** — Verifies all installations

## 📈 Statistics

| Metric | Value |
|--------|-------|
| Dependencies Managed | 5 |
| Platforms Supported | 3+ |
| Installation Time | 2-5 minutes |
| Verification Time | <1 second |
| Binary Size | ~10 MB |
| Lines of Code | 800+ |
| Test Coverage | 10+ tests |

## 🚀 Next Steps

1. **Build**: `cargo build --release --bin pre-launcher`
2. **Run**: `./target/release/pre-launcher`
3. **Verify**: Check output shows ✅ success
4. **Use**: Ready to build and use Omnisystem!

## 📄 Related Files

- `src/dependencies.rs` — Dependency management
- `src/environment.rs` — Environment setup
- `src/bootstrap.rs` — Bootstrap orchestration
- `src/bin/pre-launcher.rs` — CLI entry point
- `Cargo.toml` — Dependencies

## ✅ Status

**PRODUCTION READY**

The Pre-Launcher is fully functional and tested. Users can run it once and have everything they need to use Omnisystem without any additional setup.

---

**Last Updated**: 2026-06-12  
**Status**: ✅ Production Ready
