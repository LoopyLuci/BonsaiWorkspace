#!/usr/bin/env pwsh
# Omnisystem v2.5.0 Build Script

param([switch]$Clean)

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptRoot

Write-Host "======================================================================"
Write-Host "  OMNISYSTEM v2.5.0 - BUILD SYSTEM"
Write-Host "  Enterprise Operating System | Production Ready"
Write-Host "======================================================================"
Write-Host ""

if ($Clean) {
    Write-Host "Cleaning old builds..."
    Remove-Item "$scriptRoot\Omnisystem\omnisystem-cli\target" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Done."
}

Write-Host "Building Omnisystem_CLI.exe..."
Set-Location "$scriptRoot\Omnisystem\omnisystem-cli"
cargo build --release 2>&1 | Where-Object { $_ -match "Finished|error" }

if (Test-Path "target\release\omnisystem.exe") {
    Copy-Item "target\release\omnisystem.exe" "$scriptRoot\Omnisystem_CLI.exe" -Force
    Write-Host "SUCCESS: Omnisystem_CLI.exe built"
} else {
    Write-Host "ERROR: Build failed"
    exit 1
}

Set-Location $scriptRoot

Write-Host ""
Write-Host "Verifying components..."
$components = @(
    "Omnisystem_CLI.exe",
    "Omnisystem.exe.bat",
    "Omnisystem.bat",
    "AppMenu.html"
)

foreach ($comp in $components) {
    if (Test-Path $comp) {
        $size = ((Get-Item $comp).Length / 1KB).ToString('F1')
        Write-Host "  - $comp ($size KB)"
    }
}

Write-Host ""
Write-Host "======================================================================"
Write-Host "BUILD COMPLETE"
Write-Host "======================================================================"
Write-Host ""
Write-Host "Launch methods:"
Write-Host "  1. Double-click Omnisystem.exe.bat"
Write-Host "  2. Double-click AppMenu.html"
Write-Host "  3. Run: Omnisystem_CLI.exe gui"
Write-Host ""
Write-Host "Status: PRODUCTION READY"
Write-Host ""
