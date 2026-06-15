#Requires -Version 5.0
<#
.SYNOPSIS
    Omnisystem Build Script - Creates Omnisystem.exe in root directory

.DESCRIPTION
    Builds the Omnisystem Tauri desktop application and creates a standalone
    executable in the project root directory.

.EXAMPLE
    .\Build-Omnisystem.ps1

#>

param(
    [switch]$Release = $false,
    [switch]$Clean = $false,
    [switch]$Launch = $false
)

$ErrorActionPreference = "Stop"

# Setup paths
$ProjectRoot = Split-Path -Parent $PSCommandPath
$GuiDir = Join-Path $ProjectRoot "omnisystem-gui"
$ExePath = Join-Path $ProjectRoot "Omnisystem.exe"

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         OMNISYSTEM BUILD SCRIPT - CREATE EXECUTABLE       ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

Write-Host "Project Root: $ProjectRoot" -ForegroundColor Green
Write-Host "GUI Directory: $GuiDir" -ForegroundColor Green
Write-Host "Output: $ExePath" -ForegroundColor Green
Write-Host ""

# Validate paths
if (-not (Test-Path $GuiDir)) {
    Write-Host "ERROR: GUI directory not found" -ForegroundColor Red
    exit 1
}

# Change to GUI directory
Push-Location $GuiDir

# Clean if requested
if ($Clean) {
    Write-Host ""
    Write-Host "CLEANING BUILD ARTIFACTS" -ForegroundColor Cyan
    Write-Host ""

    if (Test-Path "node_modules") {
        Write-Host "Removing node_modules..." -ForegroundColor Yellow
        Remove-Item -Recurse -Force "node_modules" -ErrorAction SilentlyContinue
    }

    if (Test-Path "dist") {
        Remove-Item -Recurse -Force "dist" -ErrorAction SilentlyContinue
    }

    if (Test-Path "src-tauri/target") {
        Remove-Item -Recurse -Force "src-tauri/target" -ErrorAction SilentlyContinue
    }

    Write-Host "Cleanup complete" -ForegroundColor Green
    Write-Host ""
}

# Install dependencies
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "INSTALLING DEPENDENCIES" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

npm install 2>&1 | Out-Null
Write-Host "Dependencies installed" -ForegroundColor Green
Write-Host ""

# Build the application
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "BUILDING OMNISYSTEM" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

if ($Release) {
    Write-Host "Building RELEASE mode (optimized and smaller)..." -ForegroundColor Yellow
    npm run tauri:build 2>&1 | Out-Null
} else {
    Write-Host "Building DEV mode..." -ForegroundColor Yellow
    npm run tauri:dev 2>&1 | Out-Null
}

Write-Host "Build completed" -ForegroundColor Green
Write-Host ""

# Find the executable
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "LOCATING EXECUTABLE" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$PossiblePaths = @(
    (Join-Path $ProjectRoot "omnisystem-gui/src-tauri/target/release/omnisystem-gui.exe"),
    (Join-Path $ProjectRoot "omnisystem-gui/src-tauri/target/debug/omnisystem-gui.exe"),
    (Join-Path $ProjectRoot "omnisystem-gui/dist/omnisystem-gui.exe")
)

$FoundExe = $null
foreach ($Path in $PossiblePaths) {
    if (Test-Path $Path) {
        $FoundExe = $Path
        Write-Host "Found: $Path" -ForegroundColor Green
        break
    }
}

if (-not $FoundExe) {
    Write-Host "Searching for executable..." -ForegroundColor Yellow
    $ExeFiles = Get-ChildItem -Recurse -Filter "omnisystem*.exe" -ErrorAction SilentlyContinue
    if ($ExeFiles) {
        $FoundExe = $ExeFiles[0].FullName
        Write-Host "Found: $FoundExe" -ForegroundColor Green
    } else {
        Write-Host "ERROR: No executable found" -ForegroundColor Red
        Pop-Location
        exit 1
    }
}

Write-Host ""

# Copy to root
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "CREATING ROOT EXECUTABLE" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

Copy-Item -Path $FoundExe -Destination $ExePath -Force

# Verify
if (Test-Path $ExePath) {
    $FileSize = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
    Write-Host "SUCCESS: Omnisystem.exe created ($FileSize MB)" -ForegroundColor Green
} else {
    Write-Host "ERROR: Copy failed" -ForegroundColor Red
    Pop-Location
    exit 1
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "BUILD COMPLETE" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "LAUNCH OMNISYSTEM:" -ForegroundColor Green
Write-Host "  .\Omnisystem.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "Location: $ExePath" -ForegroundColor Cyan
Write-Host ""

# Launch if requested
if ($Launch) {
    Write-Host "Launching Omnisystem..." -ForegroundColor Yellow
    Start-Process $ExePath
    Write-Host "Application launched" -ForegroundColor Green
}

Pop-Location
Write-Host "Build complete!" -ForegroundColor Green
Write-Host ""
exit 0
