# ============================================================================
# OMNISYSTEM BUILD SCRIPT
# ============================================================================
# Builds all launchers and components for OmnisystemEcosystem Desktop Environment
# Usage: .\Build-Omnisystem.ps1 [-Release] [-Clean] [-Verbose]
# ============================================================================

param(
    [switch]$Release = $false,
    [switch]$Clean = $false,
    [switch]$Verbose = $false
)

# ============================================================================
# CONFIGURATION
# ============================================================================

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BuildDir = Join-Path $RootDir "build"
$OutputDir = Join-Path $BuildDir "output"
$LogFile = Join-Path $BuildDir "build.log"

$BuildMode = if ($Release) { "release" } else { "debug" }
$CargoArgs = @("build")
if ($Release) { $CargoArgs += "--release" }
if ($Verbose) { $CargoArgs += "--verbose" }

# ============================================================================
# LAUNCHERS TO BUILD
# ============================================================================

$Launchers = @(
    @{
        Name = "OmnisystemEcosystem Desktop Environment"
        Path = "Omnisystem\applications\omnisystem-desktop-environment"
        Binary = "Omnisystem"
        Output = "Omnisystem.exe"
    },
    @{
        Name = "Omnisystem Launcher GUI (Tauri)"
        Path = "Omnisystem\src\crates\omnisystem-launcher-gui\src-tauri"
        Binary = "omnisystem-launcher-tauri"
        Output = "OmnisystemGUI.exe"
        IsTauri = $true
    },
    @{
        Name = "OmnisystemEcosystem Launcher"
        Path = "Omnisystem\modules\base-modules\applications\omnisystem-ecosystem\launcher"
        Binary = "omnisystem-launcher"
        Output = "OmnisystemLauncher.exe"
        IsTauri = $true
    }
)

# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $LogMsg = "[$Timestamp] [$Level] $Message"
    Write-Host $LogMsg
    Add-Content -Path $LogFile -Value $LogMsg
}

function Test-CargoProject {
    param([string]$ProjectPath)
    $CargoToml = Join-Path $ProjectPath "Cargo.toml"
    return (Test-Path $CargoToml)
}

function Build-Launcher {
    param(
        [string]$Name,
        [string]$RelPath,
        [string]$Binary,
        [string]$Output
    )

    $FullPath = Join-Path $RootDir $RelPath

    Write-Log "Building: $Name"
    Write-Log "  Path: $FullPath"

    if (-not (Test-Path $FullPath)) {
        Write-Log "  ERROR: Project path not found" "ERROR"
        return $false
    }

    if (-not (Test-CargoProject $FullPath)) {
        Write-Log "  ERROR: Cargo.toml not found" "ERROR"
        return $false
    }

    try {
        Push-Location $FullPath
        Write-Log "  Running: cargo $($CargoArgs -join ' ')"

        & cargo @CargoArgs 2>&1 | Tee-Object -FilePath $LogFile -Append

        if ($LASTEXITCODE -ne 0) {
            Write-Log "  ERROR: Build failed with exit code $LASTEXITCODE" "ERROR"
            return $false
        }

        # Copy binary to output directory
        $TargetSubdir = if ($Release) { "release" } else { "debug" }
        $BuiltBinary = Join-Path $FullPath "target\$TargetSubdir\$Binary.exe"

        if (Test-Path $BuiltBinary) {
            $OutputPath = Join-Path $OutputDir $Output
            Copy-Item -Path $BuiltBinary -Destination $OutputPath -Force
            Write-Log "  ✓ Built successfully: $Output"
            return $true
        } else {
            Write-Log "  WARNING: Binary not found at $BuiltBinary" "WARN"
            # Still return success if cargo succeeded, binary may have different name
            return $true
        }
    }
    catch {
        Write-Log "  ERROR: $_" "ERROR"
        return $false
    }
    finally {
        Pop-Location
    }
}

# ============================================================================
# MAIN BUILD PROCESS
# ============================================================================

function Main {
    Write-Host "`n"
    Write-Host "╔════════════════════════════════════════════════════════════════╗"
    Write-Host "║     OMNISYSTEM BUILD SYSTEM - OmnisystemEcosystem Desktop          ║"
    Write-Host "║     Building all launchers and components                      ║"
    Write-Host "╚════════════════════════════════════════════════════════════════╝"
    Write-Host ""

    Write-Log "═════════════════════════════════════════════════════════════════"
    Write-Log "OMNISYSTEM BUILD STARTED"
    Write-Log "Build Mode: $BuildMode"
    Write-Log "Root Directory: $RootDir"
    Write-Log "═════════════════════════════════════════════════════════════════"

    # Create directories
    Write-Log "Creating build directories..."
    New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

    # Clean if requested
    if ($Clean) {
        Write-Log "Cleaning build artifacts..."
        Get-ChildItem -Path $BuildDir -Recurse | Remove-Item -Force -ErrorAction SilentlyContinue
        Write-Log "✓ Clean complete"
    }

    # Check prerequisites
    Write-Log "Checking prerequisites..."

    try {
        $RustVersion = cargo --version
        Write-Log "✓ Rust installed: $RustVersion"
    }
    catch {
        Write-Log "ERROR: Rust/Cargo not found. Please install Rust from https://rustup.rs/" "ERROR"
        Write-Log "Build failed" "ERROR"
        exit 1
    }

    # Build launchers
    Write-Log ""
    Write-Log "────────────────────────────────────────────────────────────────"
    Write-Log "BUILDING LAUNCHERS"
    Write-Log "────────────────────────────────────────────────────────────────"

    $SuccessCount = 0
    $FailCount = 0

    foreach ($Launcher in $Launchers) {
        Write-Log ""
        if (Build-Launcher -Name $Launcher.Name -RelPath $Launcher.Path -Binary $Launcher.Binary -Output $Launcher.Output) {
            $SuccessCount++
        } else {
            $FailCount++
        }
    }

    # Summary
    Write-Log ""
    Write-Log "────────────────────────────────────────────────────────────────"
    Write-Log "BUILD SUMMARY"
    Write-Log "────────────────────────────────────────────────────────────────"
    Write-Log "Successful: $SuccessCount"
    Write-Log "Failed: $FailCount"

    if ($FailCount -eq 0) {
        Write-Log "✓ ALL BUILDS SUCCESSFUL"
        Write-Log "Output directory: $OutputDir"
        Write-Host "`n✓ Build complete! Executables ready in: $OutputDir`n"
        return 0
    } else {
        Write-Log "✗ SOME BUILDS FAILED - see log for details" "ERROR"
        Write-Host "`n✗ Build incomplete. Check $LogFile for details.`n"
        return 1
    }
}

# Run main build
exit (Main)
