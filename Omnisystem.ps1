#Requires -Version 5.0
<#
.SYNOPSIS
    Omnisystem GUI Application Launcher

.DESCRIPTION
    Launches the built Omnisystem GUI executable from Omnisystem/launchers/

.EXAMPLE
    .\Omnisystem.ps1

#>

$ErrorActionPreference = "Stop"

# Get paths
$ProjectRoot = Split-Path -Parent $PSCommandPath
$LaunchersDir = Join-Path $ProjectRoot "Omnisystem" "launchers"
$GuiExe = Join-Path $LaunchersDir "Omnisystem.exe"

Write-Host ""

# Check if executable exists
if (-not (Test-Path $GuiExe)) {
    Write-Host "ERROR: Omnisystem.exe not found" -ForegroundColor Red
    Write-Host "Location: $GuiExe" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please build the executable first:" -ForegroundColor Yellow
    Write-Host "  .\Build-All.ps1    (build both GUI and TUI)" -ForegroundColor Cyan
    Write-Host "  .\Build-GUI.ps1    (build GUI only)" -ForegroundColor Cyan
    Write-Host ""
    exit 1
}

# Launch the GUI
Write-Host "Launching Omnisystem GUI..." -ForegroundColor Green
Write-Host ""

& $GuiExe
