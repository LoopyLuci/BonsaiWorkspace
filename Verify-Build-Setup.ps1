# ============================================================================
# BUILD SETUP VERIFICATION SCRIPT
# ============================================================================
# Verifies that all build prerequisites and project files are in place

$ErrorActionPreference = "SilentlyContinue"
$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║       OMNISYSTEM BUILD SETUP VERIFICATION                      ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# ============================================================================
# 1. Check Rust Installation
# ============================================================================

Write-Host "1. Checking Rust Installation..." -ForegroundColor Yellow
$RustVersion = & cargo --version 2>&1
if ($?) {
    Write-Host "   ✓ Cargo found: $RustVersion" -ForegroundColor Green
    $RustcVersion = & rustc --version 2>&1
    Write-Host "   ✓ Rustc: $RustcVersion" -ForegroundColor Green
} else {
    Write-Host "   ✗ Rust/Cargo not found" -ForegroundColor Red
    Write-Host "   Install from https://rustup.rs/`n" -ForegroundColor Red
    exit 1
}

# ============================================================================
# 2. Check Projects
# ============================================================================

Write-Host "`n2. Checking Project Structures..." -ForegroundColor Yellow

$Projects = @(
    @{
        Name = "Desktop Environment"
        Path = "Omnisystem\applications\bonsai-desktop-environment"
        Files = @("Cargo.toml", "src\launcher\main.rs")
    },
    @{
        Name = "GUI Launcher (Tauri)"
        Path = "Omnisystem\src\crates\omnisystem-launcher-gui\src-tauri"
        Files = @("Cargo.toml", "src\main.rs")
    },
    @{
        Name = "BonsaiEcosystem Launcher"
        Path = "Omnisystem\modules\base-modules\applications\bonsai-ecosystem\launcher"
        Files = @("Cargo.toml", "src-tauri\src\main.rs")
    }
)

$AllOk = $true

foreach ($Project in $Projects) {
    $ProjectPath = Join-Path $RootDir $Project.Path

    if (Test-Path $ProjectPath) {
        Write-Host "   ✓ $($Project.Name)" -ForegroundColor Green
        $MissingFiles = @()

        foreach ($File in $Project.Files) {
            $FilePath = Join-Path $ProjectPath $File
            if (-not (Test-Path $FilePath)) {
                $MissingFiles += $File
            }
        }

        if ($MissingFiles.Count -gt 0) {
            Write-Host "     ✗ Missing files: $($MissingFiles -join ', ')" -ForegroundColor Red
            $AllOk = $false
        } else {
            Write-Host "     ✓ All required files present" -ForegroundColor Green
        }
    } else {
        Write-Host "   ✗ $($Project.Name) - Path not found" -ForegroundColor Red
        Write-Host "     Expected: $ProjectPath" -ForegroundColor Red
        $AllOk = $false
    }
}

# ============================================================================
# 3. Check Build Scripts
# ============================================================================

Write-Host "`n3. Checking Build Scripts..." -ForegroundColor Yellow

$Scripts = @(
    "Build-Omnisystem.ps1",
    "Build-Launchers.ps1",
    "Quick-Build.ps1",
    "Verify-Build-Setup.ps1"
)

foreach ($Script in $Scripts) {
    $ScriptPath = Join-Path $RootDir $Script
    if (Test-Path $ScriptPath) {
        Write-Host "   ✓ $Script" -ForegroundColor Green
    } else {
        Write-Host "   ✗ $Script not found" -ForegroundColor Red
        $AllOk = $false
    }
}

# ============================================================================
# 4. Check Cargo Configuration
# ============================================================================

Write-Host "`n4. Verifying Cargo Configuration..." -ForegroundColor Yellow

foreach ($Project in $Projects) {
    $CargoPath = Join-Path $RootDir $Project.Path "Cargo.toml"

    if (Test-Path $CargoPath) {
        $Content = Get-Content $CargoPath | Select-Object -First 5 | Select-String "^\[package\]"
        if ($Content) {
            Write-Host "   ✓ $($Project.Name) - Valid Cargo.toml" -ForegroundColor Green
        }
    }
}

# ============================================================================
# 5. Summary
# ============================================================================

Write-Host "`n" -ForegroundColor Cyan

if ($AllOk) {
    Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║               ✓ ALL CHECKS PASSED                              ║" -ForegroundColor Green
    Write-Host "║       Build environment is ready to compile                    ║" -ForegroundColor Green
    Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Green

    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "  1. Run: .\Quick-Build.ps1" -ForegroundColor Cyan
    Write-Host "     or: .\Build-Launchers.ps1 -Target all" -ForegroundColor Cyan
    Write-Host "  2. For release build: .\Quick-Build.ps1 -Release" -ForegroundColor Cyan
    Write-Host "  3. Find outputs in: .\build\output\" -ForegroundColor Cyan

    Write-Host "`nBuild Options:" -ForegroundColor Cyan
    Write-Host "  -Target desktop     Build only desktop environment" -ForegroundColor Gray
    Write-Host "  -Target gui         Build only GUI launcher" -ForegroundColor Gray
    Write-Host "  -Target launcher    Build only BonsaiEcosystem launcher" -ForegroundColor Gray
    Write-Host "  -Release            Create optimized release build" -ForegroundColor Gray

    Write-Host ""
    exit 0
} else {
    Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║               ✗ SOME CHECKS FAILED                             ║" -ForegroundColor Red
    Write-Host "║       Please fix the issues above before building              ║" -ForegroundColor Red
    Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Red
    Write-Host ""
    exit 1
}
