#Requires -Version 5.0
<#
.SYNOPSIS
    Build Omnisystem GUI Launcher

.EXAMPLE
    .\Build-GUI.ps1
    .\Build-GUI.ps1 -Launch

#>

param([switch]$Launch)

$ProjectRoot = Split-Path -Parent $PSCommandPath
$BuildScript = Join-Path $ProjectRoot "Omnisystem" "scripts" "build" "Build-Omnisystem-GUI.ps1"

if (-not (Test-Path $BuildScript)) {
    Write-Host "ERROR: Build script not found" -ForegroundColor Red
    exit 1
}

if ($Launch) {
    & $BuildScript -Launch
} else {
    & $BuildScript
}
