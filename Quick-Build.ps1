# ============================================================================
# QUICK BUILD - Simple one-command build of all Omnisystem launchers
# ============================================================================
# Usage: .\Quick-Build.ps1
#        .\Quick-Build.ps1 -Release
#        .\Quick-Build.ps1 -Desktop (builds only desktop environment)

param(
    [switch]$Release = $false,
    [ValidateSet("all", "desktop", "gui", "launcher")]
    [string]$Target = "all"
)

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BuildScript = Join-Path $RootDir "Build-Launchers.ps1"

if (-not (Test-Path $BuildScript)) {
    Write-Host "✗ Build-Launchers.ps1 not found" -ForegroundColor Red
    exit 1
}

Write-Host "Building BonsaiEcosystem Omnisystem..." -ForegroundColor Cyan

$BuildArgs = @{
    Target = $Target
}

if ($Release) {
    $BuildArgs["Release"] = $true
}

& $BuildScript @BuildArgs
