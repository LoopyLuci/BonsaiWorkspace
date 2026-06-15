# Installation & Setup Guide

**Get Omnisystem up and running on your system**

---

## System Requirements

- **OS**: Windows 10+, macOS 10.15+, Linux (Ubuntu 20.04+, Fedora 32+)
- **RAM**: 4GB minimum (8GB recommended)
- **Disk**: 5GB free space
- **Processor**: 64-bit Intel/AMD processor

---

## Installation Methods

### Method 1: Package Manager (Recommended)

#### Windows (Chocolatey)
```bash
choco install omnisystem
```

#### macOS (Homebrew)
```bash
brew install omnisystem
```

#### Linux (Apt)
```bash
sudo apt-add-repository ppa:omnisystem/stable
sudo apt update
sudo apt install omnisystem
```

### Method 2: Download Binaries

1. Visit [omnisystem.io/download](https://omnisystem.io/download)
2. Download for your OS
3. Extract archive
4. Add to PATH
5. Verify installation: `omnisystem --version`

### Method 3: Build from Source

```bash
git clone https://github.com/omnisystem/omnisystem.git
cd omnisystem
cargo build --release
./target/release/omnisystem --version
```

---

## Quick Verification

### Check installation
```bash
omnisystem --version
# omnisystem 2.0.0

omnisystem --help
# Usage: omnisystem <command> [options]
```

### Verify all modules
```bash
omnisystem module list
# Base Modules:
#   ✓ TITAN core
#   ✓ SYLVA core
#   ✓ AETHER core
#   ✓ AXIOM core
```

---

## IDE Setup

### VS Code

1. Install Extension: "Omnisystem" (by Omnisystems Inc)
2. Create `.vscode/settings.json`:
```json
{
  "omnisystem.languageServer": true,
  "omnisystem.autoFormat": true,
  "omnisystem.checkOnSave": true
}
```

3. Restart VS Code
4. Open an `.ti` file to test

### IntelliJ / JetBrains IDEs

1. Go to Settings → Plugins → Marketplace
2. Search "Omnisystem"
3. Install and restart IDE
4. Configure in Settings → Omnisystem
5. Open an `.ti` file to test

### Other Editors

- **Vim**: Use Language Server Protocol (LSP)
- **Emacs**: Use lsp-mode package
- **Sublime**: Install Omnisystem syntax plugin

---

## Configuration

### Global Configuration

Create `~/.omnisystem/config.toml`:
```toml
[language]
default-language = "titan"
formatting = "auto"

[compiler]
optimization-level = 2
parallel-jobs = 4

[repl]
history-size = 1000
theme = "dark"

[modules]
auto-load-base = true
auto-load-universal = false
```

### Project Configuration

Create `omnisystem.toml` in project root:
```toml
[project]
name = "my-project"
version = "0.1.0"
description = "My Omnisystem project"

[dependencies]
aether = "2.0"
sylva = "2.0"

[build]
output-dir = "target"
```

---

## First Run

### 1. Start REPL
```bash
omnisystem repl
omnisystem> 2 + 3
5
omnisystem> :quit
```

### 2. Run Hello World
```bash
omnisystem run --code 'println!("Hello, Omnisystem!")'
# Hello, Omnisystem!
```

### 3. Compile Program
```bash
cat > hello.ti << 'EOF'
fun main() {
    println!("Hello from TITAN!")
}
EOF

omnisystem compile hello.ti
omnisystem run hello.ti
# Hello from TITAN!
```

---

## Troubleshooting Installation

### Command not found
```bash
# Add to PATH
export PATH="$PATH:/path/to/omnisystem/bin"

# Verify
omnisystem --version
```

### Module loading fails
```bash
# Reload modules
omnisystem module reload --force

# Check module status
omnisystem module status
```

### Language Server not working
```bash
# Verify LSP is running
omnisystem lsp --start

# Check logs
omnisystem logs --service lsp
```

### Permission denied on Linux
```bash
# Make executable
chmod +x ~/.omnisystem/omnisystem

# Or reinstall with proper permissions
sudo apt install omnisystem
```

---

## Uninstallation

### Package Manager
```bash
# Windows (Chocolatey)
choco uninstall omnisystem

# macOS (Homebrew)
brew uninstall omnisystem

# Linux (Apt)
sudo apt remove omnisystem
```

### From Source
```bash
# Remove directory
rm -rf ~/omnisystem
# Remove from PATH
# (Edit ~/.bashrc or ~/.zshrc)
```

### Clean Configuration
```bash
# Remove user config
rm -rf ~/.omnisystem
```

---

## Next Steps

1. Run [HELLO_WORLD.md](HELLO_WORLD.md) examples
2. Check [QUICK_REFERENCE.md](QUICK_REFERENCE.md) for syntax
3. Follow [TITAN_LANGUAGE_GUIDE.md](TITAN_LANGUAGE_GUIDE.md) tutorial
4. Build your first project!

---

## Getting Help

- **Docs**: Read documentation in `./docs/`
- **REPL Help**: Type `:help` in REPL
- **Command Help**: `omnisystem <command> --help`
- **Forum**: [omnisystem.io/forum](https://omnisystem.io/forum)
- **Discord**: [discord.gg/omnisystem](https://discord.gg/omnisystem)

---

**Installation Complete!** 🎉

Ready to build with Omnisystem!
