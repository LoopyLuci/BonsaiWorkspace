#!/usr/bin/env pwsh
# Omnisystem v2.5.0 - Multi-Language Build System
# TITAN | SYLVA | AETHER | AXIOM

param([switch]$Clean)

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptRoot

Write-Host "======================================================================"
Write-Host "  OMNISYSTEM v2.5.0 - PRODUCTION BUILD SYSTEM"
Write-Host "  Multi-Language Enterprise OS (TITAN | SYLVA | AETHER | AXIOM)"
Write-Host "======================================================================"
Write-Host ""

if ($Clean) {
    Write-Host "Cleaning old builds..."
    Remove-Item "$scriptRoot\Omnisystem\omnisystem-cli\target" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item "$scriptRoot\Omnisystem\omnisystem-gui\target" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Clean complete."
}

Write-Host "Building Omnisystem.exe (GUI Launcher)..."
Set-Location "$scriptRoot\Omnisystem\omnisystem-gui"
cargo build --release 2>&1 | Where-Object { $_ -match "Finished|error" }

if (Test-Path "target\release\Omnisystem.exe") {
    Copy-Item "target\release\Omnisystem.exe" "$scriptRoot\Omnisystem.exe" -Force
    Write-Host "SUCCESS: Omnisystem.exe built and deployed"
} else {
    Write-Host "ERROR: GUI build failed"
    exit 1
}

Write-Host ""
Write-Host "Building Omnisystem_CLI.exe (CLI Interface)..."
Set-Location "$scriptRoot\Omnisystem\omnisystem-cli"
cargo build --release 2>&1 | Where-Object { $_ -match "Finished|error" }

if (Test-Path "target\release\omnisystem.exe") {
    Copy-Item "target\release\omnisystem.exe" "$scriptRoot\Omnisystem_CLI.exe" -Force
    Write-Host "SUCCESS: Omnisystem_CLI.exe built and deployed"
} else {
    Write-Host "ERROR: CLI build failed"
    exit 1
}

Set-Location $scriptRoot

Write-Host ""
Write-Host "Final components:"
Get-Item "Omnisystem.exe", "Omnisystem_CLI.exe", "BUILD.ps1" -ErrorAction SilentlyContinue | ForEach-Object {
    $size = ($_.Length / 1KB).ToString('F1')
    Write-Host "  - $($_.Name) ($size KB)"
}

Write-Host ""
Write-Host "======================================================================"
Write-Host "BUILD COMPLETE - PRODUCTION READY"
Write-Host "======================================================================"
Write-Host ""
Write-Host "Launch:"
Write-Host "  - Double-click Omnisystem.exe (GUI)"
Write-Host "  - Run: Omnisystem_CLI.exe gui (CLI)"
Write-Host ""
Write-Host "Architecture: TITAN (GUI) + SYLVA (Analytics) + AETHER (Distributed) + AXIOM (Verification)"
Write-Host ""
