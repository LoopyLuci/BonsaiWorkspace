# Build System Developer Guide

Complete guide for developers working on, improving, and extending the Omnisystem build system.

---

## Quick Reference

### Most Common Tasks

| Task | Command | File |
|------|---------|------|
| Build all | `.\Quick-Build.ps1` | - |
| Build desktop | `.\Quick-Build.ps1 -Target desktop` | - |
| Debug build | `.\Build-Launchers.ps1 -Target desktop` | Build-Launchers.ps1 |
| Check setup | `.\Verify-Build-Setup.ps1` | Verify-Build-Setup.ps1 |
| View logs | `Get-Content .\build\build.log` | - |
| Add project | Edit Build-Launchers.ps1 | Build-Launchers.ps1 |

---

## File Organization

### Core Build Files

```
Z:\Projects\Omnisystem\
├── Quick-Build.ps1              [USER INTERFACE LAYER]
│   └─ Simple wrapper
│   └─ Parameter handling
│   └─ Calls Build-Launchers.ps1
│
├── Build-Launchers.ps1          [ORCHESTRATION LAYER]
│   └─ Main build logic
│   └─ Project definitions
│   └─ Helper functions
│
├── Build-Omnisystem.ps1         [STANDARD BUILD]
│   └─ Legacy / alternative
│
└── Verify-Build-Setup.ps1       [VALIDATION]
    └─ System checks
    └─ Dependency validation
```

### Project Locations

```
Z:\Projects\Omnisystem\Omnisystem\
├── applications/bonsai-desktop-environment/    [PROJECT 1]
│   ├── Cargo.toml
│   ├── src/launcher/main.rs
│   └── src/launcher/Omnisystem.rs
│
├── src/crates/omnisystem-launcher-gui/         [PROJECT 2]
│   └── src-tauri/
│       ├── Cargo.toml
│       └── src/main.rs
│
└── modules/base-modules/applications/.../      [PROJECT 3]
    └── launcher/
        ├── Cargo.toml
        └── src-tauri/src/main.rs
```

---

## Understanding the Code

### Build-Launchers.ps1 Structure

#### 1. Parameter Definition (Lines 1-20)

```powershell
param(
    [ValidateSet("all", "desktop", "gui", "launcher")]
    [string]$Target = "all",
    [switch]$Release = $false,
    [switch]$Clean = $false
)
```

**How it works:**
- `ValidateSet`: Restricts values to specific options
- `[switch]`: Boolean flags (true if present, false if not)
- Default values: `$Target` defaults to "all"

**Usage examples:**
```powershell
.\Build-Launchers.ps1                              # Uses defaults
.\Build-Launchers.ps1 -Target desktop              # Specific target
.\Build-Launchers.ps1 -Target desktop -Release     # Multiple flags
```

#### 2. Configuration Section (Lines 23-65)

```powershell
$Config = @{
    BuildDir = Join-Path $RootDir "build"
    OutputDir = Join-Path $RootDir "build\output"
    LogFile = Join-Path $RootDir "build\build.log"
    Projects = @()  # Populated below
}
```

**Key Points:**
- Hashtable (PowerShell equivalent of dictionary)
- Paths are absolute (Join-Path ensures correct separators)
- Projects array contains hashtables for each project

**Adding new project:**
```powershell
$Config.Projects += @{
    Name = "My New Project"
    Key = "myproject"
    Path = "path/to/project"
    Type = "cargo"  # or "tauri"
    BinaryName = "my-binary"
    OutputName = "MyBinary.exe"
}
```

#### 3. Helper Functions (Lines 73-150)

##### Write Functions
```powershell
function Write-Header { }     # Display formatted header
function Write-Success { }    # Green checkmark + text
function Write-Error { }      # Red X + text
function Write-Info { }       # Blue bullet + text
function Write-Log { }        # To file with timestamp
```

**Usage:**
```powershell
Write-Header "My Section"     # ╔════════════════════╗
Write-Success "All done"      # ✓ All done
Write-Error "Something broke" # ✗ Something broke
Write-Info "Building..."      # • Building...
Write-Log "Detailed message"  # [2026-06-16 18:21:18] Detailed message
```

**Extending:** To add new output type:
```powershell
function Write-Warning {
    param([string]$Text)
    Write-Host "⚠ $Text" -ForegroundColor Yellow
}
```

##### Build Functions

**Build-CargoProject**
```powershell
function Build-CargoProject {
    param(
        [hashtable]$Project,     # Project definition
        [string]$RootDir,        # Repository root
        [string]$BuildMode,      # debug or release
        [string]$BuildDir        # Build directory
    )
    
    # 1. Validate project exists
    # 2. Run: cargo build [--release]
    # 3. Find binary in target/
    # 4. Copy to output directory
    # 5. Return success/failure
}
```

**Build-TauriProject**
```powershell
function Build-TauriProject {
    # Similar to Build-CargoProject
    # But handles Tauri-specific quirks
    # Different binary paths
    # Additional configuration checks
}
```

**When to use which:**
- `Build-CargoProject`: Pure Rust projects
- `Build-TauriProject`: Projects using Tauri framework

#### 4. Main Function (Lines 250-350)

```powershell
function Main {
    # 1. Display header
    # 2. Initialize directories
    # 3. Test prerequisites
    # 4. Filter projects by target
    # 5. Build each project
    # 6. Aggregate results
    # 7. Display summary
    # 8. Return exit code
}
```

**Control Flow:**
```
Main starts
├─ Write-Header "title"
├─ Test-Prerequisites
│  └─ if failed: exit 1
├─ Filter $Config.Projects
├─ Foreach project:
│  ├─ Call Build function
│  ├─ Track results
│  └─ Log outcome
├─ Calculate stats
├─ Display summary
└─ Return 0 or 1
```

---

## How to Improve the Build System

### Task 1: Add a New Project to Build

**Steps:**
1. Create Cargo.toml in project directory
2. Add project definition to Build-Launchers.ps1
3. Test with Verify-Build-Setup.ps1
4. Update documentation

**Example:**

```powershell
# In Build-Launchers.ps1, $Config.Projects, add:
@{
    Name = "New Desktop App"
    Key = "newapp"
    Path = "Omnisystem\applications\new-app"
    Type = "cargo"
    BinaryName = "new-app"
    OutputName = "NewApp.exe"
    Description = "My new application"
}
```

**Test:**
```powershell
.\Verify-Build-Setup.ps1  # Should show your new project
.\Quick-Build.ps1 -Target newapp  # Should build it
```

---

### Task 2: Add Build Mode Support

**Current modes:** Debug, Release  
**Goal:** Add custom mode (e.g., "minimal")

**Implementation:**

```powershell
# 1. Add to parameter validation
[ValidateSet("debug", "release", "minimal")]
[string]$BuildMode = "debug"

# 2. Add case handler
switch ($BuildMode) {
    "debug" {
        $CargoArgs = @("build")
    }
    "release" {
        $CargoArgs = @("build", "--release")
    }
    "minimal" {
        $CargoArgs = @("build", "--release", "-Z", "build-std=std")
    }
}

# 3. Pass to build functions
Build-CargoProject -CargoArgs $CargoArgs ...
```

---

### Task 3: Implement Parallel Builds

**Current:** Sequential (one project at a time)  
**Goal:** Parallel execution (2-3x faster)

**Implementation with PowerShell Jobs:**

```powershell
# Create job for each project
$jobs = @()
foreach ($Project in $ProjectsToBuild) {
    $job = Start-Job -ScriptBlock {
        param($Project, $RootDir)
        # Build logic here
        Build-CargoProject $Project $RootDir
    } -ArgumentList $Project, $RootDir
    
    $jobs += $job
}

# Wait for all jobs
$results = $jobs | Wait-Job | Receive-Job

# Process results
foreach ($result in $results) {
    if ($result -eq $true) { $SuccessCount++ }
    else { $FailCount++ }
}
```

**Benefits:**
- Desktop + GUI + Launcher in parallel
- ~2-3x speedup
- No output interleaving (separate jobs)

**Trade-offs:**
- More complex code
- Harder to debug
- Resource intensive on weak systems

---

### Task 4: Add Incremental Build Checking

**Goal:** Skip building if nothing changed

**Implementation:**

```powershell
function Get-ProjectHash {
    param([string]$ProjectPath)
    
    # Get hash of all source files
    Get-ChildItem -Path "$ProjectPath/src" -Recurse -File |
        Get-FileHash |
        Measure-Object -Property Hash
}

function Build-IfChanged {
    param([string]$ProjectPath)
    
    $CurrentHash = Get-ProjectHash $ProjectPath
    $LastHash = Get-Content "$ProjectPath/.build-hash" -ErrorAction SilentlyContinue
    
    if ($CurrentHash -eq $LastHash) {
        Write-Host "No changes, skipping build"
        return $true
    }
    
    # Build...
    # On success:
    $CurrentHash | Set-Content "$ProjectPath/.build-hash"
    return $success
}
```

**Benefits:**
- Faster iteration during development
- Skip unchanged projects
- Reduced compilation time

---

### Task 5: Add Automated Testing

**Goal:** Run tests after build

**Implementation:**

```powershell
function Run-Tests {
    param([hashtable]$Project)
    
    # Run cargo test
    Push-Location $Project.Path
    $TestOutput = & cargo test 2>&1
    $TestSuccess = $LASTEXITCODE -eq 0
    Pop-Location
    
    if ($TestSuccess) {
        Write-Success "Tests passed"
    } else {
        Write-Error "Tests failed"
        # Log test output
    }
    
    return $TestSuccess
}

# In Main function:
$BuildSuccess = Build-CargoProject ...
$TestSuccess = if ($BuildSuccess) { Run-Tests $Project } else { $false }
```

---

### Task 6: Add Binary Signing

**Goal:** Sign executables for distribution

**Implementation:**

```powershell
function Sign-Binary {
    param(
        [string]$BinaryPath,
        [string]$CertPath,
        [string]$CertPassword
    )
    
    $signtool = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe"
    
    & $signtool sign /f $CertPath /p $CertPassword /t http://timestamp.server $BinaryPath
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Binary signed: $BinaryPath"
        return $true
    } else {
        Write-Error "Signing failed"
        return $false
    }
}
```

---

### Task 7: Add Release Packaging

**Goal:** Create installer or archive

**Implementation:**

```powershell
function Create-ReleasePackage {
    param(
        [string]$Version,
        [string]$OutputDir
    )
    
    # Create subdirectory
    $PackageDir = "$OutputDir\omnisystem-$Version"
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
    
    # Copy binaries
    Copy-Item "$OutputDir\*.exe" -Destination $PackageDir
    
    # Copy documentation
    Copy-Item "README.md" -Destination $PackageDir
    Copy-Item "LICENSE" -Destination $PackageDir
    
    # Create ZIP archive
    Compress-Archive -Path $PackageDir -DestinationPath "$OutputDir\omnisystem-$Version.zip"
    
    Write-Success "Package created: omnisystem-$Version.zip"
}
```

---

## Debugging & Troubleshooting

### Enable Verbose Output

```powershell
# Edit Build-Launchers.ps1
# Uncomment or add:
$VerbosePreference = "Continue"
$DebugPreference = "Continue"

# Now run:
.\Quick-Build.ps1 -Verbose -Debug
```

### Check Build Log Details

```powershell
# View last 50 lines
Get-Content .\build\build.log -Tail 50

# Search for errors
Select-String "error" .\build\build.log

# Monitor in real-time
Get-Content .\build\build.log -Wait

# Full log analysis
(Get-Content .\build\build.log | Measure-Object -Line).Lines  # Line count
```

### Test Individual Components

```powershell
# Test just the helper functions
. .\Build-Launchers.ps1  # Dot-source the script

Write-Header "Test Header"
Write-Success "Test Success"
Write-Error "Test Error"
Write-Info "Test Info"

# Test prerequisite check
Test-Prerequisites

# Test single project build
$Project = $Config.Projects[0]
Build-CargoProject $Project $RootDir "debug" $Config.BuildDir
```

---

## Code Quality Guidelines

### PowerShell Best Practices

1. **Parameter Validation**
```powershell
# Good
param(
    [ValidateSet("a", "b", "c")]
    [string]$Option = "a"
)

# Bad
param([string]$Option)  # No validation
```

2. **Error Handling**
```powershell
# Good
try {
    Push-Location $Path
    # Operation
} catch {
    Write-Error "Failed: $_"
    return $false
} finally {
    Pop-Location
}

# Bad
Push-Location $Path
# Operation
Pop-Location  # Runs even if error occurred
```

3. **Variable Scope**
```powershell
# Good
$script:GlobalVar = "value"  # Script-level

function Test {
    $localVar = "value"  # Function-level
}

# Bad
$GlobalVar = "value"  # Unclear scope
```

4. **Function Documentation**
```powershell
# Good
function Build-Project {
    <#
    .SYNOPSIS
        Build a single project
    .PARAMETER Project
        Project configuration hashtable
    .RETURNS
        Boolean success/failure
    #>
    param([hashtable]$Project)
}

# Bad
function build($p) {  # No documentation
}
```

---

## Performance Optimization

### Current Performance

```
Desktop: 2.5s (debug), 15s (release)
GUI: 60s+ (first build with deps)
Total: ~4.5s (cached desktop)
```

### Optimization Checklist

- [ ] **Parallel builds** - Use PS jobs (2-3x faster)
- [ ] **Incremental** - Skip unchanged projects
- [ ] **Caching** - Cache downloaded dependencies
- [ ] **Lazy eval** - Only load needed code
- [ ] **Async I/O** - Parallel file operations

### Benchmark Template

```powershell
# Measure build time
$StartTime = Get-Date
.\Quick-Build.ps1 -Target desktop
$EndTime = Get-Date
$Duration = $EndTime - $StartTime
Write-Host "Build time: $($Duration.TotalSeconds) seconds"
```

---

## Testing & Validation

### Test Scenarios

1. **Fresh Build**
```powershell
Remove-Item .\build -Recurse -Force
.\Quick-Build.ps1
# Should create everything from scratch
```

2. **Incremental Build**
```powershell
.\Quick-Build.ps1
.\Quick-Build.ps1  # Should be much faster
```

3. **Target Selection**
```powershell
.\Quick-Build.ps1 -Target desktop    # Only desktop
.\Quick-Build.ps1 -Target gui        # Only GUI
.\Quick-Build.ps1 -Target launcher   # Only launcher
```

4. **Build Modes**
```powershell
.\Quick-Build.ps1                    # Debug
.\Quick-Build.ps1 -Release           # Release
```

### Validation Checklist

- [ ] Script runs without errors
- [ ] Builds complete successfully
- [ ] Binaries are created
- [ ] Binaries can execute
- [ ] Logs are written
- [ ] Output directory has files
- [ ] Documentation is updated

---

## Version Control Workflow

### Committing Changes

```powershell
# 1. Make changes
# 2. Test thoroughly
.\Verify-Build-Setup.ps1
.\Quick-Build.ps1

# 3. Check what changed
git diff Build-Launchers.ps1

# 4. Stage changes
git add Build-Launchers.ps1

# 5. Commit with message
git commit -m "feat: Add parallel build support

- Implement PowerShell jobs
- 2-3x faster for multiple targets
- Still works on weak systems"

# 6. Push
git push origin branch-name
```

### Branches

```
main                         # Production-ready
├── feature/parallel-build   # WIP feature
├── fix/logging-issue        # Bug fix
└── docs/improve-docs        # Documentation
```

---

## Common Pitfalls & Solutions

### Pitfall 1: Missing Directory Creation
```powershell
# ✗ Wrong
Copy-Item $Source -Destination $OutputDir\$File

# ✓ Right
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
Copy-Item $Source -Destination $OutputDir\$File
```

### Pitfall 2: Ignoring Exit Codes
```powershell
# ✗ Wrong
& cargo build
# Continue without checking

# ✓ Right
& cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    return $false
}
```

### Pitfall 3: Hardcoded Paths
```powershell
# ✗ Wrong
$LogPath = "C:\Users\User\Projects\build\build.log"

# ✓ Right
$LogPath = Join-Path $RootDir "build\build.log"
```

---

## Resources

### PowerShell Documentation
- [PowerShell Docs](https://docs.microsoft.com/powershell)
- [Writing PowerShell Functions](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_functions)
- [Error Handling](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_try_catch_finally)

### Rust/Cargo
- [Cargo Book](https://doc.rust-lang.org/cargo)
- [Cargo.toml Reference](https://doc.rust-lang.org/cargo/reference)
- [Tauri Documentation](https://tauri.app/docs)

### Build System Docs
- ARCHITECTURE.md - System design
- OPERATION.md - How to use
- developer_guide.md - This file

---

## Summary

This guide provides everything needed to understand, use, and improve the Omnisystem build system. Key takeaways:

1. **Three-layer architecture**: UI, Orchestration, Projects
2. **PowerShell-based**: Portable, scriptable, maintainable
3. **Highly extensible**: Easy to add projects, modes, features
4. **Well-documented**: Inline comments + external docs
5. **Production-ready**: Tested, validated, in use

**Next Steps:**
- Read ARCHITECTURE.md for deep dive
- Study Build-Launchers.ps1 code
- Try suggested improvements
- Contribute enhancements

---

**Document:** Build System Developer Guide  
**Version:** 1.0  
**Date:** June 16, 2026  
**Status:** Complete
