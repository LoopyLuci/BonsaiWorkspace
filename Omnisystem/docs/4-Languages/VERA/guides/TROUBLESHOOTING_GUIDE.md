# Troubleshooting Guide - Omnisystem Graphics Build System

**Version**: 2.0.0  
**Date**: 2026-06-24  
**Status**: Production-Ready

---

## Table of Contents

1. [Build Errors](#build-errors)
2. [GPU and Driver Issues](#gpu-and-driver-issues)
3. [Graphics Initialization Failures](#graphics-initialization-failures)
4. [UI Rendering Problems](#ui-rendering-problems)
5. [Performance Problems](#performance-problems)
6. [Memory Issues](#memory-issues)
7. [Thermal and Power Issues](#thermal-and-power-issues)

---

## Build Errors

### Error: "Titan compiler not found"

**Symptoms**:
- Build stops immediately
- Message: "Titan compiler not found at..."
- No object files generated

**Root Causes**:
1. Titan compiler not built
2. Wrong path configuration
3. 32-bit vs 64-bit mismatch
4. Permissions issue

**Solutions**:

```powershell
# Solution 1: Build Titan compiler first
cd Z:\Projects\Omnisystem\Omnisystem\titan_compiler
cargo build --release

# Verify build succeeded
ls target/release/titan.exe

# Solution 2: Manually set path if different
$env:TITAN_COMPILER_PATH = "C:\custom\path\to\titan.exe"
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 3: Run validation script
.\BUILD_VALIDATION_SCRIPT.ps1 -Verbose

# Solution 4: Check file permissions
icacls Z:\Projects\Omnisystem\Omnisystem\titan_compiler\target\release\titan.exe

# Solution 5: Use 64-bit version only
ls Z:\Projects\Omnisystem\Omnisystem\titan_compiler\target\x86_64-pc-windows-msvc\release\
```

---

### Error: "Compilation failed: undefined reference"

**Symptoms**:
```
✗ Compilation failed: Undefined reference to symbol 'GraphicsFramework_Init'
Error Code: 0x00000139
```

**Root Causes**:
1. Missing graphics framework source files
2. Incorrect include paths
3. Source files not compiled before linking
4. Titan compiler cache corruption

**Solutions**:

```powershell
# Solution 1: Verify graphics framework exists
ls Z:\Projects\Omnisystem\Omnisystem\modules\base-modules\frameworks\graphics\

# Solution 2: Check for required files
$requiredFiles = @(
    'HelixGraphicsEngineInit.helix'
    'GraphicsMemoryManager.titan'
    'CommandQueue.titan'
    'GpuDriver*.titan'
)

foreach ($file in $requiredFiles) {
    $found = Test-Path (Join-Path Z:\Projects\Omnisystem\src\graphics $file)
    Write-Host "$file : $(if($found) {'✓'} else {'✗'})"
}

# Solution 3: Clean build cache
Remove-Item Z:\Projects\Omnisystem\Omnisystem\scripts\build\.build-graphics -Recurse -Force
.\Build-Omnisystem-Launcher-Graphics.ps1 -Clean -Verbose

# Solution 4: Rebuild graphics framework
# Run specific framework build if available
cd Z:\Projects\Omnisystem\Omnisystem\modules\base-modules\frameworks\graphics
# Check for build script
ls Build*.ps1
```

---

### Error: "Out of memory during compilation"

**Symptoms**:
- Build process slows dramatically
- Error message about heap or memory
- System becomes unresponsive

**Root Causes**:
1. Insufficient RAM (compiling large files)
2. Memory leak in build process
3. Other heavy applications running
4. Disk space full (using disk as swap)

**Solutions**:

```powershell
# Solution 1: Check available memory
[math]::Round((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1MB)

# Solution 2: Close unnecessary applications
Get-Process | Where-Object WorkingSet64 -gt 500MB | Sort-Object WorkingSet64 -Descending | Head -10

# Solution 3: Reduce parallelization
$env:TITAN_PARALLEL_JOBS = "2"  # Reduce from 4 to 2
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 4: Use GPU target instead of all
.\Build-Omnisystem-Launcher-Graphics.ps1 -GpuTarget nvidia  # Smaller build

# Solution 5: Enable virtual memory (swap)
# Windows Settings > System > Advanced System Settings > Performance > Virtual Memory

# Solution 6: Check disk space
Get-PSDrive -Name Z | Select-Object Name, @{N="Free(GB)";E={[math]::Round($_.Free/1GB)}}
```

---

### Error: "Link error: too many open files"

**Symptoms**:
```
✗ Linker error: Cannot open file. Too many open files.
Error Code: 0x00000104
```

**Root Causes**:
1. Too many intermediate files created
2. File handle limit exceeded
3. Antivirus scanning while building
4. Previous build not cleaned properly

**Solutions**:

```powershell
# Solution 1: Clean and rebuild
.\Build-Omnisystem-Launcher-Graphics.ps1 -Clean

# Solution 2: Increase file handle limit (Windows doesn't have a limit like Unix)
# This is usually a filesystem issue; try solution 1 first

# Solution 3: Disable antivirus temporarily
# Control Panel > Windows Defender > Virus & threat protection
# Real-time protection > Toggle OFF (temporary)

# Solution 4: Check and close file explorer windows
Get-Process explorer | Stop-Process -Force
# Then: Re-open Windows Explorer from Task Manager

# Solution 5: Rebuild with reduced parallelization
$env:TITAN_PARALLEL_JOBS = "1"
.\Build-Omnisystem-Launcher-Graphics.ps1 -Release
```

---

## GPU and Driver Issues

### Issue: "No compatible GPU found"

**Symptoms**:
- Build succeeds but warning about GPU
- Application runs but no GPU acceleration
- GPU detection returns 0 devices

**Root Causes**:
1. GPU not installed or recognized
2. GPU drivers not installed
3. GPU disabled in BIOS
4. GPU not supported by Omnisystem

**Solutions**:

```powershell
# Solution 1: Check Device Manager
Get-CimInstance Win32_VideoController

# Solution 2: Update or install GPU drivers
# NVIDIA
winget install nvidia-gpu-driver

# AMD  
winget install amd-software

# Intel Arc
winget install Intel.ArcControl

# Solution 3: Check BIOS settings
# Restart computer, press DEL/F2 during boot, look for:
# Integrated Graphics > Enabled
# Discrete Graphics > Enabled

# Solution 4: Verify GPU in Device Manager (Windows)
# Device Manager > Display adapters > Right-click GPU > Properties
# Should show: Working properly

# Solution 5: Check PCIe slot and power
# Ensure GPU is fully seated in PCIe slot
# Ensure 6-pin or 8-pin power connectors are connected
```

---

### Issue: "GPU driver version mismatch"

**Symptoms**:
```
! Warning: GPU driver version 450.0 is older than minimum 525.0
Performance may be degraded
```

**Root Causes**:
1. Driver not updated recently
2. Automatic updates disabled
3. Older driver version still installed

**Solutions**:

```powershell
# Solution 1: Update NVIDIA driver
# NVIDIA Control Panel > Help > Updates > Check for Updates
# OR: Download from https://www.nvidia.com/Download/driverDetails.aspx

# Solution 2: Update AMD driver
# AMD Software > Settings > System > Check for Update
# OR: Download from https://support.amd.com/en-us/drivers

# Solution 3: Update Intel driver
# Intel Arc Control > Settings > Check for Updates
# OR: Download from https://ark.intel.com/

# Solution 4: Force driver reinstall
# Device Manager > Display Adapters > Right-click > Uninstall device
# Restart computer (will auto-reinstall)

# Solution 5: Verify driver version
# NVIDIA
nvidia-smi

# AMD
wmic path win32_videocontroller get name, DriverVersion

# Intel
Get-PnpDevice | Where-Object Name -Match "Intel Arc"
```

---

### Issue: "GPU timeout / TDR (Timeout Detection and Recovery)"

**Symptoms**:
```
✗ GPU timeout detected: GPU did not respond within 2 seconds
Error Code: 0x00000103
Application: Window black, then crash
```

**Root Causes**:
1. GPU driver issue or bug
2. GPU overclocked or unstable
3. Infinite loop in shader code
4. GPU memory corruption
5. PCIe connection unstable

**Solutions**:

```powershell
# Solution 1: Reset GPU
nvidia-smi -pm 1 -rs  # NVIDIA only

# Solution 2: Reduce GPU clocks (if overclocked)
nvidia-smi -lgc 1500  # Set to 1500 MHz base clock
nvidia-smi -lmc 7000  # Set memory to 7000 MHz

# Solution 3: Reinstall GPU drivers cleanly
# 1. Download latest driver
# 2. Uninstall current driver in Control Panel
# 3. Download and run DDU (Display Driver Uninstaller)
# 4. Reboot in Safe Mode
# 5. Run DDU, reboot again
# 6. Install fresh driver

# Solution 4: Check PCIe connection
# Reseat GPU in PCIe slot
# Reseat power connectors
# Try different PCIe slot if available
# Use different power cables if modular PSU

# Solution 5: Check for GPU firmware updates
# Manufacturer's website > GPU model > Firmware/BIOS downloads
# Update if available

# Solution 6: Disable hardware acceleration temporarily
$env:GRAPHICS_FORCE_CPU = "true"
.\Build-Omnisystem-Launcher-Graphics.ps1 -Release
```

---

## Graphics Initialization Failures

### Error: "Failed to create GPU context"

**Symptoms**:
```
✗ Graphics initialization failed: Unable to create GPU context
Error Code: 0x80070003 (FILE_NOT_FOUND)
```

**Root Causes**:
1. Missing graphics libraries (DirectX, Vulkan)
2. GPU driver files corrupted
3. Incompatible GPU + driver combination
4. Graphics subsystem not initialized

**Solutions**:

```powershell
# Solution 1: Install/Update DirectX
# Download from: https://www.microsoft.com/en-us/download/details.aspx?id=35
# Run: dxwebsetup.exe

# Solution 2: Install Visual C++ Runtime
winget install "Microsoft Visual C++ 2022 X64 Minimum Runtime"

# Solution 3: Verify Windows SDK installed
# Windows Settings > Apps > Apps & features > Optional features
# Add Windows SDK if missing

# Solution 4: Check graphics library paths
ls "C:\Windows\System32\d3d*.dll"      # DirectX DLLs
ls "C:\Windows\System32\dxgi*.dll"     # DXGI DLLs
ls "C:\Windows\System32\vulkan*.dll"   # Vulkan DLLs

# Solution 5: Repair Windows system files
sfc /scannow  # System File Checker
# Requires admin and reboot

# Solution 6: Reinstall GPU driver
# See: "GPU driver version mismatch" solutions
```

---

### Error: "Shader compilation failed"

**Symptoms**:
```
✗ Shader compilation error:
  File: shaders/ui_render.glsl
  Line: 45
  Error: Undefined variable 'color'
```

**Root Causes**:
1. Invalid shader syntax
2. Missing shader headers/includes
3. Unsupported GPU features
4. Typo in variable names

**Solutions**:

```powershell
# Solution 1: Check shader files exist
ls Z:\Projects\Omnisystem\src\graphics\shaders\

# Solution 2: Validate shader syntax
# Use GPU vendor's shader compiler:

# NVIDIA GLSL
glslangValidator.exe shader.glsl

# AMD GLSL
glxinfo | grep "OpenGL version"

# Solution 3: Check for missing includes
Get-Content Z:\Projects\Omnisystem\src\graphics\shaders\ui_render.glsl | 
    Select-String "#include|#version"

# Solution 4: Use software fallback
$env:GRAPHICS_FORCE_CPU = "true"

# Solution 5: Request feature fallback
$env:GRAPHICS_MIN_FEATURE_LEVEL = "10"  # Use older GL version
```

---

## UI Rendering Problems

### Problem: "UI elements not visible"

**Symptoms**:
- Application window opens but blank/black
- UI components don't render
- Previous version worked fine

**Root Causes**:
1. GPU memory allocation failed
2. Render target cleared but nothing drawn
3. Viewport/scissor misconfigured
4. Shader compilation failed silently

**Solutions**:

```powershell
# Solution 1: Check GPU memory
nvidia-smi  # See Memory-Usage

# Solution 2: Rebuild with debug logging
$env:GRAPHICS_DEBUG = "true"
.\Build-Omnisystem-Launcher-Graphics.ps1 -Verbose

# Check output for:
# - "Render target created"
# - "Viewport set"
# - "Shader bound"

# Solution 3: Force software rendering
$env:GRAPHICS_FORCE_CPU = "true"
.\Build-Omnisystem-Launcher-Graphics.ps1

# If UI appears: GPU driver issue
# If UI still missing: Rendering code issue

# Solution 4: Check color format
# Verify clear color not black and alpha not 0
# Render target should be RGBA8 or equivalent

# Solution 5: Verify window creation
# Window should be created before GPU context
# Check for window messages in debug log
```

---

### Problem: "UI rendering very slow / low FPS"

**Symptoms**:
- UI responsive but renders at <30 FPS
- Jerky animations
- Scrolling is laggy

**Root Causes**:
1. Software rendering instead of GPU
2. GPU not being utilized
3. Memory bandwidth bottleneck
4. Thermal throttling

**Solutions**:

```powershell
# Solution 1: Verify GPU is being used
$env:GRAPHICS_DEBUG = "true"
# Check log for: "Using GPU: [vendor name]"

# Solution 2: Check if thermal throttling
nvidia-smi | grep "Temperature"
# Should be <80°C, throttling starts at 83°C

# If throttling: Improve cooling
# - Clean GPU heatsink and fans
# - Improve case airflow
# - Reduce ambient temperature

# Solution 3: Reduce batch size or complexity
$env:MAX_RENDER_CALLS_PER_FRAME = "1000"  # Reduce from default
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 4: Lower resolution for testing
# Reduce window size or use lower internal rendering resolution

# Solution 5: Profile to find bottleneck
$env:PROFILE_GPU = "true"
# This generates per-frame metrics

# Solution 6: Update GPU drivers
# See: "GPU driver version mismatch" solutions
```

---

## Performance Problems

### Problem: "Build takes >5 minutes"

**Symptoms**:
- Build time: 5+ minutes even on fast machine
- Previous builds were faster
- Incremental builds not faster than clean

**Root Causes**:
1. Disk I/O bottleneck (HDD vs SSD)
2. Parallelization not working
3. Build cache disabled or corrupted
4. System resource contention

**Solutions**:

```powershell
# Solution 1: Use SSD
# Move project to SSD if currently on HDD
# Example: Move from D:\ (HDD) to C:\ (SSD)

# Solution 2: Enable parallelization
$cores = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
$env:TITAN_PARALLEL_JOBS = $cores - 1
.\Build-Omnisystem-Launcher-Graphics.ps1 -Verbose

# Verify in build log:
# "Parallel jobs: 7" (or your core count - 1)

# Solution 3: Check for incremental caching
# Should see: "Cache hit: X files skipped"

# Solution 4: Monitor disk I/O
Get-Counter '\PhysicalDisk(_Total)\% Disk Time'
# If >80%: Disk is bottleneck

# Solution 5: Reduce GPU targets
.\Build-Omnisystem-Launcher-Graphics.ps1 -GpuTarget nvidia
# Smaller executable = faster

# Solution 6: Disable verification
.\Build-Omnisystem-Launcher-Graphics.ps1 -SkipVerify
# Post-build verification adds ~30 seconds
```

---

## Memory Issues

### Error: "Insufficient GPU memory"

**Symptoms**:
```
✗ GPU memory allocation failed: Out of memory
  Requested: 512 MB
  Available: 128 MB
```

**Root Causes**:
1. GPU has very little VRAM (mobile device)
2. Other applications using GPU memory
3. Memory fragmentation
4. Leak in graphics code

**Solutions**:

```powershell
# Solution 1: Check GPU memory
nvidia-smi
# Look for "FB Memory Usage" line

# Solution 2: Close other GPU-intensive apps
# Close: Games, video editors, renderers
# These compete for GPU memory

# Solution 3: Rebuild with lower resolution target
$env:RENDER_TARGET_WIDTH = "1280"
$env:RENDER_TARGET_HEIGHT = "720"
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 4: Use texture compression
$env:TEXTURE_COMPRESSION = "astc"  # Or 'bc1', 'etc2'
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 5: Rebuild for lower GPU
$env:GRAPHICS_MIN_VRAM = "1024"  # 1 GB minimum
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 6: Enable memory profiling
$env:MEMORY_DEBUG = "true"
# Will report allocations and frees
```

---

## Thermal and Power Issues

### Problem: "GPU thermal throttling detected"

**Symptoms**:
- Warning in build log: "Thermal throttling detected"
- FPS drops from 60 to 30 after 30 seconds
- GPU clocks reduced

**Root Causes**:
1. Poor case airflow
2. Dusty heatsink
3. Ambient temperature too high
4. Thermal paste degraded
5. GPU power limit too high

**Solutions**:

```powershell
# Solution 1: Check temperature
nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits

# Solution 2: Improve cooling
# - Clean GPU: Use compressed air on heatsink
# - Case airflow: Add intake/exhaust fans
# - Ambient: Lower room temperature
# - Paste: Replace thermal paste if very old

# Solution 3: Reduce power limit
nvidia-smi -pm 1 -pl 300  # Reduce from 450W to 300W
# This will reduce peak clock but improve thermal

# Solution 4: Reduce clock speed
nvidia-smi -lgc 1800  # Base clock: 1800 MHz (reduce from 2500)
.\Build-Omnisystem-Launcher-Graphics.ps1

# Solution 5: Monitor clocks during build
# Open Task Manager: GPU tab
# Watch "GPU Clock" column
# Should stay at base clock, not reduced

# Solution 6: Thermal paste replacement (advanced)
# This requires opening the GPU (voiding warranty)
# Only attempt if experienced with hardware
```

---

## Where to Get Help

### Resources

1. **Documentation**
   - [BUILD_GUIDE.md](BUILD_GUIDE.md) - Complete build instructions
   - [GRAPHICS_APPLICATION_ARCHITECTURE.md](GRAPHICS_APPLICATION_ARCHITECTURE.md) - System design
   - [GPU_DRIVER_INTEGRATION.md](GPU_DRIVER_INTEGRATION.md) - GPU drivers

2. **Support Channels**
   - GitHub Issues: https://github.com/omnisystem/omnisystem/issues
   - Discord: https://discord.gg/omnisystem
   - Email: support@omnisystem.dev

3. **External Resources**
   - NVIDIA Driver Issues: https://nvidia.custhelp.com/
   - AMD Driver Issues: https://support.amd.com/
   - Intel Arc Issues: https://www.intel.com/content/www/us/en/support.html

---

**Document Version**: 2.0.0  
**Last Updated**: 2026-06-24  
**Status**: Production-Ready
