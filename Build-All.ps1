#Requires -Version 5.0
<#
.SYNOPSIS
    Omnisystem Build Wrapper - Calls master build script in scripts/build/

.DESCRIPTION
    This is a convenience wrapper in the project root that calls the actual
    build scripts located in Omnisystem/scripts/build/

    All executables are built to: Omnisystem/launchers/

.EXAMPLE
    .\Build-All.ps1              (build both GUI and TUI)
    .\Build-All.ps1 -GUI         (build GUI only)
    .\Build-All.ps1 -TUI         (build TUI only)
    .\Build-All.ps1 -Clean       (clean build artifacts)

#>

param(
    [switch]$GUI = $true,
    [switch]$TUI = $true,
    [switch]$Launch = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"

# Get the project root (where this script is)
$ProjectRoot = Split-Path -Parent $PSCommandPath
$MasterBuildScript = Join-Path $ProjectRoot "Omnisystem" "scripts" "build" "Build-All.ps1"

if (-not (Test-Path $MasterBuildScript)) {
    Write-Host "ERROR: Master build script not found at: $MasterBuildScript" -ForegroundColor Red
    exit 1
}

# Call the master build script with all parameters
$params = @{}
if ($GUI) { $params["GUI"] = $true }
if ($TUI) { $params["TUI"] = $true }
if ($Launch) { $params["Launch"] = $true }
if ($Clean) { $params["Clean"] = $true }

& $MasterBuildScript @params
