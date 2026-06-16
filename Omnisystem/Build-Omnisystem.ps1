#Requires -Version 5.0
<#
.SYNOPSIS
    Omnisystem Build Script - Creates Omnisystem.exe

.DESCRIPTION
    Builds the Omnisystem application using Rust Cargo and creates a standalone
    executable in the Omnisystem directory.

.EXAMPLE
    .\Build-Omnisystem.ps1
    .\Build-Omnisystem.ps1 -Debug
    .\Build-Omnisystem.ps1 -Clean -Release

#>

param(
    [switch]$Debug,
    [switch]$Clean,
    [switch]$Launch
)

$ErrorActionPreference = "Stop"

# Setup paths
$ProjectRoot = Split-Path -Parent $PSCommandPath
$GuiDir = Join-Path $ProjectRoot "omnisystem-gui"
$ExePath = Join-Path $ProjectRoot "Omnisystem.exe"

Write-Host ""
Write-Host "OMNISYSTEM BUILD SCRIPT" -ForegroundColor Cyan
Write-Host ""

Write-Host "Project Root: $ProjectRoot" -ForegroundColor Green
Write-Host "GUI Directory: $GuiDir" -ForegroundColor Green
Write-Host "Output: $ExePath" -ForegroundColor Green

$BuildMode = if ($Debug) { "DEBUG (with symbols)" } else { "RELEASE (optimized)" }
Write-Host "Build Mode: $BuildMode" -ForegroundColor Green
Write-Host ""

# Validate paths
if (-not (Test-Path $GuiDir)) {
    Write-Host "ERROR: GUI directory not found at $GuiDir" -ForegroundColor Red
    exit 1
}

# Check for Cargo
$CargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $CargoCmd) {
    Write-Host "ERROR: Cargo not found. Install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host "Cargo found: $($CargoCmd.Source)" -ForegroundColor Green
Write-Host ""

# Change to GUI directory
Push-Location $GuiDir

try {
    # Clean if requested
    if ($Clean) {
        Write-Host ""
        Write-Host "CLEANING BUILD ARTIFACTS" -ForegroundColor Cyan
        Write-Host ""

        if (Test-Path "target") {
            Write-Host "Removing target directory..." -ForegroundColor Yellow
            Remove-Item -Recurse -Force "target" -ErrorAction SilentlyContinue
            Write-Host "Cleaned" -ForegroundColor Green
        }

        Write-Host ""
    }

    # Build the application
    Write-Host "BUILDING OMNISYSTEM" -ForegroundColor Cyan
    Write-Host ""

    if ($Debug) {
        Write-Host "Building DEBUG mode..." -ForegroundColor Yellow
        & cargo build
        $BuiltExePath = "target/debug/Omnisystem.exe"
    } else {
        Write-Host "Building RELEASE mode..." -ForegroundColor Yellow
        & cargo build --release
        $BuiltExePath = "target/release/Omnisystem.exe"
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Build failed" -ForegroundColor Red
        Pop-Location
        exit 1
    }

    Write-Host ""
    Write-Host "Build completed successfully" -ForegroundColor Green
    Write-Host ""

    # Locate the executable
    Write-Host "LOCATING EXECUTABLE" -ForegroundColor Cyan
    Write-Host ""

    if (Test-Path $BuiltExePath) {
        $FoundExe = (Resolve-Path $BuiltExePath).Path
        Write-Host "Found: $FoundExe" -ForegroundColor Green
    } else {
        Write-Host "ERROR: Executable not found at: $BuiltExePath" -ForegroundColor Red
        Pop-Location
        exit 1
    }

    Write-Host ""

    # Copy to root
    Write-Host "CREATING EXECUTABLE" -ForegroundColor Cyan
    Write-Host ""

    Copy-Item -Path $FoundExe -Destination $ExePath -Force
    Write-Host "Copied to: $ExePath" -ForegroundColor Green

    # Verify
    if (Test-Path $ExePath) {
        $FileSize = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
        Write-Host ""
        Write-Host "SUCCESS: Omnisystem.exe created ($FileSize MB)" -ForegroundColor Green
        Write-Host ""

        # Launch if requested
        if ($Launch) {
            Write-Host "LAUNCHING OMNISYSTEM" -ForegroundColor Cyan
            Write-Host ""
            Write-Host "Launching: $ExePath" -ForegroundColor Yellow
            & $ExePath
        }
    } else {
        Write-Host "ERROR: Copy failed" -ForegroundColor Red
        Pop-Location
        exit 1
    }

} finally {
    Pop-Location
}
