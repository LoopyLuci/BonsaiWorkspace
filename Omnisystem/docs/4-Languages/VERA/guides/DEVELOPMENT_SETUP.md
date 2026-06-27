# Development Setup Guide - Omnisystem Graphics

**Version**: 2.0.0  
**Date**: 2026-06-24  
**Status**: Production-Ready

---

## Table of Contents

1. [IDE Setup](#ide-setup)
2. [Debugging Configuration](#debugging-configuration)
3. [Hot Reload Setup](#hot-reload-setup)
4. [Performance Profiling Tools](#performance-profiling-tools)
5. [Graphics Debugging Tools](#graphics-debugging-tools)
6. [Version Control Setup](#version-control-setup)

---

## IDE Setup

### Visual Studio Code (Recommended)

#### Installation

```powershell
# Install VS Code
winget install Microsoft.VisualStudioCode

# Install extensions for Omnisystem
code --install-extension ms-vscode.powershell         # PowerShell support
code --install-extension ms-vscode.cpptools           # C++ (for Rust interop)
code --install-extension rust-lang.rust-analyzer      # Rust support
code --install-extension khaled.inspector-bom         # YAML inspection
code --install-extension ms-vscode.makefile-tools     # Build tools
code --install-extension eamodio.gitlens              # Git integration
code --install-extension ms-vscode.live-server        # Live Server
```

#### Workspace Configuration

**File**: `.vscode/settings.json`

```json
{
  "[powershell]": {
    "editor.defaultFormatter": "ms-vscode.powershell",
    "editor.formatOnSave": true,
    "editor.tabSize": 4
  },
  "[rust]": {
    "editor.defaultFormatter": "rust-analyzer",
    "editor.formatOnSave": true,
    "editor.tabSize": 2
  },
  "powershell.codeFormatting.autoCorrectAliases": true,
  "powershell.codeFormatting.Preset": "OTBS",
  "rust-analyzer.checkOnSave.command": "clippy",
  "files.exclude": {
    "**/obj": true,
    "**/bin": true,
    "**/.build*": true,
    "**/.test-logs": true,
    "**/.benchmark-logs": true
  },
  "search.exclude": {
    "**/.build*": true,
    "**/.test-logs": true,
    "**/target": true
  },
  "editor.excludePatterns": [
    "**/.build*",
    "**/target"
  ]
}
```

**File**: `.vscode/launch.json`

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Build Graphics",
      "type": "PowerShell",
      "request": "launch",
      "script": "${workspaceFolder}/Omnisystem/scripts/build/Build-Omnisystem-Launcher-Graphics.ps1",
      "args": [ "-Verbose", "-Release" ],
      "cwd": "${workspaceFolder}/Omnisystem/scripts/build"
    },
    {
      "name": "Test Graphics",
      "type": "PowerShell",
      "request": "launch",
      "script": "${workspaceFolder}/Omnisystem/scripts/build/TEST_GRAPHICS_APPLICATION.ps1",
      "args": [ "-Full", "-Verbose" ],
      "cwd": "${workspaceFolder}/Omnisystem/scripts/build"
    },
    {
      "name": "Run Application",
      "type": "PowerShell",
      "request": "launch",
      "script": "&  '${workspaceFolder}/Omnisystem/launchers/Omnisystem_Graphics.exe'",
      "cwd": "${workspaceFolder}/Omnisystem/launchers"
    },
    {
      "name": "Build & Run",
      "type": "PowerShell",
      "request": "launch",
      "script": "${workspaceFolder}/Omnisystem/scripts/build/Build-Omnisystem-Launcher-Graphics.ps1",
      "args": [ "-Launch", "-Verbose" ],
      "cwd": "${workspaceFolder}/Omnisystem/scripts/build"
    }
  ]
}
```

#### Build Tasks

**File**: `.vscode/tasks.json`

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Build Graphics (Release)",
      "type": "shell",
      "command": "powershell",
      "args": [
        "-File",
        "${workspaceFolder}/Omnisystem/scripts/build/Build-Omnisystem-Launcher-Graphics.ps1",
        "-Release",
        "-Verbose"
      ],
      "group": {
        "kind": "build",
        "isDefault": true
      },
      "presentation": {
        "reveal": "always",
        "panel": "new"
      },
      "problemMatcher": []
    },
    {
      "label": "Build Graphics (Debug)",
      "type": "shell",
      "command": "powershell",
      "args": [
        "-File",
        "${workspaceFolder}/Omnisystem/scripts/build/Build-Omnisystem-Launcher-Graphics.ps1",
        "-Verbose"
      ],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Clean Build",
      "type": "shell",
      "command": "powershell",
      "args": [
        "-File",
        "${workspaceFolder}/Omnisystem/scripts/build/Build-Omnisystem-Launcher-Graphics.ps1",
        "-Clean",
        "-Verbose"
      ]
    },
    {
      "label": "Run Tests",
      "type": "shell",
      "command": "powershell",
      "args": [
        "-File",
        "${workspaceFolder}/Omnisystem/scripts/build/TEST_GRAPHICS_APPLICATION.ps1",
        "-Full",
        "-Verbose"
      ],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Run Benchmarks",
      "type": "shell",
      "command": "powershell",
      "args": [
        "-File",
        "${workspaceFolder}/Omnisystem/scripts/build/PERFORMANCE_BENCHMARK.ps1",
        "-Full",
        "-Iterations",
        "3"
      ]
    }
  ]
}
```

#### Keyboard Shortcuts

**File**: `.vscode/keybindings.json`

```json
[
  {
    "key": "ctrl+shift+b",
    "command": "workbench.action.tasks.runTask",
    "args": "Build Graphics (Release)"
  },
  {
    "key": "ctrl+shift+t",
    "command": "workbench.action.tasks.runTask",
    "args": "Run Tests"
  },
  {
    "key": "ctrl+shift+p",
    "command": "workbench.action.tasks.runTask",
    "args": "Run Benchmarks"
  },
  {
    "key": "f5",
    "command": "workbench.action.debug.start"
  },
  {
    "key": "ctrl+alt+r",
    "command": "workbench.action.tasks.runTask",
    "args": "Build & Run"
  }
]
```

### JetBrains CLion

#### Project Configuration

1. **File > Settings > Languages & Frameworks > PowerShell**
   - Enable PowerShell support
   - Set interpreter: `pwsh.exe`

2. **File > Settings > Build, Execution, Deployment > Compiler**
   - Add external build tool:
     - Program: `powershell.exe`
     - Arguments: `-File Build-Omnisystem-Launcher-Graphics.ps1 -Verbose`
     - Working directory: `$ProjectFileDir$/Omnisystem/scripts/build`

3. **Run > Edit Configurations**
   - Create new "PowerShell" configuration
   - Script path: Build script
   - Working directory: Build scripts directory

#### Code Inspection

**Settings > Editor > Inspections**

Enable:
- PowerShell: Syntax errors
- PowerShell: Undefined variables
- Duplicated code fragments
- Suspicious variable names

---

## Debugging Configuration

### Debug Build

```powershell
# Build with debug symbols and logging
.\Build-Omnisystem-Launcher-Graphics.ps1 -Verbose

# Enable debug logging in environment
$env:GRAPHICS_DEBUG = "true"
$env:GRAPHICS_LOG_LEVEL = "DEBUG"

# Run application with debug output
Z:\Projects\Omnisystem\Omnisystem\launchers\Omnisystem_Graphics.exe
```

### Debug Output

The debug build creates:
```
Z:\Projects\Omnisystem\Omnisystem\scripts\build\.build-graphics\
├── build.log              # Complete build log
├── graphics_debug.log     # Runtime debug output
└── gpu_driver.log         # GPU driver operations
```

### WinDbg Debugging

```powershell
# Install WinDbg (Windows 11 / Windows Server 2022)
winget install Microsoft.WinDbg

# Open executable in WinDbg
windbg Z:\Projects\Omnisystem\Omnisystem\launchers\Omnisystem_Graphics.exe

# Useful commands:
# .load sos                 # Load SOS extension
# g                         # Go (run)
# bp graphicsInit           # Breakpoint at function
# !clrstack                 # Show managed stack
# ~*k                       # Stack trace all threads
# dd esp L10                # Dump memory at esp
```

### Performance Profiling with Windows Performance Analyzer

```powershell
# Install Windows Performance Toolkit (part of Windows SDK)
# https://docs.microsoft.com/en-us/windows-hardware/test/wpt/

# Capture trace while running
wpr -start GenericProfile -start DiskIO -start Networking
Z:\Projects\Omnisystem\Omnisystem\launchers\Omnisystem_Graphics.exe
# Let app run for 30 seconds
# Ctrl+C to close application
wpr -stop output.etl

# Analyze trace
wpa output.etl
```

---

## Hot Reload Setup

### Automatic Build on File Change

**Script**: `Z:\Projects\Omnisystem\watch-and-build.ps1`

```powershell
param(
    [string]$SourcePath = "Z:\Projects\Omnisystem\src\graphics",
    [string]$BuildScript = "Z:\Projects\Omnisystem\Omnisystem\scripts\build\Build-Omnisystem-Launcher-Graphics.ps1",
    [int]$DebounceMs = 1000
)

$lastBuildTime = Get-Date

# Monitor source files
$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = $SourcePath
$watcher.IncludeSubdirectories = $true
$watcher.Filter = "*.ti"

Write-Host "Watching $SourcePath for changes..." -ForegroundColor Yellow

Register-ObjectEvent -InputObject $watcher -EventName "Changed" -Action {
    $timeSinceLastBuild = (Get-Date) - $lastBuildTime
    
    if ($timeSinceLastBuild.TotalMilliseconds -gt $DebounceMs) {
        Write-Host "Files changed, rebuilding..." -ForegroundColor Yellow
        
        & $BuildScript -Verbose
        
        $lastBuildTime = Get-Date
        Write-Host "Build complete" -ForegroundColor Green
    }
} | Out-Null

# Keep script running
while ($true) { Start-Sleep -Seconds 1 }
```

**Usage**:

```powershell
# Terminal 1: Run watcher
.\watch-and-build.ps1

# Terminal 2: Edit files and save
# Build triggers automatically when files change
```

---

## Performance Profiling Tools

### Intel VTune

```powershell
# Install VTune
# https://www.intel.com/content/www/us/en/develop/documentation/vtune-help/top.html

# Profile graphics application
vtune -collect hotspots -app-working-dir Z:\Projects\Omnisystem\Omnisystem\launchers -app-working-dir . `
    Z:\Projects\Omnisystem\Omnisystem\launchers\Omnisystem_Graphics.exe

# Analyze results
# Open in VTune Profiler GUI
```

### NVIDIA Nsight

```powershell
# Install NVIDIA Nsight (bundled with NVIDIA driver)
# C:\Program Files\NVIDIA GPU Computing Toolkit\Nsight

# Profile graphics on NVIDIA GPU
# Launch: "NVIDIA Nsight > Profiler"
# Target: Omnisystem_Graphics.exe
# Metrics: GPU Utilization, Memory Bandwidth, SM Efficiency
```

### AMD GPU Profiler

```powershell
# Install AMD GPU Profiler
# https://github.com/GPUOpen-Tools/radeon_gpu_profiler

# Profile on AMD GPU
gpuprofiler.exe -a Omnisystem_Graphics.exe

# Analyze .rgp file
# Open in GPU Profiler GUI
```

---

## Graphics Debugging Tools

### RenderDoc

**Installation and Setup**:

```powershell
# Install RenderDoc
winget install RenderDocWg1 --source winget

# Launch and capture graphics
# 1. Launch: RenderDoc.exe
# 2. Target executable: Omnisystem_Graphics.exe
# 3. Capture: Press F12 in application
# 4. Analyze: Frame debugger, texture inspect, shader editor
```

**Common Analysis Tasks**:

```
1. Check draw calls
   • Frame debugger > Draw calls list
   • Verify correct order and batching

2. Inspect GPU resources
   • Textures > View all textures
   • Check formats, sizes, memory usage

3. Debug shaders
   • Draw call > Edit shader
   • Modify and recompile inline

4. Memory analysis
   • Buffers > Check allocations
   • Look for unused resources

5. Performance analysis
   • Timing > GPU time per draw call
   • Identify bottlenecks
```

### GPU-Z

```powershell
# Install GPU-Z (lightweight monitoring)
winget install TechPowerUp.GPU-Z

# Use to monitor:
# • GPU core clock
# • Memory utilization
# • Temperature
# • Power consumption
# • Memory bandwidth utilization
```

### GFXBench

```powershell
# Install GFXBench (graphics benchmark)
# https://www.gfxbench.com/

# Run to compare performance:
# • GPU model comparison
# • Driver version impact
# • Thermal throttling detection
```

---

## Version Control Setup

### Git Configuration

```powershell
# Configure Git for Omnisystem development
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"

# Set up hooks for automatic validation
# Pre-commit hook: Validate PowerShell syntax
# Pre-push hook: Run tests before pushing
```

### Pre-commit Hook

**File**: `.git/hooks/pre-commit`

```powershell
#!/usr/bin/env pwsh

# Validate PowerShell scripts before commit
$errors = 0

Get-ChildItem -Path "Omnisystem/scripts/build" -Filter "*.ps1" | ForEach-Object {
    Write-Host "Checking $($_.Name)..."
    
    $file = Get-Content $_.FullName
    $tokens = $null
    
    [System.Management.Automation.PSParser]::Tokenize($file, [ref]$tokens) | 
        Where-Object { $_.Type -eq 'Error' } | 
        ForEach-Object {
            Write-Host "  Syntax error at line $($_.StartLine): $($_.Content)"
            $errors++
        }
}

if ($errors -gt 0) {
    Write-Host "❌ Commit blocked: $errors syntax error(s) found"
    exit 1
}

Write-Host "✓ All scripts validated"
exit 0
```

### GitHub Branch Protection

**Settings > Branches > Branch Protection Rules**:

```
Branch name pattern: main
  ✓ Require a pull request before merging
  ✓ Require approvals (1)
  ✓ Require status checks to pass
    - Build Graphics
    - Test Graphics
    - Run Benchmarks
  ✓ Require branches to be up to date before merging
  ✓ Require conversation resolution before merging
```

---

**Document Version**: 2.0.0  
**Last Updated**: 2026-06-24  
**Status**: Production-Ready
