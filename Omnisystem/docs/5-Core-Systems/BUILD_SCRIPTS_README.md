# Build Scripts

Automated build and compilation scripts for BonsaiWorkspace.

## Location

`scripts/build-scripts/`

## Quick Reference

### Full System Build

**Script**: `build-complete-system.ps1`  
**Platform**: Windows (PowerShell)  
**Purpose**: Complete system build from scratch  
**Usage**: `.\scripts\build-scripts\build-complete-system.ps1`

### Windows Setup & Build

**Setup Only**:
- `windows-full-setup.ps1` - Complete Windows environment setup
- `windows-setup-minimal.ps1` - Minimal Windows setup
- `windows-gpu-build.ps1` - Setup for GPU systems

**Usage**:
```powershell
# Full setup
.\scripts\build-scripts\windows-full-setup.ps1

# GPU setup
.\scripts\build-scripts\windows-gpu-build.ps1
```

### Quick Builds

**Fast Builds**:
- `build-and-run.ps1` - Build and immediately launch
- `build-launch.ps1` - Build with launch
- `simple-gpu-build.ps1` - GPU system build

**Usage**:
```powershell
# Build and run
.\scripts\build-scripts\build-and-run.ps1

# GPU system
.\scripts\build-scripts\simple-gpu-build.ps1
```

### Python & Dependencies

**Dependency Management**:
- `fix-python-deps.ps1` - Fix Python dependency conflicts
- `install-python.ps1` - Install Python environment
- `fix-and-train.ps1` - Fix environment and train models

**Usage**:
```powershell
# Fix dependencies
.\scripts\build-scripts\fix-python-deps.ps1

# Setup Python
.\scripts\build-scripts\install-python.ps1

# Train models
.\scripts\build-scripts\fix-and-train.ps1
```

### Optimization & Caching

**Performance**:
- `setup-compilation-cache.ps1` - Configure BACE compilation cache for fast rebuilds
- `watch-build-live.ps1` - Watch build progress in real-time

**Usage**:
```powershell
# Setup caching (first time)
.\scripts\build-scripts\setup-compilation-cache.ps1

# Watch build progress
.\scripts\build-scripts\watch-build-live.ps1
```

### Advanced Orchestration

**Automation**:
- `auto-build-orchestrator.ps1` - Orchestrate complete build pipeline
- `verify_android_bridge_integration.sh` - Verify Android integration

**Usage**:
```powershell
# Full orchestration
.\scripts\build-scripts\auto-build-orchestrator.ps1
```

## Script Categories

### System Setup Scripts
```
windows-full-setup.ps1
windows-setup-minimal.ps1
windows-gpu-build.ps1
install-python.ps1
setup-compilation-cache.ps1
```

### Build Scripts
```
build-complete-system.ps1
build-and-run.ps1
build-launch.ps1
simple-gpu-build.ps1
```

### Maintenance & Repair
```
fix-python-deps.ps1
fix-and-train.ps1
```

### Monitoring & Automation
```
auto-build-orchestrator.ps1
watch-build-live.ps1
verify_android_bridge_integration.sh
```

## Recommended Build Order

### Fresh System Setup
1. `windows-full-setup.ps1` - Initial setup
2. `setup-compilation-cache.ps1` - Configure BACE cache
3. `build-complete-system.ps1` - Full build

### Quick Rebuild
1. `build-and-run.ps1` - Single command

### GPU System
1. `windows-gpu-build.ps1` - GPU-specific setup
2. `simple-gpu-build.ps1` - Build for GPU

### Problem Resolution
1. `fix-python-deps.ps1` - Fix dependency issues
2. `build-and-run.ps1` - Retry build

## Environment Requirements

### Windows
- PowerShell 5.0+
- Administrator rights (for some scripts)
- Rust toolchain
- Python 3.8+

### Dependencies
- Git
- Cargo
- Python packages (installed by scripts)

## Tips for Fast Builds

1. **Enable BACE Cache**
   ```powershell
   .\scripts\build-scripts\setup-compilation-cache.ps1
   ```
   This reduces incremental builds from minutes to seconds.

2. **Use build-and-run.ps1**
   - Fastest combined build+launch
   - Single command, full automation

3. **Watch Progress**
   ```powershell
   .\scripts\build-scripts\watch-build-live.ps1
   ```
   Monitor compilation in real-time

4. **Parallel Builds**
   - Most scripts use parallel compilation
   - Leverages all CPU cores

## Troubleshooting

### Python Dependency Issues
```powershell
.\scripts\build-scripts\fix-python-deps.ps1
```

### Rebuild from Clean State
```powershell
# Remove build artifacts
Remove-Item -Recurse -Force target
# Rebuild
.\scripts\build-scripts\build-complete-system.ps1
```

### GPU Build Issues
```powershell
.\scripts\build-scripts\windows-gpu-build.ps1
```

## Performance Metrics

Typical build times (Ryzen 9 5900X, 24 cores):
- **First build**: 10-15 minutes
- **Incremental (with BACE cache)**: 30 seconds - 2 minutes
- **GPU-enabled build**: 15-20 minutes

## Logging

Build logs are saved to `logs/` directory:
- `build.log` - Main build output
- `build-rust.log` - Rust compiler output
- `build-launch.log` - Launch output

## Script Development

To create new build scripts:
1. Place in `scripts/build-scripts/`
2. Follow PowerShell best practices
3. Add error handling
4. Log output to `logs/`
5. Document in this README

## Safety

All scripts are:
- ✅ Version controlled
- ✅ Backed up in git
- ✅ Non-destructive (unless explicitly stated)
- ✅ Reversible
- ✅ Tested on Windows

## Getting Help

1. Check relevant script comments
2. Review build logs in `logs/`
3. See `docs/guides/BUILD_AND_RUN_GUIDE.md`
4. Check `GETTING_STARTED.md`

---

**Last Updated**: June 3, 2026  
**Status**: ✅ Complete and Organized  
**Total Scripts**: 13 build scripts organized
