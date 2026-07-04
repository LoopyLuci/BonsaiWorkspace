# Repository Organization

This document describes the structure and organization of the Bonsai Ecosystem repository.

## Root-Level Files

### Essential Documentation
- **README.md** - Main repository overview and quick start
- **GETTING_STARTED.md** - Quick start guide for new developers
- **CONTRIBUTING.md** - Contribution guidelines
- **SECURITY.md** - Security policy and disclosure
- **CHANGELOG.md** - Version history and major changes

### Configuration
- **Cargo.toml** - Rust workspace configuration
- **Cargo.lock** - Rust dependency lock file
- **flake.nix** - Nix flakes configuration for reproducible builds
- **Dockerfile.bmcs** - Docker configuration (legacy)

## Directory Structure

### `/docs/` - Active Documentation
Contains current, maintained documentation for the system:
- Architecture guides
- Implementation documentation
- API references
- Deployment guides
- Active development specifications

**Key files**:
- `COMPLETE_DRIVER_DELIVERY_FINAL.md` - Brother IntelliFAX 2840 MFP driver documentation
- `UNIVERSAL_OS_COMPONENTS_SPECIFICATION.md` - UOSC specification
- `OPENCV5_INTEGRATION_PLAN.md` - Active integration plans

### `/archive/` - Historical Documentation
Contains archived, superseded, or reference documentation:
- **docs/** - Obsolete documentation files (kept for reference)
- **logs/** - Historical build and execution logs
- **results/** - Test result artifacts and benchmarks

Files in archive should not be referenced for current development unless specifically needed for historical context.

### `/config/` - Configuration Files
System-wide configuration files:
- `*.toml` - Build and runtime configuration
- `*.yaml` - Service and language configurations
- `*.json` - Additional configurations

### `/scripts/` - Utility Scripts
Helper scripts for development and deployment:
- Build scripts
- Test runners
- CI/CD utilities
- Development automation

### `/prompts/` - Prompt Files
Claude prompt files for context and configuration.

### `/crates/` - Rust Crates
Monorepo workspace containing all Rust implementation:
- Core system crates
- Language implementation crates
- Tool and utility crates

### `/Omnisystem/` - Universal Operating System Components
Main Universal OS implementation:
- **drivers/** - Universal driver modules (DriverKit, etc.)
- **udc/** - Universal Driver Conversion system
- **languages/** - Omnilang implementations
- **modules/** - System modules

### `/nix/` - Nix Flakes
NixOS integration and reproducible build configurations.

### `/runtime/`, `/runtimes/` - Runtime Systems
Runtime implementations and supporting files.

### `/tests/` - Test Suites
Comprehensive test coverage:
- Unit tests
- Integration tests
- System tests

### `/examples/` - Example Code
Example implementations and use cases.

### `/data/`, `/training-data/` - Training Data
Machine learning datasets and training materials.

### `/deploy/`, `/manifests/` - Deployment
Deployment configurations and manifests.

### `/models/` - Machine Learning Models
Pre-trained models and model artifacts.

### Other Key Directories
- `/android-runtime/` - Android runtime implementation
- `/bonsai-workspace/` - Workspace UI/IDE
- `/browser-extension/` - Browser extension components
- `/polyglot-pong/` - Polyglot language testing framework
- `/vscode-extension/` - VS Code integration
- `/ci/` - CI/CD configuration
- `/mcp/` - Model Context Protocol implementation

## Build Artifacts (Ignored)

The following directories are Git-ignored and should not be committed:
- `/target/` - Rust compilation output
- `*.exe`, `*.dll`, `*.so` - Binary outputs
- `__pycache__/` - Python cache
- `/logs/` (active only) - Current logs

## Quick Navigation

### For New Contributors
1. Start with [GETTING_STARTED.md](GETTING_STARTED.md)
2. Review [CONTRIBUTING.md](CONTRIBUTING.md)
3. Check [README.md](README.md) for architecture overview
4. Browse [/docs/](/docs/) for specific topics

### For Architecture Questions
- See [/docs/](/docs/) for all architecture documentation
- Check individual subsystem README files

### For Building/Development
- Run `cargo build` in root (standard Rust workflow)
- Use scripts in [/scripts/](/scripts/) for specialized builds
- See [/config/](/config/) for configuration options

### For Driver Development
- See [/Omnisystem/drivers/](/Omnisystem/drivers/) for driver implementations
- Review driver specifications in [/Omnisystem/udc/dis/](/Omnisystem/udc/dis/)

## Important Notes

- **Active vs. Archived**: Files in `/docs/` are actively maintained. Files in `/archive/` are for reference only.
- **Git Strategy**: Use `.gitignore` to exclude build artifacts, logs, and cached files.
- **Naming Conventions**: The repository uses "Bonsai Ecosystem" for the system name, with subsystem names (Omnisystem, UBVM, etc.) being independent functional names.

---

**Last Updated**: 2026-06-06  
**Maintained By**: Bonsai Ecosystem Team
